use pest::Parser;
use pest::iterators::Pair;

use crate::error::BsolError;
use crate::parser::{BsolParser, Rule};

use super::model::{
    BsolAssignment, BsolAttribute, BsolAttributeArg, BsolBlock, BsolBracketList, BsolInlineMap,
    BsolItem, BsolListItem, BsolMapEntry, BsolQuotedString, BsolRef, BsolSpan, BsolValue,
};
use super::spans_errors::{offset_span, span_at};

pub(super) fn build_assignment(
    pair: Pair<Rule>,
    source: &str,
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
            inner,
            source,
            source_offset,
        ))),
        Rule::bare_token => Ok(BsolValue::Ident(inner.as_str().to_string())),
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

fn build_quoted_string(pair: Pair<Rule>, source: &str, source_offset: usize) -> BsolQuotedString {
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
                    inner,
                    source,
                    source_offset,
                )));
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
                        item,
                        source,
                        source_offset,
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
                    item,
                    source,
                    source_offset,
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

pub(super) fn parse_value_text(raw: &str, span: BsolSpan) -> Result<BsolValue, BsolError> {
    let trimmed = raw.trim();
    let mut pairs = BsolParser::parse(Rule::value, trimmed)
        .map_err(|err| BsolError::parse_at(span, err.to_string()))?;
    let pair = pairs
        .next()
        .ok_or_else(|| BsolError::parse_at(span, "empty attribute value"))?;
    build_value(pair, trimmed, 0)
}
