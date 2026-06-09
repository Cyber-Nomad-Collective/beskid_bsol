//! BSOL schema profile models and loader.

mod error;
mod load;

pub use error::BsolError;

use std::collections::HashMap;

pub use load::{
    load_profile, load_profile_from_document, load_profile_from_path, load_profile_from_source,
    parse_profile_document,
};

/// Loaded schema profile (for example `project.v1`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaProfile {
    pub name: String,
    pub rules: HashMap<String, BlockRule>,
    pub top_level_order: Vec<String>,
    pub imports: Vec<ImportSchemaSpec>,
}

/// Declarative schema import inside a profile document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportSchemaSpec {
    pub name: String,
    pub alias: Option<String>,
    pub from: ImportSource,
    pub span: bsol_syntax::BsolSpan,
}

/// Where an imported schema profile is resolved from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportSource {
    File { path: String },
    Git {
        url: String,
        rev: String,
        path: String,
    },
    Registry {
        package: String,
        version: String,
        path: String,
    },
    /// Shorthand `@pckg/package/path` form.
    PckgShorthand { reference: String },
}

/// Rule for matching and validating a block kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockRule {
    pub id: String,
    pub scope: RuleScope,
    pub kind_match: KindMatch,
    pub label: LabelRequirement,
    pub cardinality: Cardinality,
    pub fields: HashMap<String, FieldRule>,
    pub nested: HashMap<String, BlockRule>,
    pub nested_order: Vec<String>,
    pub allow_extra_fields: bool,
    pub allow_extra_nested: bool,
    pub schemaless: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleScope {
    TopLevel,
    Nested,
    Any,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KindMatch {
    Keyword(String),
    FreeIdent { except: Vec<String> },
    Keywords(Vec<String>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LabelRequirement {
    #[default]
    Optional,
    Required,
    Forbidden,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Cardinality {
    #[default]
    Many,
    One,
    ZeroOrOne,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldRule {
    pub value_type: ValueType,
    pub required: bool,
    /// When set, list elements must match one of these values.
    pub list_values: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueType {
    Quoted,
    Ident,
    U32,
    List,
    EnumOrQuoted(Vec<String>),
    Loose,
}

impl SchemaProfile {
    pub fn rule(&self, id: &str) -> Option<&BlockRule> {
        self.rules.get(id)
    }

    pub fn top_level_rules(&self) -> impl Iterator<Item = &BlockRule> {
        self.top_level_order
            .iter()
            .filter_map(|id| self.rules.get(id))
    }
}

impl BlockRule {
    pub fn matches_kind(&self, kind: &str) -> bool {
        match &self.kind_match {
            KindMatch::Keyword(k) => kind == k,
            KindMatch::Keywords(keys) => keys.iter().any(|k| k == kind),
            KindMatch::FreeIdent { except } => !except.iter().any(|k| k == kind),
        }
    }

    pub fn nested_rule_for_kind(&self, kind: &str) -> Option<&BlockRule> {
        self.nested
            .values()
            .find(|rule| rule.matches_kind(kind))
            .or_else(|| {
                self.nested
                    .get(kind)
                    .filter(|rule| rule.matches_kind(kind))
            })
    }
}
