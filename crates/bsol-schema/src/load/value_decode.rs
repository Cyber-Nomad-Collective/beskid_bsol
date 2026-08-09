use bsol_syntax::{BsolBlock, BsolItem, BsolListItem, BsolValue};

use crate::{BsolError, ValueType};

pub(crate) fn parse_value_type_text(text: &str) -> Result<ValueType, String> {
    let text = text.trim();
    if text == "quoted" {
        return Ok(ValueType::Quoted);
    }
    if text == "ident" {
        return Ok(ValueType::Ident);
    }
    if text == "u32" {
        return Ok(ValueType::U32);
    }
    if text == "i64" {
        return Ok(ValueType::I64);
    }
    if text == "f64" {
        return Ok(ValueType::F64);
    }
    if text == "bool" {
        return Ok(ValueType::Bool);
    }
    if text == "path" {
        return Ok(ValueType::Path);
    }
    if text == "list" {
        return Ok(ValueType::List);
    }
    if text == "loose" {
        return Ok(ValueType::Loose);
    }
    if text == "enum_or_quoted" {
        return Err("enum_or_quoted requires values in field block".into());
    }
    if let Some(inner) = text.strip_prefix("list[") {
        let inner = inner.strip_suffix(']').ok_or("unclosed list type")?;
        let parts = split_type_union(inner);
        let mut types = Vec::new();
        for part in parts {
            types.push(parse_value_type_text(part)?);
        }
        return Ok(ValueType::ListOf(types));
    }
    if let Some(inner) = text.strip_prefix("map[") {
        let inner = inner.strip_suffix(']').ok_or("unclosed map type")?;
        let (key, value) = inner
            .split_once(',')
            .ok_or("map type requires key, value pair")?;
        return Ok(ValueType::MapOf {
            key: Box::new(parse_value_type_text(key.trim())?),
            value: Box::new(parse_value_type_text(value.trim())?),
        });
    }
    if let Some(inner) = text.strip_prefix("ref(") {
        let rule = inner.strip_suffix(')').ok_or("unclosed ref type")?;
        return Ok(ValueType::RefTo(rule.to_string()));
    }
    if let Some(inner) = text.strip_prefix("inline(") {
        let rule = inner.strip_suffix(')').ok_or("unclosed inline type")?;
        return Ok(ValueType::Inline(rule.to_string()));
    }
    Err(format!("unknown field type `{text}`"))
}

fn split_type_union(text: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut start = 0usize;
    for (index, ch) in text.char_indices() {
        match ch {
            '[' | '(' => depth += 1,
            ']' | ')' => depth -= 1,
            '|' if depth == 0 => {
                parts.push(text[start..index].trim());
                start = index + 1;
            }
            _ => {}
        }
    }
    parts.push(text[start..].trim());
    parts.retain(|p| !p.is_empty());
    parts
}

pub(super) fn parse_bool(block: &BsolBlock, key: &str) -> Option<bool> {
    assignment_string(block, key)
        .ok()
        .flatten()
        .map(|v| v == "true")
}

pub(super) fn assignment_string(block: &BsolBlock, key: &str) -> Result<Option<String>, BsolError> {
    for item in &block.items {
        let BsolItem::Assignment(a) = item else {
            continue;
        };
        if a.key != key {
            continue;
        }
        return Ok(Some(value_as_string(&a.value)?));
    }
    Ok(None)
}

pub(super) fn assignment_list(block: &BsolBlock, key: &str) -> Result<Vec<String>, BsolError> {
    for item in &block.items {
        let BsolItem::Assignment(a) = item else {
            continue;
        };
        if a.key != key {
            continue;
        }
        return value_as_list(&a.value);
    }
    Ok(Vec::new())
}

pub(super) fn value_as_string(value: &BsolValue) -> Result<String, BsolError> {
    match value {
        BsolValue::QuotedString(q) => Ok(q.value.clone()),
        BsolValue::Ident(i) => Ok(i.clone()),
        BsolValue::Bool(b) => Ok(b.to_string()),
        BsolValue::Ref(r) => Ok(r.display()),
        BsolValue::BracketList(_) | BsolValue::InlineMap(_) => Err(BsolError::Schema(
            "expected string, found structured value".into(),
        )),
    }
}

fn value_as_list(value: &BsolValue) -> Result<Vec<String>, BsolError> {
    let BsolValue::BracketList(list) = value else {
        return Err(BsolError::Schema("expected list".into()));
    };
    let mut out = Vec::new();
    for item in &list.items {
        match item {
            BsolListItem::QuotedString(q) => out.push(q.value.clone()),
            BsolListItem::Ident(i) => out.push(i.clone()),
            BsolListItem::Default => out.push("default".to_string()),
            BsolListItem::Bool(b) => out.push(b.to_string()),
            BsolListItem::Ref(r) => out.push(r.display()),
            BsolListItem::InlineMap(_) | BsolListItem::InlineBlock(_) => {
                return Err(BsolError::Schema(
                    "expected flat list item, found structured value".into(),
                ));
            }
        }
    }
    Ok(out)
}
