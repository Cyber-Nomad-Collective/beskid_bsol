//! Offline, digest-bound schema bytes. Parsing belongs to the consumer.
use bsol_analysis::MaterializedSchema;
use cap_std::{ambient_authority, fs::Dir};
use std::{collections::HashMap, io::Read, path::Path};

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Deserialize)]
#[serde(tag = "source", rename_all = "snake_case", deny_unknown_fields)]
pub enum SchemaImportRequest {
    File {
        path: String,
    },
    Git {
        url: String,
        revision: String,
        path: String,
    },
    Registry {
        package: String,
        version: String,
        path: String,
    },
}

impl SchemaImportRequest {
    pub fn pckg(reference: &str) -> Result<Self, MaterializationError> {
        let (package, version, path) = bsol_analysis::parse_pckg_shorthand(reference)
            .map_err(|error| MaterializationError::InvalidRequest(error.to_string()))?;
        let request = Self::Registry {
            package,
            version,
            path,
        };
        request.validate()?;
        Ok(request)
    }

    fn validate(&self) -> Result<(), MaterializationError> {
        let path = match self {
            Self::File { path } => path,
            Self::Git {
                url,
                revision,
                path,
            } => {
                let parsed = url::Url::parse(url).map_err(|_| invalid("invalid Git URL"))?;
                if !matches!(parsed.scheme(), "https" | "ssh")
                    || parsed.host_str().is_none()
                    || (parsed.scheme() == "https" && !parsed.username().is_empty())
                    || parsed.password().is_some()
                    || parsed.query().is_some()
                    || parsed.fragment().is_some()
                    || parsed.as_str() != url
                    || parsed.path() == "/"
                {
                    return Err(invalid(
                        "Git URL must be canonical and contain no credentials/query/fragment",
                    ));
                }
                if !matches!(revision.len(), 40 | 64)
                    || !revision
                        .bytes()
                        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
                {
                    return Err(invalid(
                        "Git revision must be a full lowercase immutable object identity",
                    ));
                }
                path
            }
            Self::Registry {
                package,
                version,
                path,
            } => {
                if package.is_empty()
                    || package.len() > 256
                    || package
                        .chars()
                        .any(|character| character.is_control() || character.is_whitespace())
                {
                    return Err(invalid("invalid package identity"));
                }
                let parsed = semver::Version::parse(version)
                    .map_err(|_| invalid("registry version must be exact SemVer"))?;
                if parsed.to_string() != *version {
                    return Err(invalid("registry version must be canonical"));
                }
                path
            }
        };
        relative_path(path)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MaterializationError {
    #[error("invalid schema import: {0}")]
    InvalidRequest(String),
    #[error("schema import has no exact materialization lock")]
    MissingLock,
    #[error("schema lock contains duplicate request or conflicting canonical identity")]
    ConflictingLock,
    #[error("schema source exceeds configured byte limit")]
    Limit,
    #[error("schema source is not UTF-8")]
    Utf8,
    #[error("schema source does not match its locked SHA-256")]
    DigestMismatch,
    #[error("controlled schema materialization I/O failed: {0}")]
    Io(#[from] std::io::Error),
}

fn invalid(message: &str) -> MaterializationError {
    MaterializationError::InvalidRequest(message.into())
}

impl MaterializationError {
    /// Stable host failure code carried into the import graph failure.
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidRequest(_) => "InvalidImportRequest",
            Self::MissingLock => "MissingImportLock",
            Self::ConflictingLock => "ConflictingImportLock",
            Self::Limit => "LimitExceeded",
            Self::Utf8 => "InvalidImportUtf8",
            Self::DigestMismatch => "ImportDigestMismatch",
            Self::Io(_) => "MaterializationIo",
        }
    }
}

impl From<MaterializationError> for bsol_analysis::ImportFailure {
    fn from(error: MaterializationError) -> Self {
        Self::new(error.code(), error.to_string())
    }
}

impl bsol_analysis::SchemaSource for LockedSchemaHost {
    fn materialize(
        &self,
        spec: &bsol_schema::ImportSchemaSpec,
        _base_dir: &Path,
    ) -> Result<MaterializedSchema, bsol_analysis::ImportFailure> {
        use bsol_schema::ImportSource;
        let request = match &spec.from {
            ImportSource::File { path } => SchemaImportRequest::File { path: path.clone() },
            ImportSource::Git { url, rev, path } => SchemaImportRequest::Git {
                url: url.clone(),
                revision: rev.clone(),
                path: path.clone(),
            },
            ImportSource::Registry {
                package,
                version,
                path,
            } => SchemaImportRequest::Registry {
                package: package.clone(),
                version: version.clone(),
                path: path.clone(),
            },
            ImportSource::PckgShorthand { reference } => SchemaImportRequest::pckg(reference)?,
        };
        Ok(LockedSchemaHost::resolve(self, &request)?)
    }
}

fn relative_path(path: &str) -> Result<(), MaterializationError> {
    if path.is_empty()
        || path
            .bytes()
            .any(|byte| byte < 32 || byte == 127 || matches!(byte, b'\\' | b':'))
        || path.split('/').any(|part| matches!(part, "" | "." | ".."))
    {
        return Err(invalid(
            "path must be portable and relative to the controlled root",
        ));
    }
    Ok(())
}

/// Configuration supplied by the already verified project materializer.
/// A directory listing or cache filename is never sufficient lock authority.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LockedImport {
    request: SchemaImportRequest,
    canonical_identity: String,
    materialized_path: String,
    source_sha256: [u8; 32],
}

impl LockedImport {
    pub fn new(
        request: SchemaImportRequest,
        canonical_identity: String,
        materialized_path: String,
        source_sha256: [u8; 32],
    ) -> Self {
        Self {
            request,
            canonical_identity,
            materialized_path,
            source_sha256,
        }
    }
}

/// One opened root capability and an immutable exact-request lock inventory.
/// It performs no network access, subprocesses, parser calls or implicit fetch.
pub struct LockedSchemaHost {
    root: Dir,
    locks: HashMap<SchemaImportRequest, LockedImport>,
    max_bytes: u64,
}

impl LockedSchemaHost {
    pub fn open(
        root: &Path,
        locks: Vec<LockedImport>,
        max_bytes: u64,
    ) -> Result<Self, MaterializationError> {
        if max_bytes == 0 || max_bytes >= usize::MAX as u64 {
            return Err(MaterializationError::Limit);
        }
        let mut inventory = HashMap::new();
        let mut identities = HashMap::new();
        for lock in locks {
            lock.request.validate()?;
            relative_path(&lock.materialized_path)?;
            if lock.canonical_identity.is_empty()
                || lock.canonical_identity.len() > 4096
                || lock.canonical_identity.chars().any(char::is_control)
            {
                return Err(invalid("missing or invalid canonical identity"));
            }
            if let Some(digest) =
                identities.insert(lock.canonical_identity.clone(), lock.source_sha256)
            {
                if digest != lock.source_sha256 {
                    return Err(MaterializationError::ConflictingLock);
                }
            }
            if inventory.insert(lock.request.clone(), lock).is_some() {
                return Err(MaterializationError::ConflictingLock);
            }
        }
        Ok(Self {
            root: Dir::open_ambient_dir(root, ambient_authority())?,
            locks: inventory,
            max_bytes,
        })
    }

    pub fn source_digest(bytes: &[u8]) -> [u8; 32] {
        MaterializedSchema::digest(bytes)
    }

    pub fn resolve(
        &self,
        request: &SchemaImportRequest,
    ) -> Result<MaterializedSchema, MaterializationError> {
        request.validate()?;
        let lock = self
            .locks
            .get(request)
            .ok_or(MaterializationError::MissingLock)?;
        // Capability-relative open enforces containment during resolution,
        // including symlinks; no ambient canonicalize-then-open race.
        let file = self.root.open(&lock.materialized_path)?;
        if !file.metadata()?.is_file() {
            return Err(invalid("schema materialization must be a regular file"));
        }
        let mut bytes = Vec::new();
        file.take(self.max_bytes + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 > self.max_bytes {
            return Err(MaterializationError::Limit);
        }
        let digest = Self::source_digest(&bytes);
        if digest != lock.source_sha256 {
            return Err(MaterializationError::DigestMismatch);
        }
        let source = String::from_utf8(bytes).map_err(|_| MaterializationError::Utf8)?;
        Ok(MaterializedSchema {
            source,
            canonical_identity: lock.canonical_identity.clone(),
            source_sha256: digest,
        })
    }
}
