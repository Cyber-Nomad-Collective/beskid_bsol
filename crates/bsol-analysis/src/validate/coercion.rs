use bsol_syntax::{BsolAssignment, BsolBlock, BsolInlineMap, BsolListItem, BsolRef, BsolValue};

use crate::BsolError;

pub(super) fn require_quoted(assignment: &BsolAssignment) -> Result<String, BsolError> {
    match &assignment.value {
        BsolValue::QuotedString(q) => Ok(q.value.clone()),
        other => Err(BsolError::schema_at(
            assignment.span,
            format!("expected quoted string, found `{}`", other.preview()),
        )),
    }
}

pub(super) fn require_ident(assignment: &BsolAssignment) -> Result<String, BsolError> {
    match &assignment.value {
        BsolValue::Ident(i) => Ok(i.clone()),
        BsolValue::QuotedString(q) => Ok(q.value.clone()),
        other => Err(BsolError::schema_at(
            assignment.span,
            format!("expected identifier, found `{}`", other.preview()),
        )),
    }
}

pub(super) fn require_u32(assignment: &BsolAssignment) -> Result<String, BsolError> {
    match &assignment.value {
        BsolValue::Ident(text) if text.chars().all(|c| c.is_ascii_digit()) => Ok(text.clone()),
        other => Err(BsolError::schema_at(
            assignment.span,
            format!("expected positive integer, found `{}`", other.preview()),
        )),
    }
}

pub(super) fn require_i64(assignment: &BsolAssignment) -> Result<String, BsolError> {
    match &assignment.value {
        BsolValue::Ident(text) if text.parse::<i64>().is_ok() => Ok(text.clone()),
        other => Err(BsolError::schema_at(
            assignment.span,
            format!("expected integer, found `{}`", other.preview()),
        )),
    }
}

pub(super) fn require_f64(assignment: &BsolAssignment) -> Result<String, BsolError> {
    match &assignment.value {
        BsolValue::Ident(text) if text.parse::<f64>().is_ok() => Ok(text.clone()),
        other => Err(BsolError::schema_at(
            assignment.span,
            format!("expected float, found `{}`", other.preview()),
        )),
    }
}

pub(super) fn require_bool(assignment: &BsolAssignment) -> Result<bool, BsolError> {
    match &assignment.value {
        BsolValue::Bool(b) => Ok(*b),
        BsolValue::Ident(i) if i == "true" || i == "false" => Ok(i == "true"),
        other => Err(BsolError::schema_at(
            assignment.span,
            format!("expected bool, found `{}`", other.preview()),
        )),
    }
}

pub(super) fn loose_string(assignment: &BsolAssignment) -> Result<String, BsolError> {
    match &assignment.value {
        BsolValue::QuotedString(q) => Ok(q.value.clone()),
        BsolValue::Ident(i) => Ok(i.clone()),
        BsolValue::Bool(b) => Ok(b.to_string()),
        other => Err(BsolError::schema_at(
            assignment.span,
            format!("expected string or identifier, found `{}`", other.preview()),
        )),
    }
}

pub(super) fn require_ref(assignment: &BsolAssignment) -> Result<BsolRef, BsolError> {
    match &assignment.value {
        BsolValue::Ref(r) => Ok(r.clone()),
        other => Err(BsolError::schema_at(
            assignment.span,
            format!("expected reference, found `{}`", other.preview()),
        )),
    }
}

pub(super) fn require_inline_map(assignment: &BsolAssignment) -> Result<BsolInlineMap, BsolError> {
    match &assignment.value {
        BsolValue::InlineMap(map) => Ok(map.clone()),
        other => Err(BsolError::schema_at(
            assignment.span,
            format!("expected inline map, found `{}`", other.preview()),
        )),
    }
}

pub(super) fn require_inline_block(assignment: &BsolAssignment) -> Result<BsolBlock, BsolError> {
    match &assignment.value {
        BsolValue::BracketList(list) => {
            if let Some(BsolListItem::InlineBlock(block)) = list.items.first() {
                return Ok(block.clone());
            }
            Err(BsolError::schema_at(
                assignment.span,
                "expected inline block",
            ))
        }
        other => Err(BsolError::schema_at(
            assignment.span,
            format!("expected inline block, found `{}`", other.preview()),
        )),
    }
}

pub(super) fn require_list_strings(assignment: &BsolAssignment) -> Result<Vec<String>, BsolError> {
    let BsolValue::BracketList(list) = &assignment.value else {
        return Err(BsolError::schema_at(
            assignment.span,
            "expected bracket list",
        ));
    };
    let mut out = Vec::new();
    for item in &list.items {
        let token = match item {
            BsolListItem::Default => "default".to_string(),
            BsolListItem::QuotedString(q) => q.value.clone(),
            BsolListItem::Ident(i) => i.clone(),
            BsolListItem::Bool(b) => b.to_string(),
            BsolListItem::Ref(r) => r.display(),
            BsolListItem::InlineMap(_) | BsolListItem::InlineBlock(_) => {
                return Err(BsolError::schema_at(
                    assignment.span,
                    "expected flat list item",
                ));
            }
        };
        out.push(token);
    }
    Ok(out)
}

pub(super) fn require_list_items(
    assignment: &BsolAssignment,
) -> Result<Vec<BsolListItem>, BsolError> {
    let BsolValue::BracketList(list) = &assignment.value else {
        return Err(BsolError::schema_at(
            assignment.span,
            "expected bracket list",
        ));
    };
    Ok(list.items.clone())
}

pub(super) fn enum_or_quoted(
    assignment: &BsolAssignment,
    allowed: &[String],
) -> Result<String, BsolError> {
    let value = match &assignment.value {
        BsolValue::QuotedString(q) => q.value.clone(),
        BsolValue::Ident(i) => i.clone(),
        BsolValue::Bool(b) => b.to_string(),
        other => {
            return Err(BsolError::schema_at(
                assignment.span,
                format!(
                    "expected quoted string or enum literal, found `{}`",
                    other.preview()
                ),
            ));
        }
    };
    if allowed.is_empty() || allowed.iter().any(|v| v == &value) {
        Ok(value)
    } else {
        Err(BsolError::schema_at(
            assignment.span,
            format!("unsupported enum value `{value}`"),
        ))
    }
}
