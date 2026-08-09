use std::collections::HashMap;

use bsol_syntax::BsolAttribute;

use crate::value::ValidatedValue;

/// Document validated against a schema profile; blocks carry matched rule ids.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedDocument {
    pub profile: String,
    pub blocks: Vec<ValidatedBlock>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedBlock {
    pub span: bsol_syntax::BsolSpan,
    pub rule_id: String,
    pub kind: String,
    pub label: Option<String>,
    pub attrs: Vec<BsolAttribute>,
    pub fields: HashMap<String, String>,
    pub values: HashMap<String, ValidatedValue>,
    pub field_spans: HashMap<String, bsol_syntax::BsolSpan>,
    pub extras: HashMap<String, String>,
    pub nested: Vec<ValidatedBlock>,
    pub lists: HashMap<String, Vec<String>>,
    pub raw_body: Option<String>,
}

impl ValidatedBlock {
    pub fn field(&self, key: &str) -> Option<&str> {
        self.fields.get(key).map(String::as_str)
    }

    pub fn value(&self, key: &str) -> Option<&ValidatedValue> {
        self.values.get(key)
    }
}
