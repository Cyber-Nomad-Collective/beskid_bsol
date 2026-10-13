//! Facade crate re-exporting parser, schema profiles, validation, and analysis session.

pub use bsol_analysis::{
    analyze_with_profile, apply_migration, load_fetched_profile, load_profile_text,
    migrate_document, parse_pckg_shorthand, plan_migration, profile_base_dir,
    resolve_active_profile, resolve_profile, resolve_references, validate,
    validate_profile_document, validate_with, validate_with_pattern_limits, AnalysisOptions,
    AnalysisSession, BsolError, CompositeSchemaSource, MigrationPlan, MigrationRoute,
    PatternLimits, SchemaCollection, SchemaSource, ValidatedBlock, ValidatedBlockLite,
    ValidatedDocument, ValidatedValue, ValidatorRegistry,
};
pub use bsol_schema::{
    compose_profile, load_profile, load_profile_from_document, load_profile_from_path,
    load_profile_from_source, merge_profiles, parse_profile_document, BlockRule, Cardinality,
    ExtendSpec, FieldConstraints, FieldRule, ImportSchemaSpec, ImportSource, KindMatch,
    LabelRequirement, MigrationRewrite, MigrationSpec, MigrationWhenClause, RuleScope,
    SchemaProfile, ValueType, VariantRule,
};
pub use bsol_syntax::{
    parse_bsol_document, parse_bsol_document_with_limits, parse_document, write_bsol_document,
    write_bsol_value, BsolAssignment, BsolAttribute, BsolAttributeArg, BsolBlock, BsolBracketList,
    BsolDocument, BsolInlineMap, BsolItem, BsolLimitKind, BsolLimits, BsolListItem, BsolMapEntry,
    BsolNumber, BsolParser, BsolQuotedString, BsolRef, BsolSpan, BsolValue, Rule,
};
