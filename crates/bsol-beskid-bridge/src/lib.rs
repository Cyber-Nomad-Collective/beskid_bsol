//! Controlled offline schema import bytes and bootstrap tooling adapter.
pub use bsol_analysis::MaterializedSchema;

mod locked;
pub use locked::{LockedImport, LockedSchemaHost, MaterializationError, SchemaImportRequest};
