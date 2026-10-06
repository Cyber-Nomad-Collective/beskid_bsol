//! Controlled offline schema import bytes and bootstrap tooling adapter.

mod locked;
pub use locked::{
    LockedImport, LockedSchemaHost, MaterializationError, MaterializedSchemaSource,
    SchemaImportRequest,
};
