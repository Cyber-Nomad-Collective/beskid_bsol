//! Stable phase identifiers for BSOL pipeline observers.

/// Parse BSOL source into generic AST.
pub const PARSE_SYNTAX: &str = "parse.syntax";
/// Collect and resolve `import_schema` declarations.
pub const SCHEMA_COLLECT: &str = "schema.collect";
/// Resolve file-backed schema imports.
pub const SCHEMA_RESOLVE_FILE: &str = "schema.resolve.file";
/// Resolve git-backed schema imports.
pub const SCHEMA_RESOLVE_GIT: &str = "schema.resolve.git";
/// Resolve registry-backed schema imports.
pub const SCHEMA_RESOLVE_REGISTRY: &str = "schema.resolve.registry";
/// Structural validation against resolved profile.
pub const SCHEMA_VALIDATE: &str = "schema.validate";
/// Custom semantic validators and cross-reference rules.
pub const SCHEMA_SEMANTIC: &str = "schema.semantic";
/// Immutable validated document boundary.
pub const SCHEMA_SNAPSHOT: &str = "schema.snapshot";
/// Plan profile migration routes for a document.
pub const MIGRATE_PLAN: &str = "migrate.plan";
/// Apply profile migration rewrites to source text.
pub const MIGRATE_APPLY: &str = "migrate.apply";
