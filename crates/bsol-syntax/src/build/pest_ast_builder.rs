use super::parse_source::ParseSource;
use pest::iterators::Pair;

use crate::error::BsolError;
use crate::parser::Rule;

use super::model::{
    BsolAssignment, BsolAttribute, BsolAttributeArg, BsolBlock, BsolBracketList, BsolInlineMap,
    BsolItem, BsolListItem, BsolMapEntry, BsolNumber, BsolQuotedString, BsolRef, BsolValue,
};
use super::spans_errors::{offset_span, span_at};

pub(super) fn build_assignment(
    pair: Pair<Rule>,
    source: &ParseSource<'_>,
    base: usize,
    outer_attrs: Vec<BsolAttribute>,
) -> Result<BsolAssignment, BsolError> {
    let span = span_at(
        source,
        base + pair.as_span().start(),
        base + pair.as_span().end(),
    );
    let mut inner = pair.into_inner();
    let mut attrs = outer_attrs;
    if let Some(next) = inner.peek() {
        if next.as_rule() == Rule::attribute_list {
            attrs = build_attribute_list(inner.next().unwrap(), source, base)?;
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

pub(super) fn build_attribute_list(
    pair: Pair<Rule>,
    source: &ParseSource<'_>,
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
    source: &ParseSource<'_>,
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
    source: &ParseSource<'_>,
    source_offset: usize,
) -> Result<BsolValue, BsolError> {
    let inner = pair
        .into_inner()
        .next()
        .ok_or_else(|| BsolError::Parse("empty Bsol value".to_string()))?;
    match inner.as_rule() {
        Rule::quoted_string => Ok(BsolValue::QuotedString(build_quoted_string(
            inner,
            source,
            source_offset,
        )?)),
        Rule::bare_token => {
            let text = inner.as_str();
            if matches!(text, "NaN" | "Infinity") {
                return Err(BsolError::parse_at(
                    offset_span(source, source_offset, inner.as_span()),
                    "nonfinite number",
                ));
            }
            Ok(BsolValue::Ident(text.to_string()))
        }
        Rule::number => Ok(BsolValue::Number(build_number(
            inner,
            source,
            source_offset,
        )?)),
        Rule::inline_block => Ok(BsolValue::InlineBlock(build_inline_block(
            inner,
            source,
            source_offset,
        )?)),
        Rule::bool_literal => Ok(BsolValue::Bool(inner.as_str() == "true")),
        Rule::bracket_list => Ok(BsolValue::BracketList(build_bracket_list(
            inner,
            source,
            source_offset,
        )?)),
        Rule::inline_map => Ok(BsolValue::InlineMap(build_inline_map(
            inner,
            source,
            source_offset,
        )?)),
        Rule::ref_literal => Ok(BsolValue::Ref(build_ref_from_pair(
            inner,
            source,
            source_offset,
        )?)),
        other => Err(BsolError::parse_at(
            offset_span(source, source_offset, inner.as_span()),
            format!("unexpected value rule `{other:?}`"),
        )),
    }
}

fn build_ref_from_pair(
    pair: Pair<Rule>,
    source: &ParseSource<'_>,
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
    if matches!(first.as_rule(), Rule::ident | Rule::qualified_ident) {
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
    source: &ParseSource<'_>,
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

fn build_number(
    pair: Pair<Rule>,
    source: &ParseSource<'_>,
    offset: usize,
) -> Result<BsolNumber, BsolError> {
    let raw = pair.as_str();
    let span = offset_span(source, offset, pair.as_span());
    if raw.contains(['.', 'e', 'E']) && !raw.parse::<f64>().is_ok_and(f64::is_finite) {
        return Err(BsolError::parse_at(span, "nonfinite decimal"));
    }
    Ok(BsolNumber {
        span,
        raw: raw.to_string(),
    })
}

fn build_quoted_string(
    pair: Pair<Rule>,
    source: &ParseSource<'_>,
    source_offset: usize,
) -> Result<BsolQuotedString, BsolError> {
    let span = offset_span(source, source_offset, pair.as_span());
    BsolQuotedString::new(span, pair.as_str())
}

fn build_bracket_list(
    pair: Pair<Rule>,
    source: &ParseSource<'_>,
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
    source: &ParseSource<'_>,
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
                    inner,
                    source,
                    source_offset,
                )?));
            }
            Rule::number => {
                return Ok(BsolListItem::Number(build_number(
                    inner,
                    source,
                    source_offset,
                )?));
            }
            Rule::ident => return Ok(BsolListItem::Ident(inner.as_str().to_string())),
            Rule::bool_literal => {
                return Ok(BsolListItem::Bool(inner.as_str() == "true"));
            }
            Rule::ref_literal => {
                return Ok(BsolListItem::Ref(build_ref_from_pair(
                    inner,
                    source,
                    source_offset,
                )?));
            }
            Rule::inline_map => {
                return Ok(BsolListItem::InlineMap(build_inline_map(
                    inner,
                    source,
                    source_offset,
                )?));
            }
            Rule::inline_block => {
                return Ok(BsolListItem::InlineBlock(build_inline_block(
                    inner,
                    source,
                    source_offset,
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
    source: &ParseSource<'_>,
    source_offset: usize,
) -> Result<BsolBlock, BsolError> {
    let span = offset_span(source, source_offset, pair.as_span());
    let mut inner = pair.into_inner();
    let kind = inner
        .next()
        .ok_or_else(|| BsolError::parse_at(span, "missing inline block kind"))?
        .as_str()
        .to_string();
    let label = if inner
        .peek()
        .is_some_and(|p| p.as_rule() == Rule::quoted_string)
    {
        Some(build_quoted_string(
            inner.next().unwrap(),
            source,
            source_offset,
        )?)
    } else {
        None
    };
    let body = inner
        .next()
        .ok_or_else(|| BsolError::parse_at(span, "missing inline block body"))?;
    let mut items = Vec::new();
    for item in body.into_inner() {
        let item = if item.as_rule() == Rule::block_item {
            item.into_inner().next().unwrap()
        } else {
            item
        };
        match item.as_rule() {
            Rule::assignment => items.push(BsolItem::Assignment(build_assignment(
                item,
                source,
                source_offset,
                Vec::new(),
            )?)),
            Rule::block => items.push(BsolItem::Block(build_inline_block_as_block(
                item,
                source,
                source_offset,
            )?)),
            _ => return Err(BsolError::parse_at(span, "unexpected inline block item")),
        }
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
    source: &ParseSource<'_>,
    source_offset: usize,
) -> Result<BsolBlock, BsolError> {
    let (block, _) =
        super::block_parser::parse_block_at(source, source_offset + pair.as_span().start())?;
    Ok(block)
}
