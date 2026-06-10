//! Facade crate re-exporting parser, schema profiles, validation, and analysis session.

pub use bsol_analysis::{
    AnalysisOptions, AnalysisSession, BsolError, CompositeSchemaSource, FileSchemaSource,
    MigrationPlan, MigrationRoute, SchemaCollection, SchemaSource, ValidatedBlock,
    ValidatedBlockLite, ValidatedDocument, ValidatedValue, ValidatorRegistry,
    analyze_with_profile, apply_migration, load_fetched_profile, load_profile_text,
    migrate_document, parse_pckg_shorthand, plan_migration, profile_base_dir,
    resolve_active_profile, resolve_profile, resolve_references, validate,
    validate_profile_document, validate_with,
};
pub use bsol_schema::{
    BlockRule, Cardinality, ExtendSpec, FieldConstraints, FieldRule, ImportSchemaSpec,
    ImportSource, KindMatch, LabelRequirement, MigrationRewrite, MigrationSpec,
    MigrationWhenClause, RuleScope, SchemaProfile, ValueType, VariantRule, compose_profile,
    load_profile, load_profile_from_document, load_profile_from_path, load_profile_from_source,
    merge_profiles, parse_profile_document,
};
pub use bsol_syntax::{
    BsolAssignment, BsolAttribute, BsolAttributeArg, BsolBlock, BsolBracketList, BsolDocument,
    BsolInlineMap, BsolItem, BsolListItem, BsolMapEntry, BsolParser, BsolQuotedString, BsolRef,
    BsolSpan, BsolValue, Rule, parse_bsol_document, parse_document,
};
