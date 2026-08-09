use pest::Parser;

use crate::error::BsolError;
use crate::parser::{BsolParser, Rule};

use super::model::{BsolAttribute, BsolBlock, BsolDocument, BsolItem, BsolQuotedString};
use super::pest_ast_builder::build_assignment;
use super::scanner_primitives::{
    find_assignment_end, find_matching_close_brace, read_attribute_list, read_ident,
    read_quoted_string, read_schemaless_marker, skip_ws_and_comments,
};
use super::spans_errors::{pest_error_with_offset, span_at};

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
        let raw = source
            .get(body_open + 1..body_end)
            .unwrap_or("")
            .to_string();
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
            items.push(parse_assignment_slice(
                source, item_start, assign_end, attrs,
            )?);
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
