//! Build a [`BsolDocument`] from pest pairs and top-level block scanning.

use pest::Parser;
use pest::error::InputLocation;
use pest::iterators::Pair;

use crate::ast::{
    BsolAssignment, BsolAttribute, BsolAttributeArg, BsolBlock, BsolBracketList, BsolDocument,
    BsolInlineMap, BsolItem, BsolListItem, BsolMapEntry, BsolQuotedString, BsolRef, BsolSpan,
    BsolValue,
};
use crate::error::BsolError;
use crate::parser::{BsolParser, Rule};

/// Parse Bsol source into the generic document AST.
pub fn parse_bsol_document(source: &str) -> Result<BsolDocument, BsolError> {
    let mut blocks = Vec::new();
    let mut cursor = 0usize;
    while cursor < source.len() {
        skip_ws_and_comments(source, &mut cursor);
        if cursor >= source.len() {
            break;
        }
        let (block, next) = parse_block_at(source, cursor)?;
        blocks.push(block);
        cursor = next;
    }
    Ok(BsolDocument { blocks })
}

fn parse_block_at(source: &str, start: usize) -> Result<(BsolBlock, usize), BsolError> {
    let mut cursor = start;
    skip_ws_and_comments(source, &mut cursor);
    let block_start = cursor;
    let attrs = read_attribute_list(source, &mut cursor)?;
    skip_ws_and_comments(source, &mut cursor);
    let kind_start = cursor;
    let kind = read_ident(source, &mut cursor).ok_or_else(|| {
        BsolError::parse_at(
            span_at(source, start, cursor.max(start + 1)),
            "expected block kind identifier",
        )
    })?;
    skip_ws_and_comments(source, &mut cursor);

    let label = if source.as_bytes().get(cursor) == Some(&b'"') {
        let label_start = cursor;
        let value = read_quoted_string(source, &mut cursor).ok_or_else(|| {
            BsolError::parse_at(
                span_at(source, label_start, cursor.max(label_start + 1)),
                "expected quoted block label",
            )
        })?;
        skip_ws_and_comments(source, &mut cursor);
        Some(BsolQuotedString {
            span: span_at(source, label_start, cursor),
            value,
        })
    } else {
        None
    };

    let schemaless = read_schemaless_marker(source, &mut cursor);
    skip_ws_and_comments(source, &mut cursor);
    if source.as_bytes().get(cursor) != Some(&b'{') {
        return Err(BsolError::parse_at(
            span_at(source, kind_start, cursor.max(kind_start + 1)),
            "expected `{` to open block body",
        ));
    }
    let body_open = cursor;
    let body_close = find_matching_close_brace(source, body_open).ok_or_else(|| {
        BsolError::parse_at(
            span_at(source, body_open, source.len()),
            "unclosed block body",
        )
    })?;
    let body_end = body_close - 1;
    let block_end = body_close;
    let span = span_at(source, block_start, block_end);

    if schemaless {
        let raw = source.get(body_open + 1..body_end).unwrap_or("").to_string();
        return Ok((
            BsolBlock {
                span,
                attrs,
                kind,
                label,
                schemaless_body: Some(raw),
                items: Vec::new(),
            },
            block_end,
        ));
    }

    let items = parse_block_items_in_range(source, body_open + 1, body_end)?;
    Ok((
        BsolBlock {
            span,
            attrs,
            kind,
            label,
            schemaless_body: None,
            items,
        },
        block_end,
    ))
}

fn parse_block_items_in_range(
    source: &str,
    start: usize,
    end: usize,
) -> Result<Vec<BsolItem>, BsolError> {
    let mut items = Vec::new();
    let mut cursor = start;
    while cursor < end {
        skip_ws_and_comments(source, &mut cursor);
        if cursor >= end {
            break;
        }
        let item_start = cursor;
        let attrs = read_attribute_list(source, &mut cursor)?;
        skip_ws_and_comments(source, &mut cursor);
        if source.as_bytes().get(cursor) == Some(&b'@') && !is_schemaless_at(source, cursor) {
            // attribute-only prefix already consumed; continue
        }
        let Some(_kind) = read_ident(source, &mut cursor) else {
            break;
        };
        skip_ws_and_comments(source, &mut cursor);
        if source.as_bytes().get(cursor) == Some(&b'=') {
            let assign_end = find_assignment_end(source, item_start, end)?;
            items.push(parse_assignment_slice(source, item_start, assign_end, attrs)?);
            cursor = assign_end;
            continue;
        }
        let (block, next) = parse_block_at(source, item_start)?;
        if next > end {
            return Err(BsolError::parse_at(
                span_at(source, item_start, end),
                "nested block extends past enclosing body",
            ));
        }
        items.push(BsolItem::Block(block));
        cursor = next;
    }
    Ok(items)
}

fn skip_horizontal_ws(source: &str, cursor: &mut usize) {
    while *cursor < source.len() {
        match source.as_bytes().get(*cursor) {
            Some(b' ' | b'\t') => *cursor += 1,
            _ => break,
        }
    }
}

fn find_assignment_end(source: &str, start: usize, end: usize) -> Result<usize, BsolError> {
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
    } else     if first.is_ascii_alphabetic() || first == '_' {
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

fn parse_assignment_slice(
    source: &str,
    item_start: usize,
    item_end: usize,
    attrs: Vec<BsolAttribute>,
) -> Result<BsolItem, BsolError> {
    let raw = source.get(item_start..item_end).unwrap_or("");
    let trim_offset = raw.len().saturating_sub(raw.trim_start().len());
    let base = item_start + trim_offset;
    let trimmed = raw.trim();
    let mut pairs = BsolParser::parse(Rule::assignment, trimmed)
        .map_err(|err| pest_error_with_offset(source, base, err))?;
    let pair = pairs
        .next()
        .ok_or_else(|| BsolError::Parse("Bsol parse produced no assignment pair".to_string()))?;
    Ok(BsolItem::Assignment(build_assignment(
        pair, source, base, attrs,
    )?))
}

fn build_assignment(
    pair: Pair<Rule>,
    source: &str,
    base: usize,
    outer_attrs: Vec<BsolAttribute>,
) -> Result<BsolAssignment, BsolError> {
    let span = span_at(source, base + pair.as_span().start(), base + pair.as_span().end());
    let mut inner = pair.into_inner();
    let mut attrs = outer_attrs;
    if let Some(next) = inner.peek() {
        if next.as_rule() == Rule::attribute_list {
            attrs.extend(build_attribute_list(inner.next().unwrap(), source, base)?);
        }
    }
    let key_pair = inner
        .next()
        .ok_or_else(|| BsolError::parse_at(span, "missing assignment key"))?;
    let key = key_pair.as_str().to_string();
    let value_pair = inner
        .next()
        .ok_or_else(|| BsolError::parse_at(span, "missing assignment value"))?;
    let value = build_value(value_pair, source, base)?;
    Ok(BsolAssignment {
        span,
        attrs,
        key,
        value,
    })
}

fn build_attribute_list(
    pair: Pair<Rule>,
    source: &str,
    source_offset: usize,
) -> Result<Vec<BsolAttribute>, BsolError> {
    let mut attrs = Vec::new();
    for child in pair.into_inner() {
        if child.as_rule() == Rule::attribute {
            attrs.push(build_attribute(child, source, source_offset)?);
        }
    }
    Ok(attrs)
}

fn build_attribute(
    pair: Pair<Rule>,
    source: &str,
    source_offset: usize,
) -> Result<BsolAttribute, BsolError> {
    let span = offset_span(source, source_offset, pair.as_span());
    let mut inner = pair.into_inner();
    let name = inner
        .next()
        .ok_or_else(|| BsolError::parse_at(span, "missing attribute name"))?
        .as_str()
        .to_string();
    let mut args = Vec::new();
    if let Some(args_pair) = inner.next() {
        for arg_pair in args_pair.into_inner() {
            if arg_pair.as_rule() == Rule::attribute_arg {
                let mut arg_inner = arg_pair.into_inner();
                let key = arg_inner
                    .next()
                    .ok_or_else(|| BsolError::parse_at(span, "missing attribute arg key"))?
                    .as_str()
                    .to_string();
                let value_pair = arg_inner
                    .next()
                    .ok_or_else(|| BsolError::parse_at(span, "missing attribute arg value"))?;
                let value = build_value(value_pair, source, source_offset)?;
                args.push(BsolAttributeArg { key, value });
            }
        }
    }
    Ok(BsolAttribute { span, name, args })
}

fn build_value(
    pair: Pair<Rule>,
    source: &str,
    source_offset: usize,
) -> Result<BsolValue, BsolError> {
    let inner = pair
        .into_inner()
        .next()
        .ok_or_else(|| BsolError::Parse("empty Bsol value".to_string()))?;
    match inner.as_rule() {
        Rule::quoted_string => Ok(BsolValue::QuotedString(build_quoted_string(
            inner, source, source_offset,
        ))),
        Rule::bare_token => Ok(BsolValue::Ident(inner.as_str().to_string())),
        Rule::bool_literal => Ok(BsolValue::Bool(inner.as_str() == "true")),
        Rule::bracket_list => Ok(BsolValue::BracketList(build_bracket_list(
            inner, source, source_offset,
        )?)),
        Rule::inline_map => Ok(BsolValue::InlineMap(build_inline_map(
            inner, source, source_offset,
        )?)),
        Rule::ref_literal => Ok(BsolValue::Ref(build_ref_from_pair(
            inner, source, source_offset,
        )?)),
        other => Err(BsolError::parse_at(
            offset_span(source, source_offset, inner.as_span()),
            format!("unexpected value rule `{other:?}`"),
        )),
    }
}

fn build_ref_from_pair(
    pair: Pair<Rule>,
    source: &str,
    source_offset: usize,
) -> Result<BsolRef, BsolError> {
    let span = offset_span(source, source_offset, pair.as_span());
    let path = pair
        .into_inner()
        .next()
        .ok_or_else(|| BsolError::parse_at(span, "empty reference"))?;
    let mut parts = path.into_inner();
    let first = parts
        .next()
        .ok_or_else(|| BsolError::parse_at(span, "empty reference path"))?;
    if first.as_rule() == Rule::ident {
        if let Some(second) = parts.next() {
            return Ok(BsolRef {
                span,
                rule_kind: Some(first.as_str().to_string()),
                label: second.as_str().to_string(),
            });
        }
        return Ok(BsolRef {
            span,
            rule_kind: None,
            label: first.as_str().to_string(),
        });
    }
    Err(BsolError::parse_at(span, "invalid reference path"))
}

fn build_inline_map(
    pair: Pair<Rule>,
    source: &str,
    source_offset: usize,
) -> Result<BsolInlineMap, BsolError> {
    let span = offset_span(source, source_offset, pair.as_span());
    let mut entries = Vec::new();
    for child in pair.into_inner() {
        if child.as_rule() == Rule::map_entry {
            let entry_span = offset_span(source, source_offset, child.as_span());
            let mut inner = child.into_inner();
            let key = inner
                .next()
                .ok_or_else(|| BsolError::parse_at(entry_span, "missing map key"))?
                .as_str()
                .to_string();
            let value_pair = inner
                .next()
                .ok_or_else(|| BsolError::parse_at(entry_span, "missing map value"))?;
            let value = build_value(value_pair, source, source_offset)?;
            entries.push(BsolMapEntry {
                span: entry_span,
                key,
                value,
            });
        }
    }
    Ok(BsolInlineMap { span, entries })
}

fn build_quoted_string(
    pair: Pair<Rule>,
    source: &str,
    source_offset: usize,
) -> BsolQuotedString {
    let span = offset_span(source, source_offset, pair.as_span());
    BsolQuotedString::new(span, pair.as_str())
}

fn build_bracket_list(
    pair: Pair<Rule>,
    source: &str,
    source_offset: usize,
) -> Result<BsolBracketList, BsolError> {
    let span = offset_span(source, source_offset, pair.as_span());
    let mut items = Vec::new();
    for child in pair.into_inner() {
        match child.as_rule() {
            Rule::list_content => {
                for item in child.into_inner() {
                    items.push(build_list_item(item, source, source_offset)?);
                }
            }
            Rule::list_item => items.push(build_list_item(child, source, source_offset)?),
            _ => {}
        }
    }
    Ok(BsolBracketList { span, items })
}

fn build_list_item(
    pair: Pair<Rule>,
    source: &str,
    source_offset: usize,
) -> Result<BsolListItem, BsolError> {
    let span = offset_span(source, source_offset, pair.as_span());
    let text = pair.as_str().trim();
    if text == "default" {
        return Ok(BsolListItem::Default);
    }
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::quoted_string => {
                return Ok(BsolListItem::QuotedString(build_quoted_string(
                    inner, source, source_offset,
                )));
            }
            Rule::ident => return Ok(BsolListItem::Ident(inner.as_str().to_string())),
            Rule::bool_literal => {
                return Ok(BsolListItem::Bool(inner.as_str() == "true"));
            }
            Rule::ref_literal => {
                return Ok(BsolListItem::Ref(build_ref_from_pair(
                    inner, source, source_offset,
                )?));
            }
            Rule::inline_map => {
                return Ok(BsolListItem::InlineMap(build_inline_map(
                    inner, source, source_offset,
                )?));
            }
            Rule::inline_block => {
                return Ok(BsolListItem::InlineBlock(build_inline_block(
                    inner, source, source_offset,
                )?));
            }
            _ => {}
        }
    }
    Err(BsolError::parse_at(
        span,
        format!("unexpected list item `{text}`"),
    ))
}

fn build_inline_block(
    pair: Pair<Rule>,
    source: &str,
    source_offset: usize,
) -> Result<BsolBlock, BsolError> {
    let span = offset_span(source, source_offset, pair.as_span());
    let raw = pair.as_str();
    let mut inner = pair.into_inner();
    let kind = inner
        .find(|p| p.as_rule() == Rule::block_kind)
        .map(|p| p.as_str().to_string())
        .or_else(|| {
            raw.split('{')
                .next()
                .map(|s| s.split_whitespace().next().unwrap_or("").to_string())
                .filter(|s| !s.is_empty())
        })
        .ok_or_else(|| BsolError::parse_at(span, "missing inline block kind"))?;
    let label = inner.find(|p| p.as_rule() == Rule::quoted_string).map(|p| {
        BsolQuotedString::new(offset_span(source, source_offset, p.as_span()), p.as_str())
    });
    let mut items = Vec::new();
    if let Some(body) = inner.find(|p| p.as_rule() == Rule::block_body) {
        for item in body.into_inner() {
            match item.as_rule() {
                Rule::assignment => {
                    items.push(BsolItem::Assignment(build_assignment(
                        item,
                        source,
                        source_offset,
                        Vec::new(),
                    )?));
                }
                Rule::block => {
                    items.push(BsolItem::Block(build_inline_block_as_block(
                        item, source, source_offset,
                    )?));
                }
                _ => {}
            }
        }
    } else if let (Some(open), Some(close)) = (raw.find('{'), raw.rfind('}')) {
        let body_text = &raw[open + 1..close];
        if let Ok(mut parsed) = BsolParser::parse(Rule::block_body, body_text) {
            if let Some(body_pair) = parsed.next() {
                for item in body_pair.into_inner() {
                    if item.as_rule() == Rule::assignment {
                        items.push(BsolItem::Assignment(build_assignment(
                            item,
                            source,
                            source_offset + open + 1,
                            Vec::new(),
                        )?));
                    }
                }
            }
        }
    } else {
        return Err(BsolError::parse_at(span, "missing inline block body"));
    }
    Ok(BsolBlock {
        span,
        attrs: Vec::new(),
        kind,
        label,
        schemaless_body: None,
        items,
    })
}

fn build_inline_block_as_block(
    pair: Pair<Rule>,
    source: &str,
    source_offset: usize,
) -> Result<BsolBlock, BsolError> {
    let span = offset_span(source, source_offset, pair.as_span());
    let mut inner = pair.into_inner();
    let mut attrs = Vec::new();
    if let Some(next) = inner.peek() {
        if next.as_rule() == Rule::attribute_list {
            attrs = build_attribute_list(inner.next().unwrap(), source, source_offset)?;
        }
    }
    let kind = inner
        .next()
        .ok_or_else(|| BsolError::parse_at(span, "missing block kind"))?
        .as_str()
        .to_string();
    let label = inner.find(|p| p.as_rule() == Rule::quoted_string).map(|p| {
        BsolQuotedString::new(offset_span(source, source_offset, p.as_span()), p.as_str())
    });
    let body = inner
        .find(|p| p.as_rule() == Rule::block_body)
        .ok_or_else(|| BsolError::parse_at(span, "missing block body"))?;
    let mut items = Vec::new();
    for item in body.into_inner() {
        match item.as_rule() {
            Rule::assignment => {
                items.push(BsolItem::Assignment(build_assignment(
                    item,
                    source,
                    source_offset,
                    Vec::new(),
                )?));
            }
            Rule::block => {
                items.push(BsolItem::Block(build_inline_block_as_block(
                    item, source, source_offset,
                )?));
            }
            _ => {}
        }
    }
    Ok(BsolBlock {
        span,
        attrs,
        kind,
        label,
        schemaless_body: None,
        items,
    })
}

fn read_attribute_list(source: &str, cursor: &mut usize) -> Result<Vec<BsolAttribute>, BsolError> {
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

fn parse_value_text(raw: &str, span: BsolSpan) -> Result<BsolValue, BsolError> {
    let trimmed = raw.trim();
    let mut pairs = BsolParser::parse(Rule::value, trimmed)
        .map_err(|err| BsolError::parse_at(span, err.to_string()))?;
    let pair = pairs
        .next()
        .ok_or_else(|| BsolError::parse_at(span, "empty attribute value"))?;
    build_value(pair, trimmed, 0)
}

fn read_ident(source: &str, cursor: &mut usize) -> Option<String> {
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

fn read_quoted_string(source: &str, cursor: &mut usize) -> Option<String> {
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

fn is_schemaless_at(source: &str, cursor: usize) -> bool {
    source[cursor..].starts_with("@schemaless")
        && !source[cursor + "@schemaless".len()..]
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn read_schemaless_marker(source: &str, cursor: &mut usize) -> bool {
    if is_schemaless_at(source, *cursor) {
        *cursor += "@schemaless".len();
        return true;
    }
    false
}

fn skip_ws_and_comments(source: &str, cursor: &mut usize) {
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

fn find_matching_close_brace(source: &str, open_brace: usize) -> Option<usize> {
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

fn span_at(source: &str, start: usize, end: usize) -> BsolSpan {
    let start = start.min(source.len());
    let end = end.max(start.saturating_add(1)).min(source.len());
    let pest_span = pest::Span::new(source, start, end)
        .or_else(|| pest::Span::new(source, start, start.saturating_add(1).min(source.len())))
        .expect("span bounds");
    BsolSpan::from_pest(pest_span, source)
}

fn offset_span(source: &str, offset: usize, span: pest::Span<'_>) -> BsolSpan {
    span_at(source, offset + span.start(), offset + span.end())
}

fn pest_error_with_offset(
    source: &str,
    offset: usize,
    err: pest::error::Error<Rule>,
) -> BsolError {
    let start = match err.location {
        InputLocation::Pos(pos) => offset + pos,
        InputLocation::Span((start, _)) => offset + start,
    };
    let line = source[..start.min(source.len())].lines().count().max(1);
    BsolError::ParseAt {
        line,
        message: err.to_string(),
        start: Some(start),
        end: source.get(start..).map(|tail| start + tail.len().min(1)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_nested_blocks() {
        let src = r#"p {
  name = "p"
  mod {
    maxGeneratorRounds = 4
  }
}
target "t" {
  kind = Lib
}
"#;
        let doc = parse_bsol_document(src).expect("parse");
        assert_eq!(doc.blocks.len(), 2);
        assert_eq!(doc.blocks[0].kind, "p");
        assert!(matches!(doc.blocks[0].items[1], BsolItem::Block(_)));
    }

    #[test]
    fn parse_schemaless_block_captures_raw_body() {
        let src = r#"raw @schemaless {
  this is not = valid bsol { but it stays }
  nested { braces } ok
}
"#;
        let doc = parse_bsol_document(src).expect("parse");
        assert_eq!(doc.blocks.len(), 1);
        let block = &doc.blocks[0];
        assert_eq!(block.kind, "raw");
        let body = block.schemaless_body.as_ref().expect("schemaless body");
        assert!(body.contains("this is not = valid bsol"));
        assert!(body.contains("nested { braces }"));
        assert!(block.items.is_empty());
    }

    #[test]
    fn parse_v2_values() {
        let src = r#"demo {
  enabled = true
  root = @node/main
  env = { DEBUG = "1", PORT = 8080 }
  tags = [a, @node/x, node { kind = panel }]
}
"#;
        let doc = parse_bsol_document(src).expect("parse");
        let block = &doc.blocks[0];
        let assign = |key: &str| {
            block
                .items
                .iter()
                .find_map(|item| match item {
                    BsolItem::Assignment(a) if a.key == key => Some(a),
                    _ => None,
                })
                .expect(key)
        };
        assert!(matches!(assign("enabled").value, BsolValue::Bool(true)));
        assert!(matches!(assign("root").value, BsolValue::Ref(_)));
        assert!(matches!(assign("env").value, BsolValue::InlineMap(_)));
        assert!(matches!(assign("tags").value, BsolValue::BracketList(_)));
    }

    #[test]
    fn parse_attributes() {
        let src = r#"[Deprecated(since = "2.0")]
demo {
  name = "demo"
}
"#;
        let doc = parse_bsol_document(src).expect("parse");
        assert_eq!(doc.blocks[0].attrs[0].name, "Deprecated");
    }
}
