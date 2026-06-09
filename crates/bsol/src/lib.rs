//! BSOL — Beskid Structured Object Language.
//!
//! Facade crate re-exporting parser, schema profiles, validation, and analysis session.

pub use bsol_analysis::{
    AnalysisOptions, AnalysisSession, BsolError, CompositeSchemaSource, FileSchemaSource,
    SchemaCollection, SchemaSource, ValidatedBlock, ValidatedDocument, ValidatorRegistry,
    analyze_with_profile, load_fetched_profile, load_profile_text, parse_pckg_shorthand,
    profile_base_dir, resolve_profile, validate, validate_profile_document, validate_with,
};
pub use bsol_schema::{
    BlockRule, Cardinality, FieldRule, ImportSchemaSpec, ImportSource, KindMatch,
    LabelRequirement, RuleScope, SchemaProfile, ValueType, load_profile,
    load_profile_from_document, load_profile_from_path, load_profile_from_source,
    parse_profile_document,
};
pub use bsol_syntax::{
    BsolAssignment, BsolBlock, BsolBracketList, BsolDocument, BsolItem, BsolListItem,
    BsolParser, BsolQuotedString, BsolSpan, BsolValue, Rule, parse_bsol_document, parse_document,
};