//! Typed validated values (v2).

use std::collections::HashMap;

use bsol_syntax::{BsolBlock, BsolRef};

/// Typed field value after schema validation.
#[derive(Debug, Clone, PartialEq)]
pub enum ValidatedValue {
    String(String),
    U32(u32),
    I64(i64),
    F64(f64),
    Bool(bool),
    List(Vec<ValidatedValue>),
    Map(HashMap<String, ValidatedValue>),
    Ref(BsolRef),
    Block(Box<ValidatedBlockLite>),
}

/// Lightweight inline block validation result.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedBlockLite {
    pub kind: String,
    pub label: Option<String>,
    pub values: HashMap<String, ValidatedValue>,
}

impl ValidatedBlockLite {
    pub fn from_block(block: &BsolBlock) -> Self {
        let mut values = HashMap::new();
        for item in &block.items {
            if let bsol_syntax::BsolItem::Assignment(a) = item {
                if let Some(s) = raw_to_string(&a.value) {
                    values.insert(a.key.clone(), ValidatedValue::String(s));
                }
            }
        }
        Self {
            kind: block.kind.clone(),
            label: block.label.as_ref().map(|q| q.value.clone()),
            values,
        }
    }
}

impl ValidatedValue {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            ValidatedValue::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_string(&self) -> Option<String> {
        match self {
            ValidatedValue::String(s) => Some(s.clone()),
            ValidatedValue::U32(v) => Some(v.to_string()),
            ValidatedValue::I64(v) => Some(v.to_string()),
            ValidatedValue::Bool(v) => Some(v.to_string()),
            ValidatedValue::Ref(r) => Some(r.display()),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<&[ValidatedValue]> {
        match self {
            ValidatedValue::List(items) => Some(items),
            _ => None,
        }
    }

    pub fn as_list_strings(&self) -> Option<Vec<String>> {
        match self {
            ValidatedValue::List(items) => Some(
                items
                    .iter()
                    .filter_map(|v| v.as_string())
                    .collect(),
            ),
            _ => None,
        }
    }
}

pub(crate) fn raw_to_string(value: &bsol_syntax::BsolValue) -> Option<String> {
    match value {
        bsol_syntax::BsolValue::QuotedString(q) => Some(q.value.clone()),
        bsol_syntax::BsolValue::Ident(i) => Some(i.clone()),
        bsol_syntax::BsolValue::Bool(b) => Some(b.to_string()),
        bsol_syntax::BsolValue::Ref(r) => Some(r.display()),
        _ => None,
    }
}
