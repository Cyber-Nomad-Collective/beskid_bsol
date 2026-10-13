//! Resolved schema import closure with the native `Core.Bsol.ImportGraph` budgets.
//!
//! A [`SchemaSource`] host materializes owned bytes for each `import_schema` request; this module
//! verifies them, parses them under the remaining aggregate node budget and debits the closure's
//! import count, import depth and resolved schema bytes in native order. The first failure is
//! returned with the native code and message, and no partial graph is published.

use std::path::{Path, PathBuf};

use bsol_schema::{
    load_profile, load_profile_from_document, load_profile_from_source, parse_import_schema,
    BsolError, ImportSchemaSpec, SchemaProfile,
};
use bsol_syntax::{
    parse_bsol_document_counted, BsolDocument, BsolItem, BsolLimits, BsolSpan, BsolValue,
};
use sha2::{Digest, Sha256};

/// Owned schema source returned by a host, bound to its canonical identity and content digest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaterializedSchema {
    pub source: String,
    pub canonical_identity: String,
    pub source_sha256: [u8; 32],
}

impl MaterializedSchema {
    /// Bind source the caller already owns (for example an embedded or package-local profile).
    pub fn owned(source: String, canonical_identity: String) -> Self {
        let source_sha256 = Self::digest(source.as_bytes());
        Self {
            source,
            canonical_identity,
            source_sha256,
        }
    }

    pub fn digest(bytes: &[u8]) -> [u8; 32] {
        Sha256::digest(bytes).into()
    }
}

/// Host failure with a stable code, retained as the graph failure code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportFailure {
    pub code: String,
    pub message: String,
}

impl ImportFailure {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

/// Host that materializes the owned bytes of one schema import request.
pub trait SchemaSource: Send + Sync {
    fn materialize(
        &self,
        spec: &ImportSchemaSpec,
        base_dir: &Path,
    ) -> Result<MaterializedSchema, ImportFailure>;
}

/// First failure while resolving an import closure; `profile` names the declaring profile.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{code}: {message}")]
pub struct ImportGraphFailure {
    pub code: String,
    pub message: String,
    pub span: BsolSpan,
    pub profile: String,
}

impl From<ImportGraphFailure> for BsolError {
    fn from(failure: ImportGraphFailure) -> Self {
        BsolError::schema_at(failure.span, failure.to_string())
    }
}

/// One verified source of the closure.
#[derive(Debug, Clone)]
pub struct ResolvedSchema {
    pub materialized: MaterializedSchema,
    pub document: BsolDocument,
    pub profile: String,
    active: bool,
}

/// Namespace edge from a declaring source to an imported source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportBinding {
    pub owner: usize,
    pub target: usize,
    pub name: String,
    pub span: BsolSpan,
}

/// Every source and namespace binding owned by one resolved closure.
#[derive(Debug, Clone)]
pub struct ImportGraph {
    pub sources: Vec<ResolvedSchema>,
    pub bindings: Vec<ImportBinding>,
    pub root: usize,
}

fn failure(
    code: &str,
    message: impl Into<String>,
    span: BsolSpan,
    profile: &str,
) -> ImportGraphFailure {
    ImportGraphFailure {
        code: code.to_string(),
        message: message.into(),
        span,
        profile: profile.to_string(),
    }
}

/// Native `Imports.VerifyMaterialized`.
fn verify(
    materialized: &MaterializedSchema,
    max_bytes: usize,
    span: BsolSpan,
    profile: &str,
) -> Result<(), ImportGraphFailure> {
    if materialized.canonical_identity.is_empty() {
        return Err(failure(
            "MissingImportIdentity",
            "host returned no canonical materialization identity",
            span,
            profile,
        ));
    }
    if materialized.source.len() > max_bytes {
        return Err(failure(
            "LimitExceeded",
            "materialized source exceeds the resolved schema budget",
            span,
            profile,
        ));
    }
    if MaterializedSchema::digest(materialized.source.as_bytes()) != materialized.source_sha256 {
        return Err(failure(
            "ImportDigestMismatch",
            "materialized source does not match its locked content digest",
            span,
            profile,
        ));
    }
    Ok(())
}

struct Resolution<'a> {
    graph: ImportGraph,
    limits: &'a BsolLimits,
    base_dir: &'a Path,
    host: &'a dyn SchemaSource,
    bytes: usize,
    nodes: usize,
    imports: usize,
}

impl Resolution<'_> {
    fn visit(
        &mut self,
        materialized: MaterializedSchema,
        expected: &str,
        depth: usize,
        span: BsolSpan,
        owner: &str,
    ) -> Result<usize, ImportGraphFailure> {
        if depth > self.limits.max_import_depth {
            return Err(failure(
                "ImportDepthLimit",
                "resolved import depth exceeded",
                span,
                owner,
            ));
        }
        // An identity may be reused only with identical content. An active reuse is a cycle; a
        // completed reuse is a diamond and does not debit source twice.
        for (index, previous) in self.graph.sources.iter().enumerate() {
            if previous.materialized.canonical_identity != materialized.canonical_identity {
                continue;
            }
            if previous.materialized.source_sha256 != materialized.source_sha256
                || previous.materialized.source != materialized.source
            {
                return Err(failure(
                    "AmbiguousImportIdentity",
                    "one canonical identity returned different bytes",
                    span,
                    owner,
                ));
            }
            if previous.active {
                return Err(failure(
                    "ImportCycle",
                    "active canonical source imported recursively",
                    span,
                    owner,
                ));
            }
            if !expected.is_empty() && previous.profile != expected {
                return Err(failure(
                    "ImportProfileMismatch",
                    "materialization contains another profile",
                    span,
                    owner,
                ));
            }
            return Ok(index);
        }
        let source_bytes = materialized.source.len();
        let remaining_bytes = self.limits.max_resolved_schema_bytes - self.bytes;
        if source_bytes > remaining_bytes {
            return Err(failure(
                "ResolvedSchemaLimit",
                "aggregate schema bytes exceeded",
                span,
                owner,
            ));
        }
        verify(&materialized, remaining_bytes, span, owner)?;
        // Parsing itself uses the remaining aggregate node budget.
        let remaining_nodes = self.limits.max_nodes - self.nodes;
        if remaining_nodes < 1 {
            return Err(failure(
                "ResolvedNodeLimit",
                "aggregate schema nodes exceeded",
                span,
                owner,
            ));
        }
        let parse_limits = BsolLimits {
            max_nodes: remaining_nodes,
            ..*self.limits
        };
        let (document, nodes) = parse_bsol_document_counted(&materialized.source, &parse_limits)
            .map_err(|error| {
                let message = match &error {
                    bsol_syntax::BsolError::Limit { message, .. } => (*message).to_string(),
                    other => other.to_string(),
                };
                failure("ImportSyntaxError", message, span, owner)
            })?;
        if document.blocks.len() != 1 {
            return Err(failure(
                "ProfileRootRequired",
                "one profile root required",
                span,
                owner,
            ));
        }
        let root = &document.blocks[0];
        let label = root.label.as_ref().map_or("", |label| label.value.as_str());
        if root.kind != "profile" || label.is_empty() {
            return Err(failure(
                "ProfileRootRequired",
                "labelled profile root required",
                root.span,
                owner,
            ));
        }
        if !expected.is_empty() && label != expected {
            return Err(failure(
                "ImportProfileMismatch",
                "materialization contains another profile",
                root.span,
                owner,
            ));
        }
        let profile = label.to_string();
        // Every source of a resolved closure declares exactly one supported schema version.
        let mut versions = 0usize;
        for item in &root.items {
            let BsolItem::Assignment(property) = item else {
                continue;
            };
            if property.key != "version" {
                continue;
            }
            versions += 1;
            let text = match &property.value {
                BsolValue::Number(number) => Some(number.raw.as_str()),
                BsolValue::QuotedString(quoted) => Some(quoted.value.as_str()),
                _ => None,
            };
            if !matches!(text, Some("1" | "2")) {
                return Err(failure(
                    "InvalidProfileVersion",
                    "supported schema version required",
                    property.span,
                    &profile,
                ));
            }
        }
        if versions != 1 {
            return Err(failure(
                "InvalidProfileVersion",
                "exactly one schema version required",
                root.span,
                &profile,
            ));
        }
        let declarations = root
            .items
            .iter()
            .filter_map(|item| match item {
                BsolItem::Block(block) if block.kind == "import_schema" => Some(block.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        let current = self.graph.sources.len();
        self.graph.sources.push(ResolvedSchema {
            materialized,
            document,
            profile: profile.clone(),
            active: true,
        });
        self.bytes += source_bytes;
        self.nodes += nodes;
        for declaration in &declarations {
            if self.imports >= self.limits.max_imports {
                return Err(failure(
                    "ImportCountLimit",
                    "aggregate import count exceeded",
                    declaration.span,
                    &profile,
                ));
            }
            self.imports += 1;
            let spec = parse_import_schema(declaration).map_err(|error| {
                failure(
                    "InvalidImportDeclaration",
                    error.to_string(),
                    declaration.span,
                    &profile,
                )
            })?;
            let binding = spec.alias.clone().unwrap_or_else(|| spec.name.clone());
            if self
                .graph
                .bindings
                .iter()
                .any(|existing| existing.owner == current && existing.name == binding)
            {
                return Err(failure(
                    "DuplicateImportAlias",
                    "import namespace binding is repeated",
                    spec.span,
                    &profile,
                ));
            }
            let imported = self
                .host
                .materialize(&spec, self.base_dir)
                .map_err(|error| failure(&error.code, error.message, spec.span, &profile))?;
            verify(
                &imported,
                self.limits.max_resolved_schema_bytes,
                spec.span,
                &profile,
            )?;
            let target = self.visit(imported, &spec.name, depth + 1, spec.span, &profile)?;
            self.graph.bindings.push(ImportBinding {
                owner: current,
                target,
                name: binding,
                span: spec.span,
            });
        }
        self.graph.sources[current].active = false;
        Ok(current)
    }
}

/// Resolve and verify the whole import closure of `root`; publish it only when every import
/// resolved within budget.
pub fn resolve_import_graph(
    root: MaterializedSchema,
    base_dir: &Path,
    host: &dyn SchemaSource,
    limits: &BsolLimits,
) -> Result<ImportGraph, ImportGraphFailure> {
    let root_span = BsolSpan {
        start: 0,
        end: root.source.len(),
        line: 1,
    };
    if !limits.is_valid() {
        return Err(failure(
            "InvalidLimits",
            "invalid import closure limits",
            root_span,
            "",
        ));
    }
    let mut resolution = Resolution {
        graph: ImportGraph {
            sources: Vec::new(),
            bindings: Vec::new(),
            root: 0,
        },
        limits,
        base_dir,
        host,
        bytes: 0,
        nodes: 0,
        imports: 0,
    };
    let root_index = resolution.visit(root, "", 0, root_span, "")?;
    resolution.graph.root = root_index;
    Ok(resolution.graph)
}

/// Compose the active validation profile of `root` from its resolved imports and extends.
pub fn resolve_active_profile(
    root: MaterializedSchema,
    base_dir: &Path,
    host: &dyn SchemaSource,
    limits: &BsolLimits,
) -> Result<SchemaProfile, BsolError> {
    let graph = resolve_import_graph(root, base_dir, host, limits)?;
    let profiles = graph
        .sources
        .iter()
        .map(|source| load_profile_from_document(&source.document))
        .collect::<Result<Vec<_>, _>>()?;
    let mut active = profiles[graph.root].clone();
    for binding in graph
        .bindings
        .iter()
        .filter(|binding| binding.owner == graph.root)
    {
        active = bsol_schema::merge_profiles(profiles[binding.target].clone(), active);
    }
    for extend in &active.extends.clone() {
        if let Some(base) = profiles.iter().find(|profile| profile.name == extend.base) {
            active = bsol_schema::merge_profiles(base.clone(), active);
        } else if let Ok(base) = load_profile(&extend.base) {
            let mut overlay = active.clone();
            overlay.extends = vec![extend.clone()];
            active = bsol_schema::merge_profiles(base, overlay);
        }
    }
    bsol_schema::compose_profile(active)
}

/// Composite host trying each source in order.
pub struct CompositeSchemaSource {
    sources: Vec<Box<dyn SchemaSource>>,
}

impl CompositeSchemaSource {
    pub fn new(sources: Vec<Box<dyn SchemaSource>>) -> Self {
        Self { sources }
    }

    pub fn add_source(&mut self, source: Box<dyn SchemaSource>) {
        self.sources.push(source);
    }
}

impl SchemaSource for CompositeSchemaSource {
    fn materialize(
        &self,
        spec: &ImportSchemaSpec,
        base_dir: &Path,
    ) -> Result<MaterializedSchema, ImportFailure> {
        let mut last = None;
        for source in &self.sources {
            match source.materialize(spec, base_dir) {
                Ok(materialized) => return Ok(materialized),
                Err(error) => last = Some(error),
            }
        }
        Err(last.unwrap_or_else(|| {
            ImportFailure::new(
                "MissingCache",
                format!("no resolver for schema `{}`", spec.name),
            )
        }))
    }
}

/// Parse `@pckg/package/version/path` shorthand into registry import fields.
pub fn parse_pckg_shorthand(reference: &str) -> Result<(String, String, String), BsolError> {
    let rest = reference
        .strip_prefix("@pckg/")
        .ok_or_else(|| BsolError::Import(format!("invalid pckg reference `{reference}`")))?;
    let mut parts = rest.splitn(3, '/');
    let package = parts
        .next()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| BsolError::Import("pckg reference missing package".into()))?
        .to_string();
    let version = parts
        .next()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| BsolError::Import("pckg reference missing version".into()))?
        .to_string();
    let path = parts
        .next()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| BsolError::Import("pckg reference missing profile path".into()))?
        .to_string();
    Ok((package, version, path))
}

/// Load profile text from a resolved path (used by bridge implementations).
pub fn load_profile_text(path: &Path) -> Result<String, BsolError> {
    std::fs::read_to_string(path)
        .map_err(|e| BsolError::Import(format!("read `{}`: {e}", path.display())))
}

/// Load profile from text after external fetch.
pub fn load_fetched_profile(source: &str) -> Result<SchemaProfile, BsolError> {
    load_profile_from_source(source)
}

/// Base directory helper for profile documents on disk.
pub fn profile_base_dir(path: Option<&Path>) -> PathBuf {
    path.and_then(|p| p.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_pckg_reference() {
        let (pkg, ver, path) =
            parse_pckg_shorthand("@pckg/beskid-schemas/1.0.0/project.v1.bsol").expect("parse");
        assert_eq!(pkg, "beskid-schemas");
        assert_eq!(ver, "1.0.0");
        assert_eq!(path, "project.v1.bsol");
    }
}
