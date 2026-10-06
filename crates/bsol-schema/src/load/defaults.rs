//! Check defaults at profile load without converting source values into text.
#![allow(non_snake_case)]

use bsol_syntax::{BsolAssignment, BsolListItem, BsolValue};

use crate::{BsolError, FieldRule, ValueType};

pub(super) fn ValidateDefault(default: &BsolAssignment, rule: &FieldRule) -> Result<(), BsolError> {
    if !MatchesType(&default.value, &rule.value_type) {
        return Err(BsolError::schema_at(
            default.span,
            format!(
                "default `{}` does not match {:?}",
                default.value.preview(),
                rule.value_type
            ),
        ));
    }
    if matches!(rule.value_type, ValueType::U32 | ValueType::I64) {
        let BsolValue::Number(number) = &default.value else {
            unreachable!()
        };
        let value = number
            .raw
            .parse::<i64>()
            .map_err(|_| BsolError::schema_at(default.span, "invalid integer default"))?;
        if rule.constraints.min.is_some_and(|min| value < min)
            || rule.constraints.max.is_some_and(|max| value > max)
        {
            return Err(BsolError::schema_at(
                default.span,
                "default outside field numeric bounds",
            ));
        }
    }
    if let Some(allowed) = &rule.list_values {
        if let BsolValue::BracketList(list) = &default.value {
            for item in &list.items {
                let text = match item {
                    BsolListItem::Default => "default".to_owned(),
                    BsolListItem::QuotedString(value) => value.value.clone(),
                    BsolListItem::Ident(value) => value.clone(),
                    BsolListItem::Number(value) => value.raw.clone(),
                    BsolListItem::Bool(value) => value.to_string(),
                    BsolListItem::Ref(value) => value.display(),
                    _ => continue,
                };
                if !allowed.contains(&text) {
                    return Err(BsolError::schema_at(
                        default.span,
                        format!("unsupported default list value `{text}`"),
                    ));
                }
            }
        }
    }
    Ok(())
}

fn MatchesType(value: &BsolValue, expected: &ValueType) -> bool {
    match (value, expected) {
        (
            BsolValue::QuotedString(_),
            ValueType::Quoted | ValueType::Path | ValueType::Ident | ValueType::Loose,
        ) => true,
        (BsolValue::Ident(_), ValueType::Ident | ValueType::Loose) => true,
        (BsolValue::Bool(_), ValueType::Bool | ValueType::Loose) => true,
        (BsolValue::Ident(value), ValueType::Bool) => value == "true" || value == "false",
        (BsolValue::Number(value), ValueType::U32) => value.raw.parse::<u32>().is_ok(),
        (BsolValue::Number(value), ValueType::I64) => value.raw.parse::<i64>().is_ok(),
        (BsolValue::Number(value), ValueType::F64) => {
            value.raw.parse::<f64>().is_ok_and(f64::is_finite)
        }
        (BsolValue::Ref(reference), ValueType::RefTo(kind)) => reference
            .rule_kind
            .as_ref()
            .is_none_or(|actual| actual == kind),
        (BsolValue::InlineBlock(block), ValueType::Inline(kind)) => block.kind == *kind,
        (BsolValue::BracketList(list), ValueType::Inline(kind)) => {
            matches!(list.items.first(), Some(BsolListItem::InlineBlock(block)) if block.kind == *kind)
        }
        (BsolValue::BracketList(list), ValueType::List) => list.items.iter().all(|item| {
            !matches!(
                item,
                BsolListItem::InlineMap(_) | BsolListItem::InlineBlock(_)
            )
        }),
        (BsolValue::BracketList(list), ValueType::ListOf(types)) => list
            .items
            .iter()
            .all(|item| types.iter().any(|expected| MatchesListItem(item, expected))),
        (BsolValue::InlineMap(map), ValueType::MapOf { value, .. }) => map
            .entries
            .iter()
            .all(|entry| MatchesType(&entry.value, value)),
        (value, ValueType::EnumOrQuoted(allowed)) => {
            let text = match value {
                BsolValue::QuotedString(value) => value.value.clone(),
                BsolValue::Ident(value) => value.clone(),
                BsolValue::Bool(value) => value.to_string(),
                _ => return false,
            };
            allowed.is_empty() || allowed.contains(&text)
        }
        _ => false,
    }
}

fn MatchesListItem(item: &BsolListItem, expected: &ValueType) -> bool {
    let value = match item {
        BsolListItem::QuotedString(value) => BsolValue::QuotedString(value.clone()),
        BsolListItem::Ident(value) => BsolValue::Ident(value.clone()),
        BsolListItem::Number(value) => BsolValue::Number(value.clone()),
        BsolListItem::Bool(value) => BsolValue::Bool(*value),
        BsolListItem::Ref(value) => BsolValue::Ref(value.clone()),
        BsolListItem::InlineBlock(value) => BsolValue::InlineBlock(value.clone()),
        BsolListItem::Default | BsolListItem::InlineMap(_) => return false,
    };
    // List unions retain the narrower supplied-list semantics.
    match (item, expected) {
        (BsolListItem::QuotedString(_), ValueType::Quoted)
        | (BsolListItem::Ident(_), ValueType::Ident)
        | (BsolListItem::Number(_), ValueType::U32 | ValueType::I64 | ValueType::F64)
        | (BsolListItem::Bool(_), ValueType::Bool)
        | (BsolListItem::InlineBlock(_), ValueType::Inline(_)) => MatchesType(&value, expected),
        (BsolListItem::Ref(_), ValueType::RefTo(_)) => true,
        _ => false,
    }
}
