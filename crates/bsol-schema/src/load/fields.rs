use std::collections::HashMap;

use bsol_syntax::{BsolBlock, BsolItem};

use super::value_decode::{
    assignment_list, assignment_string, parse_bool, parse_value_type_text, value_as_string,
};
use crate::{BsolError, FieldConstraints, FieldRule, ValueType};

pub(super) fn parse_field_rule(block: &BsolBlock) -> Result<FieldRule, BsolError> {
    let value_type = parse_value_type(block)?;
    let required = parse_bool(block, "required").unwrap_or(false);
    let list_values = assignment_list(block, "list_values")
        .ok()
        .filter(|v| !v.is_empty());
    let allowed_attrs = assignment_list(block, "allowed_attrs").unwrap_or_default();
    let constraints = FieldConstraints {
        default_value: assignment_string(block, "default")?,
        min: assignment_string(block, "min")?.and_then(|v| v.parse().ok()),
        max: assignment_string(block, "max")?.and_then(|v| v.parse().ok()),
        pattern: assignment_string(block, "pattern")?,
        required_if: parse_if_map(block, "required_if")?,
        forbid_if: parse_if_map(block, "forbid_if")?,
    };
    Ok(FieldRule {
        value_type,
        required,
        list_values,
        constraints,
        allowed_attrs,
    })
}

fn parse_if_map(block: &BsolBlock, key: &str) -> Result<HashMap<String, String>, BsolError> {
    for item in &block.items {
        let BsolItem::Block(nested) = item else {
            continue;
        };
        if nested.kind != key {
            continue;
        }
        let mut map = HashMap::new();
        for entry in &nested.items {
            let BsolItem::Assignment(a) = entry else {
                continue;
            };
            map.insert(a.key.clone(), value_as_string(&a.value)?);
        }
        return Ok(map);
    }
    Ok(HashMap::new())
}

fn parse_value_type(block: &BsolBlock) -> Result<ValueType, BsolError> {
    let ty = assignment_string(block, "type")?
        .ok_or_else(|| BsolError::schema_at(block.span, "`field` block requires `type`"))?;
    if ty == "enum_or_quoted" {
        let values = assignment_list(block, "values")?;
        return Ok(ValueType::EnumOrQuoted(values));
    }
    parse_value_type_text(&ty).map_err(|msg| BsolError::schema_at(block.span, msg))
}
