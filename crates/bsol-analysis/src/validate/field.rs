use std::collections::HashMap;

use bsol_schema::{FieldRule, ValueType};
use bsol_syntax::{BsolAssignment, BsolInlineMap, BsolListItem};

use super::{
    coercion::{
        enum_or_quoted, loose_string, require_bool, require_f64, require_i64, require_ident,
        require_inline_block, require_inline_map, require_list_items, require_list_strings,
        require_quoted, require_ref, require_u32,
    },
    constraints::{apply_numeric_constraints, apply_string_constraints},
};
use crate::{
    BsolError,
    value::{ValidatedBlockLite, ValidatedValue},
};

pub(super) fn validate_field_value(
    assignment: &BsolAssignment,
    rule: &FieldRule,
) -> Result<ValidatedValue, BsolError> {
    let value = match &rule.value_type {
        ValueType::Quoted => ValidatedValue::String(require_quoted(assignment)?),
        ValueType::Ident => ValidatedValue::String(require_ident(assignment)?),
        ValueType::U32 => {
            let text = require_u32(assignment)?;
            let parsed: u32 = text.parse().map_err(|_| {
                BsolError::schema_at(assignment.span, format!("invalid u32 `{text}`"))
            })?;
            apply_numeric_constraints(&rule.constraints, parsed as i64, assignment.span)?;
            ValidatedValue::U32(parsed)
        }
        ValueType::I64 => {
            let text = require_i64(assignment)?;
            let parsed: i64 = text.parse().map_err(|_| {
                BsolError::schema_at(assignment.span, format!("invalid i64 `{text}`"))
            })?;
            apply_numeric_constraints(&rule.constraints, parsed, assignment.span)?;
            ValidatedValue::I64(parsed)
        }
        ValueType::F64 => {
            let text = require_f64(assignment)?;
            let parsed: f64 = text.parse().map_err(|_| {
                BsolError::schema_at(assignment.span, format!("invalid f64 `{text}`"))
            })?;
            ValidatedValue::F64(parsed)
        }
        ValueType::Bool => ValidatedValue::Bool(require_bool(assignment)?),
        ValueType::Path => ValidatedValue::String(require_quoted(assignment)?),
        ValueType::Loose => ValidatedValue::String(loose_string(assignment)?),
        ValueType::List => {
            let list = require_list_strings(assignment)?;
            if let Some(allowed) = &rule.list_values {
                for item in &list {
                    if !allowed.iter().any(|v| v == item) {
                        return Err(BsolError::schema_at(
                            assignment.span,
                            format!("unsupported list value `{item}`"),
                        ));
                    }
                }
            }
            ValidatedValue::List(list.into_iter().map(ValidatedValue::String).collect())
        }
        ValueType::ListOf(element_types) => {
            let items = require_list_items(assignment)?;
            let mut validated = Vec::new();
            for item in items {
                validated.push(validate_list_item(&item, element_types, assignment.span)?);
            }
            ValidatedValue::List(validated)
        }
        ValueType::MapOf { key, value } => {
            let map = require_inline_map(assignment)?;
            validate_inline_map(&map, key, value, assignment.span)?
        }
        ValueType::RefTo(rule_id) => {
            let reference = require_ref(assignment)?;
            if let Some(kind) = &reference.rule_kind {
                if kind != rule_id {
                    return Err(BsolError::schema_at(
                        assignment.span,
                        format!("expected ref({rule_id}), found @{kind}/{}", reference.label),
                    ));
                }
            }
            ValidatedValue::Ref(reference)
        }
        ValueType::Inline(kind) => {
            let block = require_inline_block(assignment)?;
            if block.kind != *kind {
                return Err(BsolError::schema_at(
                    assignment.span,
                    format!("expected inline `{kind}` block"),
                ));
            }
            ValidatedValue::Block(Box::new(ValidatedBlockLite::from_block(&block)))
        }
        ValueType::EnumOrQuoted(values) => {
            ValidatedValue::String(enum_or_quoted(assignment, values)?)
        }
    };
    apply_string_constraints(&rule.constraints, &value, assignment.span)?;
    Ok(value)
}

fn validate_list_item(
    item: &BsolListItem,
    types: &[ValueType],
    span: bsol_syntax::BsolSpan,
) -> Result<ValidatedValue, BsolError> {
    for ty in types {
        if let Ok(value) = try_list_item_as_type(item, ty, span) {
            return Ok(value);
        }
    }
    Err(BsolError::schema_at(
        span,
        "list item does not match allowed types",
    ))
}

fn try_list_item_as_type(
    item: &BsolListItem,
    ty: &ValueType,
    span: bsol_syntax::BsolSpan,
) -> Result<ValidatedValue, BsolError> {
    match (item, ty) {
        (BsolListItem::QuotedString(q), ValueType::Quoted) => {
            Ok(ValidatedValue::String(q.value.clone()))
        }
        (BsolListItem::Ident(i), ValueType::Ident) => Ok(ValidatedValue::String(i.clone())),
        (BsolListItem::Ident(i), ValueType::U32) if i.chars().all(|c| c.is_ascii_digit()) => {
            Ok(ValidatedValue::U32(i.parse().unwrap_or(0)))
        }
        (BsolListItem::Bool(b), ValueType::Bool) => Ok(ValidatedValue::Bool(*b)),
        (BsolListItem::Ref(r), ValueType::RefTo(_)) => Ok(ValidatedValue::Ref(r.clone())),
        (BsolListItem::InlineBlock(block), ValueType::Inline(kind)) if block.kind == *kind => Ok(
            ValidatedValue::Block(Box::new(ValidatedBlockLite::from_block(block))),
        ),
        _ => Err(BsolError::schema_at(span, "type mismatch")),
    }
}

fn validate_inline_map(
    map: &BsolInlineMap,
    _key_ty: &ValueType,
    value_ty: &ValueType,
    span: bsol_syntax::BsolSpan,
) -> Result<ValidatedValue, BsolError> {
    let mut out = HashMap::new();
    for entry in &map.entries {
        let key = ValidatedValue::String(entry.key.clone());
        let fake = BsolAssignment {
            span: entry.span,
            attrs: Vec::new(),
            key: entry.key.clone(),
            value: entry.value.clone(),
        };
        let field = FieldRule {
            value_type: value_ty.clone(),
            required: true,
            list_values: None,
            constraints: Default::default(),
            allowed_attrs: Vec::new(),
        };
        let value = validate_field_value(&fake, &field)?;
        out.insert(key.as_string().unwrap_or(entry.key.clone()), value);
    }
    let _ = span;
    Ok(ValidatedValue::Map(out))
}
