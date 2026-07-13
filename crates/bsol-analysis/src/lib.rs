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
    apply_migration, migrate_document, plan_migration, MigrationPlan, MigrationRoute,
};
pub use registry::ValidatorRegistry;
pub use resolver::{
    load_fetched_profile, load_profile_text, parse_pckg_shorthand, profile_base_dir,
    resolve_active_profile, resolve_profile, CompositeSchemaSource, FileSchemaSource,
    SchemaCollection, SchemaSource,
};
pub use semantic::resolve_references;
pub use session::{
    analyze_with_profile, validate_profile_document, AnalysisOptions, AnalysisSession,
};
pub use validate::{validate, validate_with, ValidatedBlock, ValidatedDocument};
pub use value::{ValidatedBlockLite, ValidatedValue};
