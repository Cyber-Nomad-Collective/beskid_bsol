use super::parse_source::ParseSource;
use crate::error::BsolError;

use super::model::BsolAttribute;
use super::spans_errors::span_at;

fn skip_horizontal_ws(source: &ParseSource<'_>, cursor: &mut usize) {
    while *cursor < source.len() {
        match source.as_bytes().get(*cursor) {
            Some(b' ' | b'\t') => *cursor += 1,
            _ => break,
        }
    }
}

pub(super) fn find_assignment_end(
    source: &ParseSource<'_>,
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

fn scan_value_end(source: &ParseSource<'_>, start: usize, end: usize) -> Result<usize, BsolError> {
    use pest::Parser;
    let mut pairs = crate::BsolParser::parse(crate::Rule::value, &source[start..end])
        .map_err(|error| super::spans_errors::pest_error_with_offset(source, start, error))?;
    let pair = pairs
        .next()
        .ok_or_else(|| BsolError::parse_at(span_at(source, start, end), "missing value"))?;
    Ok(start + pair.as_span().end())
}

pub(super) fn read_attribute_list(
    source: &ParseSource<'_>,
    cursor: &mut usize,
) -> Result<Vec<BsolAttribute>, BsolError> {
    use pest::Parser;
    skip_ws_and_comments(source, cursor);
    if source.as_bytes().get(*cursor) != Some(&b'@') || is_schemaless_at(source, *cursor) {
        return Ok(Vec::new());
    }
    let start = *cursor;
    let mut pairs = crate::BsolParser::parse(crate::Rule::attribute_list, &source[start..])
        .map_err(|error| super::spans_errors::pest_error_with_offset(source, start, error))?;
    let pair = pairs.next().ok_or_else(|| {
        BsolError::parse_at(span_at(source, start, source.len()), "missing attribute")
    })?;
    *cursor = start + pair.as_span().end();
    super::pest_ast_builder::build_attribute_list(pair, source, start)
}

pub(super) fn read_ident(source: &ParseSource<'_>, cursor: &mut usize) -> Option<String> {
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

fn quoted_end(source: &ParseSource<'_>, start: usize) -> Option<usize> {
    if source.as_bytes().get(start) != Some(&b'"') {
        return None;
    }
    let mut cursor = start + 1;
    while cursor < source.len() {
        match source.as_bytes()[cursor] {
            b'\\' => {
                cursor += 1;
                let ch = source.get(cursor..)?.chars().next()?;
                cursor += ch.len_utf8();
            }
            b'"' => return Some(cursor + 1),
            _ => {
                cursor += source[cursor..].chars().next()?.len_utf8();
            }
        }
    }
    None
}

pub(super) fn read_quoted_string(source: &ParseSource<'_>, cursor: &mut usize) -> Option<String> {
    let start = *cursor;
    *cursor = quoted_end(source, start)?;
    crate::BsolQuotedString::new(span_at(source, start, *cursor), &source[start..*cursor])
        .ok()
        .map(|q| q.value)
}

pub(super) fn is_schemaless_at(source: &ParseSource<'_>, cursor: usize) -> bool {
    source[cursor..].starts_with("@schemaless")
        && !source[cursor + "@schemaless".len()..]
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
}

pub(super) fn read_schemaless_marker(source: &ParseSource<'_>, cursor: &mut usize) -> bool {
    if is_schemaless_at(source, *cursor) {
        *cursor += "@schemaless".len();
        return true;
    }
    false
}

pub(super) fn skip_ws_and_comments(source: &ParseSource<'_>, cursor: &mut usize) {
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

pub(super) fn find_matching_close_brace(
    source: &ParseSource<'_>,
    open_brace: usize,
) -> Option<usize> {
    let mut depth = 0usize;
    let mut i = open_brace;
    while i < source.len() {
        match source.as_bytes()[i] {
            b'"' => {
                i = quoted_end(source, i)?;
                continue;
            }
            b'#' => {
                skip_ws_and_comments(source, &mut i);
                continue;
            }
            b'/' if source[i..].starts_with("//") => {
                skip_ws_and_comments(source, &mut i);
                continue;
            }
            b'{' => depth += 1,
            b'}' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(i + 1);
                }
            }
            _ => {}
        }
        i += source[i..].chars().next()?.len_utf8();
    }
    None
}
