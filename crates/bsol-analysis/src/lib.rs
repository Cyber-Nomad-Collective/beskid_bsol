//! BSOL schema resolution, validation, and analysis session.

mod registry;
mod resolver;
mod session;
mod validate;

pub use bsol_schema::BsolError;
pub use registry::ValidatorRegistry;
pub use resolver::{
    CompositeSchemaSource, FileSchemaSource, SchemaCollection, SchemaSource,
    load_fetched_profile, load_profile_text, parse_pckg_shorthand, profile_base_dir,
    resolve_profile,
};
pub use session::{AnalysisOptions, AnalysisSession, analyze_with_profile, validate_profile_document};
pub use validate::{ValidatedBlock, ValidatedDocument, validate, validate_with};
