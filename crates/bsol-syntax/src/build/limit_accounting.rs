//! Syntax budget accounting with the native `Core.Bsol.Parser` rules.
//!
//! Native counts one node per statement, assignment, attribute and value, plus one trivia entry per
//! comment, against `maxNodes`; it enters depth 1 for a root statement and one level per nested
//! statement, assignment and value; and it bounds every identifier, number, type token, reference
//! qualifier and decoded string by `maxScalarBytes`. The first violation in source order is the
//! reported diagnostic.

use super::block_parser::parse_block_items_in_range;
use super::model::{
    BsolAttribute, BsolBlock, BsolBracketList, BsolDocument, BsolInlineMap, BsolItem, BsolListItem,
    BsolNumber, BsolQuotedString, BsolRef, BsolValue,
};
use super::parse_source::ParseSource;
use crate::error::BsolError;
use crate::limits::{BsolLimitKind, BsolLimits};

const NESTING: &str = "BSOL nesting limit";
const IDENTIFIER: &str = "identifier byte limit";

/// Input budget, descriptor validity and a nesting pre-scan that bounds parser recursion.
pub(super) fn check_before_parse(
    source: &ParseSource<'_>,
    limits: &BsolLimits,
) -> Result<(), BsolError> {
    if source.len() > limits.max_input_bytes {
        return Err(limit_error(
            source,
            0,
            BsolLimitKind::InputLimit,
            "input byte limit",
        ));
    }
    if !limits.is_valid() {
        return Err(limit_error(
            source,
            0,
            BsolLimitKind::InvalidSyntax,
            "invalid parser limits",
        ));
    }
    // Every structural `{`, `[` or `(` adds at least one native depth level, and type-token
    // brackets are bounded separately by `maxDepth`. Nesting beyond twice the budget therefore
    // exceeds one of them, so rejecting it here is exact and keeps recursion bounded.
    let ceiling = limits.max_depth.saturating_mul(2);
    let bytes = source.as_bytes();
    let mut depth = 0usize;
    let mut at = 0usize;
    while at < bytes.len() {
        match bytes[at] {
            b'"' => at = skip_quoted(bytes, at),
            b'#' => at = skip_comment(bytes, at),
            b'/' if bytes.get(at + 1) == Some(&b'/') => at = skip_comment(bytes, at),
            b'{' | b'[' | b'(' => {
                depth += 1;
                if depth > ceiling {
                    return Err(limit_error(source, at, BsolLimitKind::DepthLimit, NESTING));
                }
                at += 1;
            }
            b'}' | b']' | b')' => {
                depth = depth.saturating_sub(1);
                at += 1;
            }
            _ => at += 1,
        }
    }
    Ok(())
}

/// Node, depth and scalar budgets over the parsed document.
pub(super) fn check_document(
    source: &ParseSource<'_>,
    document: &BsolDocument,
    limits: &BsolLimits,
) -> Result<usize, BsolError> {
    let mut accounting = Accounting {
        source,
        limits,
        events: Vec::new(),
        first: None,
    };
    for block in &document.blocks {
        accounting.statement_block(block, 1);
    }
    let nodes = accounting.events.len();
    accounting.comments();
    if accounting.events.len() > limits.max_nodes {
        accounting.events.sort_unstable();
        let (offset, comment) = accounting.events[limits.max_nodes];
        let message = if comment {
            "comment limit"
        } else {
            "syntax node limit"
        };
        accounting.violate(offset, BsolLimitKind::NodeLimit, message);
    }
    match accounting.first {
        Some((offset, kind, message)) => Err(limit_error(source, offset, kind, message)),
        None => Ok(nodes),
    }
}

fn limit_error(
    source: &ParseSource<'_>,
    offset: usize,
    kind: BsolLimitKind,
    message: &'static str,
) -> BsolError {
    let offset = offset.min(source.len());
    BsolError::limit(kind, message, source.span(offset, offset).line, offset)
}

fn skip_quoted(bytes: &[u8], start: usize) -> usize {
    let mut at = start + 1;
    while at < bytes.len() {
        match bytes[at] {
            b'\\' => at += 2,
            b'"' => return at + 1,
            _ => at += 1,
        }
    }
    at
}

fn skip_comment(bytes: &[u8], start: usize) -> usize {
    let mut at = start;
    while at < bytes.len() && bytes[at] != b'\n' && bytes[at] != b'\r' {
        at += 1;
    }
    at
}

fn id_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// One native `Value` node, from an assignment value or a list item.
#[derive(Clone, Copy)]
enum Value<'a> {
    Quoted(&'a BsolQuotedString),
    Bare(&'a str),
    Number(&'a BsolNumber),
    Block(&'a BsolBlock),
    Bool,
    List(&'a BsolBracketList),
    Map(&'a BsolInlineMap),
    Ref(&'a BsolRef),
}

struct Accounting<'s, 'l> {
    source: &'s ParseSource<'s>,
    limits: &'l BsolLimits,
    /// Node and trivia positions; `true` marks a comment.
    events: Vec<(usize, bool)>,
    first: Option<(usize, BsolLimitKind, &'static str)>,
}

impl Accounting<'_, '_> {
    fn violate(&mut self, offset: usize, kind: BsolLimitKind, message: &'static str) {
        if self.first.is_none_or(|(first, _, _)| offset < first) {
            self.first = Some((offset, kind, message));
        }
    }

    fn enter(&mut self, depth: usize, offset: usize) -> bool {
        if depth > self.limits.max_depth {
            self.violate(offset, BsolLimitKind::DepthLimit, NESTING);
            return false;
        }
        true
    }

    fn node(&mut self, offset: usize) {
        self.events.push((offset, false));
    }

    fn ident(&mut self, text: &str, offset: usize) {
        if text.len() > self.limits.max_scalar_bytes {
            self.violate(offset, BsolLimitKind::ScalarLimit, IDENTIFIER);
        }
    }

    /// Native `Lexer.Quoted`: a raw run fails as a decoded string, an escape as a decoded scalar.
    fn quoted(&mut self, quoted: &BsolQuotedString) {
        let max = self.limits.max_scalar_bytes;
        let start = quoted.span.start;
        let Some(raw) = self.source.get(start..quoted.span.end) else {
            return;
        };
        let bytes = raw.as_bytes();
        let mut text = 0usize;
        let mut at = 1usize;
        let mut run = at;
        while at < bytes.len() {
            match bytes[at] {
                b'"' => {
                    if at - run > max - text {
                        self.violate(
                            start,
                            BsolLimitKind::ScalarLimit,
                            "decoded string byte limit",
                        );
                    }
                    return;
                }
                b'\\' => {
                    if at - run > max - text {
                        self.violate(
                            start,
                            BsolLimitKind::ScalarLimit,
                            "decoded string byte limit",
                        );
                        return;
                    }
                    text += at - run;
                    let (scalar, next) = escape(bytes, at);
                    let width = char::from_u32(scalar).map_or(1, char::len_utf8);
                    if width > max - text {
                        self.violate(
                            start,
                            BsolLimitKind::ScalarLimit,
                            "decoded scalar byte limit",
                        );
                        return;
                    }
                    text += width;
                    at = next;
                    run = at;
                }
                _ => at += 1,
            }
        }
    }

    fn statement_block(&mut self, block: &BsolBlock, depth: usize) {
        let offset = block.span.start;
        if !self.enter(depth, offset) {
            return;
        }
        self.attributes(&block.attrs, depth);
        self.ident(&block.kind, offset);
        if let Some(label) = &block.label {
            self.quoted(label);
        }
        self.node(offset);
        self.body(block, depth);
    }

    fn body(&mut self, block: &BsolBlock, depth: usize) {
        match &block.schemaless_body {
            // Native parses a schemaless body as ordinary statements; account for it the same way.
            Some(raw) => {
                let end = block.span.end.saturating_sub(1);
                let start = end.saturating_sub(raw.len());
                if let Ok(items) = parse_block_items_in_range(self.source, start, end) {
                    self.items(&items, depth + 1);
                }
            }
            None => self.items(&block.items, depth + 1),
        }
    }

    fn items(&mut self, items: &[BsolItem], depth: usize) {
        for item in items {
            match item {
                BsolItem::Block(block) => self.statement_block(block, depth),
                BsolItem::Assignment(assignment) => {
                    let offset = assignment.span.start;
                    if !self.enter(depth, offset) {
                        continue;
                    }
                    self.attributes(&assignment.attrs, depth);
                    self.ident(&assignment.key, offset);
                    self.node(offset);
                    self.value(&assignment.value, depth + 1, offset);
                }
            }
        }
    }

    fn attributes(&mut self, attributes: &[BsolAttribute], depth: usize) {
        for attribute in attributes {
            let offset = attribute.span.start;
            self.ident(&attribute.name, offset);
            self.node(offset);
            for arg in &attribute.args {
                if !self.enter(depth + 1, offset) {
                    continue;
                }
                self.ident(&arg.key, offset);
                self.node(offset);
                self.value(&arg.value, depth + 2, offset);
            }
        }
    }

    fn value(&mut self, value: &BsolValue, depth: usize, at: usize) {
        let item = match value {
            BsolValue::QuotedString(quoted) => Value::Quoted(quoted),
            BsolValue::Ident(text) => Value::Bare(text),
            BsolValue::Number(number) => Value::Number(number),
            BsolValue::InlineBlock(block) => Value::Block(block),
            BsolValue::Bool(_) => Value::Bool,
            BsolValue::BracketList(list) => Value::List(list),
            BsolValue::InlineMap(map) => Value::Map(map),
            BsolValue::Ref(reference) => Value::Ref(reference),
        };
        self.value_item(item, depth, at);
    }

    fn list_item(&mut self, item: &BsolListItem, depth: usize, at: usize) {
        let item = match item {
            BsolListItem::Default => Value::Bare("default"),
            BsolListItem::QuotedString(quoted) => Value::Quoted(quoted),
            BsolListItem::Ident(text) => Value::Bare(text),
            BsolListItem::Number(number) => Value::Number(number),
            BsolListItem::Bool(_) => Value::Bool,
            BsolListItem::Ref(reference) => Value::Ref(reference),
            BsolListItem::InlineMap(map) => Value::Map(map),
            BsolListItem::InlineBlock(block) => Value::Block(block),
        };
        self.value_item(item, depth, at);
    }

    fn value_item(&mut self, value: Value<'_>, depth: usize, at: usize) {
        let offset = match value {
            Value::Quoted(quoted) => quoted.span.start,
            Value::Number(number) => number.span.start,
            Value::Block(block) => block.span.start,
            Value::List(list) => list.span.start,
            Value::Map(map) => map.span.start,
            Value::Ref(reference) => reference.span.start,
            Value::Bare(_) | Value::Bool => at,
        };
        if !self.enter(depth, offset) {
            return;
        }
        self.node(offset);
        match value {
            Value::Quoted(quoted) => self.quoted(quoted),
            Value::Bare(text) => self.bare_token(text, offset),
            Value::Number(number) => self.number(&number.raw, offset),
            Value::Block(block) => self.inline_block(block, depth),
            Value::Bool => {}
            Value::List(list) => {
                for item in &list.items {
                    self.list_item(item, depth + 1, offset);
                }
            }
            Value::Map(map) => {
                for entry in &map.entries {
                    let entry_offset = entry.span.start;
                    if !self.enter(depth + 1, entry_offset) {
                        continue;
                    }
                    self.ident(&entry.key, entry_offset);
                    self.node(entry_offset);
                    self.value(&entry.value, depth + 2, entry_offset);
                }
            }
            Value::Ref(reference) => self.reference(reference),
        }
    }

    fn inline_block(&mut self, block: &BsolBlock, depth: usize) {
        let offset = block.span.start;
        self.ident(&block.kind, offset);
        if let Some(label) = &block.label {
            self.quoted(label);
        }
        self.body(block, depth);
    }

    fn number(&mut self, raw: &str, offset: usize) {
        if raw.len() > self.limits.max_scalar_bytes {
            self.violate(offset, BsolLimitKind::ScalarLimit, "number byte limit");
        }
    }

    /// Native reads the head identifier, then scans a type token when `[` or `(` follows.
    fn bare_token(&mut self, text: &str, offset: usize) {
        let head = text.bytes().take_while(|byte| id_byte(*byte)).count();
        self.ident(&text[..head], offset);
        if head == text.len() {
            return;
        }
        let mut nesting = 0usize;
        for byte in text[head..].bytes() {
            match byte {
                b'[' | b'(' => {
                    nesting += 1;
                    if nesting > self.limits.max_depth {
                        self.violate(
                            offset,
                            BsolLimitKind::DepthLimit,
                            "type expression nesting limit",
                        );
                        return;
                    }
                }
                b']' | b')' => nesting = nesting.saturating_sub(1),
                _ => {}
            }
        }
        if text.len() > self.limits.max_scalar_bytes {
            self.violate(offset, BsolLimitKind::ScalarLimit, "type token byte limit");
        }
    }

    fn reference(&mut self, reference: &BsolRef) {
        let offset = reference.span.start;
        if let Some(qualifier) = &reference.rule_kind {
            let mut segments = qualifier.split('.');
            let first = segments.next().unwrap_or_default();
            self.ident(first, offset);
            let mut combined = first.len();
            for segment in segments {
                self.ident(segment, offset);
                if combined + 1 + segment.len() > self.limits.max_scalar_bytes {
                    self.violate(
                        offset,
                        BsolLimitKind::ScalarLimit,
                        "reference qualifier byte limit",
                    );
                    return;
                }
                combined += 1 + segment.len();
            }
        }
        self.ident(&reference.label, offset);
    }

    fn comments(&mut self) {
        let bytes = self.source.as_bytes();
        let mut at = 0usize;
        while at < bytes.len() {
            match bytes[at] {
                b'"' => at = skip_quoted(bytes, at),
                b'#' => {
                    self.events.push((at, true));
                    at = skip_comment(bytes, at);
                }
                b'/' if bytes.get(at + 1) == Some(&b'/') => {
                    self.events.push((at, true));
                    at = skip_comment(bytes, at);
                }
                _ => at += 1,
            }
        }
    }
}

/// Decode the already-validated escape at `at` (the backslash); returns the scalar and next offset.
fn escape(bytes: &[u8], at: usize) -> (u32, usize) {
    match bytes.get(at + 1) {
        Some(b'u') => {
            let high = hex4(bytes, at + 2);
            if (0xD800..=0xDBFF).contains(&high) && bytes.get(at + 6) == Some(&b'\\') {
                let low = hex4(bytes, at + 8);
                if (0xDC00..=0xDFFF).contains(&low) {
                    return (0x10000 + ((high - 0xD800) << 10) + (low - 0xDC00), at + 12);
                }
            }
            (high, at + 6)
        }
        _ => (0x20, at + 2),
    }
}

fn hex4(bytes: &[u8], at: usize) -> u32 {
    bytes
        .get(at..at + 4)
        .and_then(|digits| std::str::from_utf8(digits).ok())
        .and_then(|digits| u32::from_str_radix(digits, 16).ok())
        .unwrap_or(0)
}
