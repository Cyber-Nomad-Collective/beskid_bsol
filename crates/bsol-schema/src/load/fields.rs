use std::collections::HashMap;

use bsol_syntax::{BsolBlock, BsolItem, BsolValue};

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
        default_value: block.items.iter().find_map(|item| match item {
            BsolItem::Assignment(assignment) if assignment.key == "default" => {
                Some(assignment.clone())
            }
            _ => None,
        }),
        min: assignment_string(block, "min")?.and_then(|v| v.parse().ok()),
        max: assignment_string(block, "max")?.and_then(|v| v.parse().ok()),
        pattern: assignment_string(block, "pattern")?,
        required_if: parse_if_map(block, "required_if")?,
        forbid_if: parse_if_map(block, "forbid_if")?,
    };
    if let Some(pattern) = &constraints.pattern {
        crate::pattern::parse(pattern, 64)
            .map_err(|error| BsolError::schema_at(block.span, error.to_string()))?;
    }
    let rule = FieldRule {
        value_type,
        required,
        list_values,
        constraints,
        allowed_attrs,
    };
    if let Some(default) = &rule.constraints.default_value {
        super::defaults::ValidateDefault(default, &rule)?;
    }
    Ok(rule)
}

fn parse_if_map(block: &BsolBlock, key: &str) -> Result<HashMap<String, String>, BsolError> {
    let mut result = None;
    for item in &block.items {
        let (span, entries) = match item {
            BsolItem::Assignment(assignment) if assignment.key == key => {
                let BsolValue::InlineMap(map) = &assignment.value else {
                    return Err(BsolError::schema_at(
                        assignment.span,
                        format!("`{key}` requires an inline map"),
                    ));
                };
                (
                    assignment.span,
                    map.entries
                        .iter()
                        .map(|entry| (entry.key.as_str(), &entry.value, entry.span))
                        .collect::<Vec<_>>(),
                )
            }
            BsolItem::Block(nested) if nested.kind == key => {
                let mut entries = Vec::new();
                for entry in &nested.items {
                    let BsolItem::Assignment(assignment) = entry else {
                        return Err(BsolError::schema_at(
                            nested.span,
                            format!("`{key}` requires condition assignments"),
                        ));
                    };
                    entries.push((assignment.key.as_str(), &assignment.value, assignment.span));
                }
                (nested.span, entries)
            }
            _ => continue,
        };
        if result.is_some() {
            return Err(BsolError::schema_at(
                span,
                format!("duplicate `{key}` constraint"),
            ));
        }
        let mut map = HashMap::new();
        for (name, value, span) in entries {
            let value = value_as_string(value)
                .map_err(|_| BsolError::schema_at(span, "condition requires a scalar value"))?;
            if map.insert(name.to_owned(), value).is_some() {
                return Err(BsolError::schema_at(
                    span,
                    format!("duplicate condition `{name}`"),
                ));
            }
        }
        result = Some(map);
    }
    if let Some(map) = result {
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
