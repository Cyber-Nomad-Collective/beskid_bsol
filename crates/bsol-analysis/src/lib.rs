//! BSOL schema resolution, validation, and analysis session.

mod migrate;
mod registry;
mod resolver;
mod semantic;
mod session;
mod validate;
mod value;

pub use bsol_schema::BsolError;
pub use migrate::{
    MigrationPlan, MigrationRoute, apply_migration, migrate_document, plan_migration,
};
pub use registry::ValidatorRegistry;
pub use resolver::{
    CompositeSchemaSource, FileSchemaSource, SchemaCollection, SchemaSource,
    load_fetched_profile, load_profile_text, parse_pckg_shorthand, profile_base_dir,
    resolve_profile, resolve_active_profile,
};
pub use semantic::resolve_references;
pub use session::{AnalysisOptions, AnalysisSession, analyze_with_profile, validate_profile_document};
pub use validate::{ValidatedBlock, ValidatedDocument, validate, validate_with};
pub use value::{ValidatedBlockLite, ValidatedValue};
