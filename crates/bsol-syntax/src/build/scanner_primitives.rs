use crate::error::BsolError;

use super::model::{BsolAttribute, BsolAttributeArg, BsolRef};
use super::pest_ast_builder::parse_value_text;
use super::spans_errors::span_at;

fn skip_horizontal_ws(source: &str, cursor: &mut usize) {
    while *cursor < source.len() {
        match source.as_bytes().get(*cursor) {
            Some(b' ' | b'\t') => *cursor += 1,
            _ => break,
        }
    }
}

pub(super) fn find_assignment_end(
    source: &str,
    start: usize,
    end: usize,
) -> Result<usize, BsolError> {
    let mut cursor = start;
    read_attribute_list(source, &mut cursor)?;
    skip_horizontal_ws(source, &mut cursor);
    if read_ident(source, &mut cursor).is_none() {
        return Err(BsolError::parse_at(
            span_at(source, start, end.max(start + 1)),
            "expected assignment key",
        ));
    }
    skip_horizontal_ws(source, &mut cursor);
    if source.as_bytes().get(cursor) != Some(&b'=') {
        return Err(BsolError::parse_at(
            span_at(source, start, end.max(start + 1)),
            "expected `=` in assignment",
        ));
    }
    cursor += 1;
    skip_horizontal_ws(source, &mut cursor);
    if cursor >= end {
        return Err(BsolError::parse_at(
            span_at(source, start, end),
            "missing assignment value",
        ));
    }
    cursor = scan_value_end(source, cursor, end)?;
    Ok(cursor.min(end))
}

fn scan_value_end(source: &str, start: usize, end: usize) -> Result<usize, BsolError> {
    let mut cursor = start;
    if source.as_bytes().get(cursor) == Some(&b'"') {
        if read_quoted_string(source, &mut cursor).is_none() {
            return Err(BsolError::parse_at(
                span_at(source, start, end),
                "unclosed string in assignment",
            ));
        }
    } else if source.as_bytes().get(cursor) == Some(&b'[') {
        let bracket_end = find_matching_close_bracket(source, cursor).ok_or_else(|| {
            BsolError::parse_at(span_at(source, start, end), "unclosed bracket list")
        })?;
        cursor = bracket_end;
    } else if source.as_bytes().get(cursor) == Some(&b'{') {
        let brace_end = find_matching_close_brace(source, cursor).ok_or_else(|| {
            BsolError::parse_at(span_at(source, start, end), "unclosed inline map")
        })?;
        cursor = brace_end;
    } else if source.as_bytes().get(cursor) == Some(&b'@') {
        read_ref_literal(source, &mut cursor).ok_or_else(|| {
            BsolError::parse_at(span_at(source, start, end), "invalid reference literal")
        })?;
    } else if read_bool_literal(source, &mut cursor).is_some() {
        // consumed
    } else if read_type_expr(source, &mut cursor).is_some() {
        // consumed parameterized type expression (ref(node), list[...], map[...])
    } else if read_bare_token(source, &mut cursor).is_none() {
        return Err(BsolError::parse_at(
            span_at(source, start, end),
            "missing assignment value",
        ));
    }
    Ok(cursor)
}

fn read_type_expr(source: &str, cursor: &mut usize) -> Option<()> {
    let start = *cursor;
    read_ident(source, cursor)?;
    skip_horizontal_ws(source, cursor);
    if source.as_bytes().get(*cursor) == Some(&b'(') {
        if find_matching_close_paren(source, *cursor).is_none() {
            *cursor = start;
            return None;
        }
        *cursor = find_matching_close_paren(source, *cursor).unwrap();
        return Some(());
    }
    if source.as_bytes().get(*cursor) == Some(&b'[') {
        if find_matching_close_bracket(source, *cursor).is_none() {
            *cursor = start;
            return None;
        }
        *cursor = find_matching_close_bracket(source, *cursor).unwrap();
        return Some(());
    }
    *cursor = start;
    None
}

fn find_matching_close_paren(source: &str, open: usize) -> Option<usize> {
    debug_assert_eq!(source.as_bytes().get(open), Some(&b'('));
    let mut depth = 0i32;
    let mut i = open;
    let mut in_string = false;
    while i < source.len() {
        let b = source.as_bytes()[i];
        match b {
            b'"' => in_string = !in_string,
            b'(' if !in_string => depth += 1,
            b')' if !in_string => {
                depth -= 1;
                if depth == 0 {
                    return Some(i + 1);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

fn read_bare_token(source: &str, cursor: &mut usize) -> Option<()> {
    let start = *cursor;
    let first = source[*cursor..].chars().next()?;
    if first.is_ascii_digit() {
        while let Some(ch) = source[*cursor..].chars().next() {
            if !ch.is_ascii_digit() {
                break;
            }
            *cursor += ch.len_utf8();
        }
    } else if first.is_ascii_alphabetic() || first == '_' {
        read_ident(source, cursor)?;
    } else {
        return None;
    }
    (*cursor > start).then_some(())
}

fn read_bool_literal(source: &str, cursor: &mut usize) -> Option<bool> {
    for (text, value) in [("true", true), ("false", false)] {
        if source[*cursor..].starts_with(text) {
            let next = *cursor + text.len();
            let continues = source[next..]
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_');
            if !continues {
                *cursor = next;
                return Some(value);
            }
        }
    }
    None
}

fn read_ref_literal(source: &str, cursor: &mut usize) -> Option<BsolRef> {
    if source.as_bytes().get(*cursor) != Some(&b'@') {
        return None;
    }
    let start = *cursor;
    *cursor += 1;
    if is_schemaless_at(source, start) {
        *cursor = start;
        return None;
    }
    let rule_kind = read_ident(source, cursor);
    skip_horizontal_ws(source, cursor);
    if source.as_bytes().get(*cursor) == Some(&b'/') {
        *cursor += 1;
        skip_horizontal_ws(source, cursor);
        let label = read_ident(source, cursor)?;
        return Some(BsolRef {
            span: span_at(source, start, *cursor),
            rule_kind,
            label,
        });
    }
    if let Some(label) = rule_kind {
        return Some(BsolRef {
            span: span_at(source, start, *cursor),
            rule_kind: None,
            label,
        });
    }
    *cursor = start;
    None
}

fn find_matching_close_bracket(source: &str, open: usize) -> Option<usize> {
    debug_assert_eq!(source.as_bytes().get(open), Some(&b'['));
    let mut depth = 0i32;
    let mut i = open;
    let mut in_string = false;
    while i < source.len() {
        let b = source.as_bytes()[i];
        match b {
            b'"' => in_string = !in_string,
            b'[' if !in_string => depth += 1,
            b']' if !in_string => {
                depth -= 1;
                if depth == 0 {
                    return Some(i + 1);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

pub(super) fn read_attribute_list(
    source: &str,
    cursor: &mut usize,
) -> Result<Vec<BsolAttribute>, BsolError> {
    let mut attrs = Vec::new();
    loop {
        skip_ws_and_comments(source, cursor);
        if source.as_bytes().get(*cursor) == Some(&b'[') {
            let start = *cursor;
            *cursor += 1;
            let name = read_ident(source, cursor).ok_or_else(|| {
                BsolError::parse_at(
                    span_at(source, start, *cursor),
                    "expected attribute name inside `[`",
                )
            })?;
            skip_horizontal_ws(source, cursor);
            let mut args = Vec::new();
            if source.as_bytes().get(*cursor) == Some(&b'(') {
                *cursor += 1;
                loop {
                    skip_horizontal_ws(source, cursor);
                    if source.as_bytes().get(*cursor) == Some(&b')') {
                        *cursor += 1;
                        break;
                    }
                    let key = read_ident(source, cursor).ok_or_else(|| {
                        BsolError::parse_at(
                            span_at(source, start, *cursor),
                            "expected attribute argument key",
                        )
                    })?;
                    skip_horizontal_ws(source, cursor);
                    if source.as_bytes().get(*cursor) != Some(&b'=') {
                        return Err(BsolError::parse_at(
                            span_at(source, start, *cursor),
                            "expected `=` in attribute argument",
                        ));
                    }
                    *cursor += 1;
                    skip_horizontal_ws(source, cursor);
                    let value_end = scan_value_end(source, *cursor, source.len())?;
                    let raw = source.get(*cursor..value_end).unwrap_or("");
                    let value = parse_value_text(raw, span_at(source, *cursor, value_end))?;
                    args.push(BsolAttributeArg { key, value });
                    *cursor = value_end;
                    skip_horizontal_ws(source, cursor);
                    if source.as_bytes().get(*cursor) == Some(&b',') {
                        *cursor += 1;
                    }
                }
            }
            if source.as_bytes().get(*cursor) != Some(&b']') {
                return Err(BsolError::parse_at(
                    span_at(source, start, *cursor),
                    "expected `]` to close attribute",
                ));
            }
            *cursor += 1;
            attrs.push(BsolAttribute {
                span: span_at(source, start, *cursor),
                name,
                args,
            });
            continue;
        }
        if source.as_bytes().get(*cursor) != Some(&b'@') || is_schemaless_at(source, *cursor) {
            break;
        }
        let start = *cursor;
        *cursor += 1;
        let name = read_ident(source, cursor).ok_or_else(|| {
            BsolError::parse_at(
                span_at(source, start, *cursor),
                "expected attribute name after `@`",
            )
        })?;
        skip_horizontal_ws(source, cursor);
        let mut args = Vec::new();
        if source.as_bytes().get(*cursor) == Some(&b'(') {
            *cursor += 1;
            loop {
                skip_horizontal_ws(source, cursor);
                if source.as_bytes().get(*cursor) == Some(&b')') {
                    *cursor += 1;
                    break;
                }
                let key = read_ident(source, cursor).ok_or_else(|| {
                    BsolError::parse_at(
                        span_at(source, start, *cursor),
                        "expected attribute argument key",
                    )
                })?;
                skip_horizontal_ws(source, cursor);
                if source.as_bytes().get(*cursor) != Some(&b'=') {
                    return Err(BsolError::parse_at(
                        span_at(source, start, *cursor),
                        "expected `=` in attribute argument",
                    ));
                }
                *cursor += 1;
                skip_horizontal_ws(source, cursor);
                let value_end = scan_value_end(source, *cursor, source.len())?;
                let raw = source.get(*cursor..value_end).unwrap_or("");
                let value = parse_value_text(raw, span_at(source, *cursor, value_end))?;
                args.push(BsolAttributeArg { key, value });
                *cursor = value_end;
                skip_horizontal_ws(source, cursor);
                if source.as_bytes().get(*cursor) == Some(&b',') {
                    *cursor += 1;
                }
            }
        }
        attrs.push(BsolAttribute {
            span: span_at(source, start, *cursor),
            name,
            args,
        });
    }
    Ok(attrs)
}

pub(super) fn read_ident(source: &str, cursor: &mut usize) -> Option<String> {
    let start = *cursor;
    let mut chars = source[*cursor..].chars();
    let first = chars.next()?;
    if !first.is_ascii_alphabetic() && first != '_' {
        return None;
    }
    *cursor += first.len_utf8();
    while let Some(ch) = source[*cursor..].chars().next() {
        if !ch.is_ascii_alphanumeric() && ch != '_' {
            break;
        }
        *cursor += ch.len_utf8();
    }
    Some(source[start..*cursor].to_string())
}

pub(super) fn read_quoted_string(source: &str, cursor: &mut usize) -> Option<String> {
    if source.as_bytes().get(*cursor) != Some(&b'"') {
        return None;
    }
    let start = *cursor;
    *cursor += 1;
    while *cursor < source.len() {
        let ch = source[*cursor..].chars().next()?;
        if ch == '"' {
            let value = source.get(start + 1..*cursor)?.to_string();
            *cursor += 1;
            return Some(value);
        }
        *cursor += ch.len_utf8();
    }
    None
}

pub(super) fn is_schemaless_at(source: &str, cursor: usize) -> bool {
    source[cursor..].starts_with("@schemaless")
        && !source[cursor + "@schemaless".len()..]
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
}

pub(super) fn read_schemaless_marker(source: &str, cursor: &mut usize) -> bool {
    if is_schemaless_at(source, *cursor) {
        *cursor += "@schemaless".len();
        return true;
    }
    false
}

pub(super) fn skip_ws_and_comments(source: &str, cursor: &mut usize) {
    while *cursor < source.len() {
        let remaining = &source[*cursor..];
        if remaining.starts_with("//") {
            if let Some(end) = remaining.find('\n') {
                *cursor += end + 1;
                continue;
            }
            *cursor = source.len();
            break;
        }
        if remaining.starts_with('#') {
            if let Some(end) = remaining.find('\n') {
                *cursor += end + 1;
                continue;
            }
            *cursor = source.len();
            break;
        }
        let Some(ch) = remaining.chars().next() else {
            break;
        };
        if ch.is_whitespace() {
            *cursor += ch.len_utf8();
            continue;
        }
        break;
    }
}

pub(super) fn find_matching_close_brace(source: &str, open_brace: usize) -> Option<usize> {
    debug_assert_eq!(source.as_bytes().get(open_brace), Some(&b'{'));
    let mut depth = 0i32;
    let mut i = open_brace;
    let mut in_string = false;
    while i < source.len() {
        let b = source.as_bytes()[i];
        match b {
            b'"' => in_string = !in_string,
            b'{' if !in_string => depth += 1,
            b'}' if !in_string => {
                depth -= 1;
                if depth == 0 {
                    return Some(i + 1);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}
