//! Unified BSOL analysis session driving the pipeline phases.

use std::path::{Path, PathBuf};

use bsol_pipeline::{
    NullObserver, PipelineObserver, observe_phase,
    phases::{
        PARSE_SYNTAX, SCHEMA_COLLECT, SCHEMA_RESOLVE_FILE, SCHEMA_SEMANTIC, SCHEMA_SNAPSHOT,
        SCHEMA_VALIDATE,
    },
};
use bsol_schema::{BsolError, SchemaProfile, load_profile, load_profile_from_source};
use bsol_syntax::{BsolDocument, parse_bsol_document};

use crate::registry::ValidatorRegistry;
use crate::resolver::{CompositeSchemaSource, resolve_profile};
use crate::validate::{ValidatedDocument, validate_with};

/// Options for a single document analysis run.
#[derive(Debug, Clone)]
pub struct AnalysisOptions {
    pub profile_name: String,
    pub base_dir: PathBuf,
}

impl AnalysisOptions {
    pub fn for_profile(name: &str) -> Self {
        Self {
            profile_name: name.to_string(),
            base_dir: PathBuf::from("."),
        }
    }

    pub fn with_base_dir(mut self, base_dir: impl Into<PathBuf>) -> Self {
        self.base_dir = base_dir.into();
        self
    }
}

/// Stateful analysis session (parse → resolve schemas → validate).
pub struct AnalysisSession {
    observer: Box<dyn PipelineObserver>,
    registry: ValidatorRegistry,
    source: CompositeSchemaSource,
}

impl Default for AnalysisSession {
    fn default() -> Self {
        Self::new()
    }
}

impl AnalysisSession {
    pub fn new() -> Self {
        Self {
            observer: Box::new(NullObserver),
            registry: ValidatorRegistry::default(),
            source: CompositeSchemaSource::file_only(),
        }
    }

    pub fn with_observer(mut self, observer: Box<dyn PipelineObserver>) -> Self {
        self.observer = observer;
        self
    }

    pub fn registry_mut(&mut self) -> &mut ValidatorRegistry {
        &mut self.registry
    }

    pub fn add_schema_source(&mut self, source: Box<dyn crate::resolver::SchemaSource>) {
        self.source.add_source(source);
    }

    pub fn analyze_source(
        &mut self,
        source: &str,
        options: &AnalysisOptions,
    ) -> Result<ValidatedDocument, BsolError> {
        let document = observe_phase(self.observer.as_mut(), PARSE_SYNTAX, || {
            parse_bsol_document(source).map_err(BsolError::from)
        })?;

        let profile = load_profile(&options.profile_name)?;
        self.analyze_document(&document, profile, &options.base_dir)
    }

    pub fn analyze_document(
        &mut self,
        document: &BsolDocument,
        profile: SchemaProfile,
        base_dir: &Path,
    ) -> Result<ValidatedDocument, BsolError> {
        let profile_name = profile.name.clone();
        let _collection = observe_phase(self.observer.as_mut(), SCHEMA_COLLECT, || {
            resolve_profile(profile, base_dir, &self.source)
        })?;
        observe_phase(self.observer.as_mut(), SCHEMA_RESOLVE_FILE, || Ok(()))?;

        let active = load_profile(&profile_name)?;

        let validated = observe_phase(self.observer.as_mut(), SCHEMA_VALIDATE, || {
            validate_with(document, &active, &self.registry)
        })?;

        observe_phase(self.observer.as_mut(), SCHEMA_SEMANTIC, || Ok(()))?;
        observe_phase(self.observer.as_mut(), SCHEMA_SNAPSHOT, || Ok(()))?;

        Ok(validated)
    }

    /// Validate a document against an already-loaded profile (no import resolution).
    pub fn validate_document(
        &self,
        document: &BsolDocument,
        profile: &SchemaProfile,
    ) -> Result<ValidatedDocument, BsolError> {
        validate_with(document, profile, &self.registry)
    }
}

/// Convenience: parse, load embedded profile, validate.
pub fn analyze_with_profile(
    source: &str,
    profile_name: &str,
) -> Result<ValidatedDocument, BsolError> {
    let document = parse_bsol_document(source).map_err(BsolError::from)?;
    let profile = load_profile(profile_name)?;
    validate_with(&document, &profile, &ValidatorRegistry::default())
}

/// Load and validate a schema profile document against the meta-schema.
pub fn validate_profile_document(source: &str) -> Result<SchemaProfile, BsolError> {
    let meta = load_profile("schema.v1")?;
    let document = parse_bsol_document(source).map_err(BsolError::from)?;
    validate_with(&document, &meta, &ValidatorRegistry::default())?;
    load_profile_from_source(source)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analyze_minimal_project_block() {
        let src = r#"demo {
  name = "demo"
  version = "0.1.0"
  root = "."
}
"#;
        let result = analyze_with_profile(src, "project.v1");
        assert!(result.is_ok(), "{:?}", result.err());
    }
}
