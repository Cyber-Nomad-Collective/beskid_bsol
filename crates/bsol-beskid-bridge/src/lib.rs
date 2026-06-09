//! Optional bridge for registry/git schema imports via Beskid materialization hooks.
//!
//! This crate is workspace-only until the Beskid dependency boundary stabilizes.

use std::path::Path;

use bsol_analysis::{SchemaSource, parse_pckg_shorthand};
use bsol_schema::{BsolError, ImportSchemaSpec, ImportSource, SchemaProfile, load_profile_from_path};

/// Resolves `@pckg/` shorthand and registry imports when a pre-materialized cache root is provided.
#[derive(Debug, Clone)]
pub struct PckgSchemaSource {
    pub cache_root: std::path::PathBuf,
}

impl PckgSchemaSource {
    pub fn new(cache_root: impl Into<std::path::PathBuf>) -> Self {
        Self {
            cache_root: cache_root.into(),
        }
    }
}

impl SchemaSource for PckgSchemaSource {
    fn resolve(&self, spec: &ImportSchemaSpec, _base_dir: &Path) -> Result<SchemaProfile, BsolError> {
        let (package, version, profile_path) = match &spec.from {
            ImportSource::PckgShorthand { reference } => parse_pckg_shorthand(reference)?,
            ImportSource::Registry {
                package,
                version,
                path,
            } => (package.clone(), version.clone(), path.clone()),
            other => {
                return Err(BsolError::Import(format!(
                    "pckg resolver cannot resolve `{other:?}`"
                )));
            }
        };
        let materialized = self
            .cache_root
            .join(package)
            .join(version)
            .join(profile_path);
        load_profile_from_path(&materialized)
    }
}
