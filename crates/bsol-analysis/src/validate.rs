//! Validate a parsed document against a schema profile.

use std::collections::{HashMap, HashSet};

use bsol_schema::{
    BlockRule, Cardinality, FieldRule, LabelRequirement, SchemaProfile, ValueType, VariantRule,
};
use bsol_syntax::{
    BsolAssignment, BsolAttribute, BsolBlock, BsolDocument, BsolInlineMap, BsolItem, BsolListItem,
    BsolRef, BsolValue,
};

use crate::registry::ValidatorRegistry;
use crate::value::{ValidatedBlockLite, ValidatedValue};
use crate::BsolError;

/// Document validated against a schema profile; blocks carry matched rule ids.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedDocument {
    pub profile: String,
    pub blocks: Vec<ValidatedBlock>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedBlock {
    pub span: bsol_syntax::BsolSpan,
    pub rule_id: String,
    pub kind: String,
    pub label: Option<String>,
    pub attrs: Vec<BsolAttribute>,
    pub fields: HashMap<String, String>,
    pub values: HashMap<String, ValidatedValue>,
    pub field_spans: HashMap<String, bsol_syntax::BsolSpan>,
    pub extras: HashMap<String, String>,
    pub nested: Vec<ValidatedBlock>,
    pub lists: HashMap<String, Vec<String>>,
    pub raw_body: Option<String>,
}

impl ValidatedBlock {
    pub fn field(&self, key: &str) -> Option<&str> {
        self.fields.get(key).map(String::as_str)
    }

    pub fn value(&self, key: &str) -> Option<&ValidatedValue> {
        self.values.get(key)
    }
}

/// Validate `document` against `profile`.
pub fn validate(document: &BsolDocument, profile: &SchemaProfile) -> Result<ValidatedDocument, BsolError> {
    validate_with(document, profile, &ValidatorRegistry::default())
}

/// Validate with optional custom semantic validators.
pub fn validate_with(
    document: &BsolDocument,
    profile: &SchemaProfile,
    registry: &ValidatorRegistry,
) -> Result<ValidatedDocument, BsolError> {
    let mut validated_top = Vec::new();
    let mut rule_counts: HashMap<String, usize> = HashMap::new();

    for block in &document.blocks {
        let rule = match_top_level_rule(profile, &block.kind).ok_or_else(|| {
            BsolError::schema_at(
                block.span,
                format!("unknown top-level block `{kind}`", kind = block.kind),
            )
        })?;
        let validated = validate_block(block, rule, registry)?;
        *rule_counts.entry(rule.id.clone()).or_default() += 1;
        validated_top.push(validated);
    }

    for rule in profile.top_level_rules() {
        let count = rule_counts.get(&rule.id).copied().unwrap_or(0);
        match rule.cardinality {
            Cardinality::One if count != 1 => {
                return Err(BsolError::Schema(format!(
                    "profile `{}` requires exactly one `{}` block, found {count}",
                    profile.name, rule.id
                )));
            }
            Cardinality::ZeroOrOne if count > 1 => {
                return Err(BsolError::Schema(format!(
                    "profile `{}` allows at most one `{}` block, found {count}",
                    profile.name, rule.id
                )));
            }
            _ => {}
        }
    }

    Ok(ValidatedDocument {
        profile: profile.name.clone(),
        blocks: validated_top,
    })
}

fn match_top_level_rule<'a>(profile: &'a SchemaProfile, kind: &str) -> Option<&'a BlockRule> {
    profile.top_level_rules().find(|rule| rule.matches_kind(kind))
}

fn validate_block(
    block: &BsolBlock,
    rule: &BlockRule,
    registry: &ValidatorRegistry,
) -> Result<ValidatedBlock, BsolError> {
    validate_block_attrs(&block.attrs, &rule.allowed_attrs, block.span)?;

    match (&block.schemaless_body, rule.schemaless) {
        (Some(_), false) => {
            return Err(BsolError::schema_at(
                block.span,
                format!(
                    "block `{}` uses `@schemaless` but profile rule `{}` is structured",
                    block.kind, rule.id
                ),
            ));
        }
        (None, true) => {
            return Err(BsolError::schema_at(
                block.span,
                format!(
                    "block `{}` requires `@schemaless` for profile rule `{}`",
                    block.kind, rule.id
                ),
            ));
        }
        (Some(raw), true) => {
            return Ok(empty_validated_block(block, rule, Some(raw.clone())));
        }
        (None, false) => {}
    }

    match rule.label {
        LabelRequirement::Required if block.label.is_none() => {
            return Err(BsolError::schema_at(
                block.span,
                format!("block `{}` requires a label", block.kind),
            ));
        }
        LabelRequirement::Forbidden if block.label.is_some() => {
            return Err(BsolError::schema_at(
                block.span,
                format!("block `{}` cannot carry a label", block.kind),
            ));
        }
        _ => {}
    }

    let mut fields = HashMap::new();
    let mut values = HashMap::new();
    let mut field_spans = HashMap::new();
    let mut extras = HashMap::new();
    let mut lists = HashMap::new();
    let mut nested = Vec::new();
    let mut nested_counts: HashMap<String, usize> = HashMap::new();
    let mut seen_keys: HashSet<String> = HashSet::new();

    for item in &block.items {
        match item {
            BsolItem::Assignment(assignment) => {
                validate_block_attrs(&assignment.attrs, &[], assignment.span)?;
                let key = assignment.key.clone();
                if !seen_keys.insert(key.clone()) {
                    return Err(BsolError::schema_at(
                        assignment.span,
                        format!("duplicate field `{key}` on block `{}`", block.kind),
                    ));
                }
                if let Some(field_rule) = rule.fields.get(&key) {
                    let validated = validate_field_value(assignment, field_rule)?;
                    field_spans.insert(key.clone(), assignment.span);
                    if let Some(list) = validated.as_list_strings() {
                        lists.insert(key.clone(), list);
                    }
                    if let Some(s) = validated.as_string() {
                        fields.insert(key.clone(), s);
                    }
                    values.insert(key, validated);
                } else if rule.allow_extra_fields {
                    let value = extra_field_value(assignment)?;
                    extras.insert(key, value);
                } else {
                    return Err(BsolError::schema_at(
                        assignment.span,
                        format!("unknown field `{key}` on block `{}`", block.kind),
                    ));
                }
            }
            BsolItem::Block(nested_block) => {
                if let Some(nested_rule) = rule.nested_rule_for_kind(&nested_block.kind) {
                    let validated = validate_block(nested_block, nested_rule, registry)?;
                    *nested_counts.entry(nested_rule.id.clone()).or_default() += 1;
                    nested.push(validated);
                } else if rule.allow_extra_nested {
                    nested.push(empty_validated_block(
                        nested_block,
                        &BlockRule {
                            id: nested_block.kind.clone(),
                            scope: bsol_schema::RuleScope::Nested,
                            kind_match: bsol_schema::KindMatch::Keyword(nested_block.kind.clone()),
                            label: LabelRequirement::Optional,
                            cardinality: Cardinality::Many,
                            fields: HashMap::new(),
                            nested: HashMap::new(),
                            nested_order: Vec::new(),
                            allow_extra_fields: true,
                            allow_extra_nested: true,
                            schemaless: nested_block.schemaless_body.is_some(),
                            extends: None,
                            mixes: Vec::new(),
                            variants: Vec::new(),
                            allowed_attrs: Vec::new(),
                        },
                        nested_block.schemaless_body.clone(),
                    ));
                } else {
                    return Err(BsolError::schema_at(
                        nested_block.span,
                        format!(
                            "nested block `{}` not allowed inside `{}`",
                            nested_block.kind, block.kind
                        ),
                    ));
                }
            }
        }
    }

    apply_defaults(rule, &mut fields, &mut values, &mut lists);
    validate_conditional_fields(rule, &fields, block.span)?;
    validate_variants(rule, &fields, block.span)?;

    for (field_name, field_rule) in &rule.fields {
        let present = fields.contains_key(field_name)
            || lists.contains_key(field_name)
            || values.contains_key(field_name);
        if field_rule.required && !present {
            return Err(BsolError::schema_at(
                block.span,
                format!("missing required field `{field_name}`"),
            ));
        }
    }

    for nested_rule in rule.nested_order.iter().filter_map(|id| rule.nested.get(id)) {
        let count = nested_counts.get(&nested_rule.id).copied().unwrap_or(0);
        match nested_rule.cardinality {
            Cardinality::One if count != 1 => {
                return Err(BsolError::schema_at(
                    block.span,
                    format!(
                        "expected exactly one nested `{}` block, found {count}",
                        nested_rule.id
                    ),
                ));
            }
            Cardinality::ZeroOrOne if count > 1 => {
                return Err(BsolError::schema_at(
                    block.span,
                    format!(
                        "expected at most one nested `{}` block, found {count}",
                        nested_rule.id
                    ),
                ));
            }
            _ => {}
        }
    }

    let validated = ValidatedBlock {
        span: block.span,
        rule_id: rule.id.clone(),
        kind: block.kind.clone(),
        label: block.label.as_ref().map(|q| q.value.clone()),
        attrs: block.attrs.clone(),
        fields,
        values,
        field_spans,
        extras,
        nested,
        lists,
        raw_body: None,
    };

    registry.run(&validated)?;
    Ok(validated)
}

fn empty_validated_block(
    block: &BsolBlock,
    rule: &BlockRule,
    raw_body: Option<String>,
) -> ValidatedBlock {
    ValidatedBlock {
        span: block.span,
        rule_id: rule.id.clone(),
        kind: block.kind.clone(),
        label: block.label.as_ref().map(|q| q.value.clone()),
        attrs: block.attrs.clone(),
        fields: HashMap::new(),
        values: HashMap::new(),
        field_spans: HashMap::new(),
        extras: HashMap::new(),
        nested: Vec::new(),
        lists: HashMap::new(),
        raw_body,
    }
}

fn validate_block_attrs(
    attrs: &[BsolAttribute],
    allowed: &[String],
    span: bsol_syntax::BsolSpan,
) -> Result<(), BsolError> {
    if allowed.is_empty() {
        return Ok(());
    }
    for attr in attrs {
        if !allowed.iter().any(|a| a == &attr.name) {
            return Err(BsolError::schema_at(
                attr.span,
                format!("attribute `@{}` not allowed here", attr.name),
            ));
        }
    }
    let _ = span;
    Ok(())
}

fn apply_defaults(
    rule: &BlockRule,
    fields: &mut HashMap<String, String>,
    values: &mut HashMap<String, ValidatedValue>,
    lists: &mut HashMap<String, Vec<String>>,
) {
    for (name, field_rule) in &rule.fields {
        if values.contains_key(name) {
            continue;
        }
        if let Some(default) = &field_rule.constraints.default_value {
            fields.insert(name.clone(), default.clone());
            values.insert(name.clone(), ValidatedValue::String(default.clone()));
            if matches!(field_rule.value_type, ValueType::List | ValueType::ListOf(_)) {
                lists.insert(
                    name.clone(),
                    default.split(',').map(|s| s.trim().to_string()).collect(),
                );
            }
        }
    }
}

fn validate_conditional_fields(
    rule: &BlockRule,
    fields: &HashMap<String, String>,
    span: bsol_syntax::BsolSpan,
) -> Result<(), BsolError> {
    for (name, field_rule) in &rule.fields {
        for (key, expected) in &field_rule.constraints.required_if {
            if fields.get(key).is_some_and(|v| v == expected) && !fields.contains_key(name) {
                return Err(BsolError::schema_at(
                    span,
                    format!("field `{name}` required when `{key}` = `{expected}`"),
                ));
            }
        }
        for (key, expected) in &field_rule.constraints.forbid_if {
            if fields.get(key).is_some_and(|v| v == expected) && fields.contains_key(name) {
                return Err(BsolError::schema_at(
                    span,
                    format!("field `{name}` forbidden when `{key}` = `{expected}`"),
                ));
            }
        }
    }
    Ok(())
}

fn validate_variants(
    rule: &BlockRule,
    fields: &HashMap<String, String>,
    span: bsol_syntax::BsolSpan,
) -> Result<(), BsolError> {
    if rule.variants.is_empty() {
        return Ok(());
    }
    let active = select_variant(&rule.variants, fields).ok_or_else(|| {
        BsolError::schema_at(span, "no matching variant for block fields")
    })?;
    for key in &active.require {
        if !fields.contains_key(key) {
            return Err(BsolError::schema_at(
                span,
                format!("variant `{}` requires field `{key}`", active.name),
            ));
        }
    }
    for key in &active.forbid {
        if fields.contains_key(key) {
            return Err(BsolError::schema_at(
                span,
                format!("variant `{}` forbids field `{key}`", active.name),
            ));
        }
    }
    Ok(())
}

fn select_variant<'a>(
    variants: &'a [VariantRule],
    fields: &HashMap<String, String>,
) -> Option<&'a VariantRule> {
    if let Some(source) = fields.get("source") {
        return variants.iter().find(|v| v.name == *source);
    }
    if let Some(kind) = fields.get("kind") {
        return variants.iter().find(|v| v.name == *kind);
    }
    variants.first()
}

fn validate_field_value(
    assignment: &BsolAssignment,
    rule: &FieldRule,
) -> Result<ValidatedValue, BsolError> {
    let value = match &rule.value_type {
        ValueType::Quoted => ValidatedValue::String(require_quoted(assignment)?),
        ValueType::Ident => ValidatedValue::String(require_ident(assignment)?),
        ValueType::U32 => {
            let text = require_u32(assignment)?;
            let parsed: u32 = text.parse().map_err(|_| {
                BsolError::schema_at(assignment.span, format!("invalid u32 `{text}`"))
            })?;
            apply_numeric_constraints(&rule.constraints, parsed as i64, assignment.span)?;
            ValidatedValue::U32(parsed)
        }
        ValueType::I64 => {
            let text = require_i64(assignment)?;
            let parsed: i64 = text.parse().map_err(|_| {
                BsolError::schema_at(assignment.span, format!("invalid i64 `{text}`"))
            })?;
            apply_numeric_constraints(&rule.constraints, parsed, assignment.span)?;
            ValidatedValue::I64(parsed)
        }
        ValueType::F64 => {
            let text = require_f64(assignment)?;
            let parsed: f64 = text.parse().map_err(|_| {
                BsolError::schema_at(assignment.span, format!("invalid f64 `{text}`"))
            })?;
            ValidatedValue::F64(parsed)
        }
        ValueType::Bool => ValidatedValue::Bool(require_bool(assignment)?),
        ValueType::Path => ValidatedValue::String(require_quoted(assignment)?),
        ValueType::Loose => ValidatedValue::String(loose_string(assignment)?),
        ValueType::List => {
            let list = require_list_strings(assignment)?;
            if let Some(allowed) = &rule.list_values {
                for item in &list {
                    if !allowed.iter().any(|v| v == item) {
                        return Err(BsolError::schema_at(
                            assignment.span,
                            format!("unsupported list value `{item}`"),
                        ));
                    }
                }
            }
            ValidatedValue::List(list.into_iter().map(ValidatedValue::String).collect())
        }
        ValueType::ListOf(element_types) => {
            let items = require_list_items(assignment)?;
            let mut validated = Vec::new();
            for item in items {
                validated.push(validate_list_item(&item, element_types, assignment.span)?);
            }
            ValidatedValue::List(validated)
        }
        ValueType::MapOf { key, value } => {
            let map = require_inline_map(assignment)?;
            validate_inline_map(&map, key, value, assignment.span)?
        }
        ValueType::RefTo(rule_id) => {
            let reference = require_ref(assignment)?;
            if let Some(kind) = &reference.rule_kind {
                if kind != rule_id {
                    return Err(BsolError::schema_at(
                        assignment.span,
                        format!("expected ref({rule_id}), found @{kind}/{}", reference.label),
                    ));
                }
            }
            ValidatedValue::Ref(reference)
        }
        ValueType::Inline(kind) => {
            let block = require_inline_block(assignment)?;
            if block.kind != *kind {
                return Err(BsolError::schema_at(
                    assignment.span,
                    format!("expected inline `{kind}` block"),
                ));
            }
            ValidatedValue::Block(Box::new(ValidatedBlockLite::from_block(&block)))
        }
        ValueType::EnumOrQuoted(values) => {
            ValidatedValue::String(enum_or_quoted(assignment, values)?)
        }
    };
    apply_string_constraints(&rule.constraints, &value, assignment.span)?;
    Ok(value)
}

fn validate_list_item(
    item: &BsolListItem,
    types: &[ValueType],
    span: bsol_syntax::BsolSpan,
) -> Result<ValidatedValue, BsolError> {
    for ty in types {
        if let Ok(value) = try_list_item_as_type(item, ty, span) {
            return Ok(value);
        }
    }
    Err(BsolError::schema_at(
        span,
        "list item does not match allowed types",
    ))
}

fn try_list_item_as_type(
    item: &BsolListItem,
    ty: &ValueType,
    span: bsol_syntax::BsolSpan,
) -> Result<ValidatedValue, BsolError> {
    match (item, ty) {
        (BsolListItem::QuotedString(q), ValueType::Quoted) => {
            Ok(ValidatedValue::String(q.value.clone()))
        }
        (BsolListItem::Ident(i), ValueType::Ident) => Ok(ValidatedValue::String(i.clone())),
        (BsolListItem::Ident(i), ValueType::U32) if i.chars().all(|c| c.is_ascii_digit()) => {
            Ok(ValidatedValue::U32(i.parse().unwrap_or(0)))
        }
        (BsolListItem::Bool(b), ValueType::Bool) => Ok(ValidatedValue::Bool(*b)),
        (BsolListItem::Ref(r), ValueType::RefTo(_)) => Ok(ValidatedValue::Ref(r.clone())),
        (BsolListItem::InlineBlock(block), ValueType::Inline(kind)) if block.kind == *kind => {
            Ok(ValidatedValue::Block(Box::new(ValidatedBlockLite::from_block(
                block,
            ))))
        }
        _ => Err(BsolError::schema_at(span, "type mismatch")),
    }
}

fn validate_inline_map(
    map: &BsolInlineMap,
    _key_ty: &ValueType,
    value_ty: &ValueType,
    span: bsol_syntax::BsolSpan,
) -> Result<ValidatedValue, BsolError> {
    let mut out = HashMap::new();
    for entry in &map.entries {
        let key = ValidatedValue::String(entry.key.clone());
        let fake = BsolAssignment {
            span: entry.span,
            attrs: Vec::new(),
            key: entry.key.clone(),
            value: entry.value.clone(),
        };
        let field = FieldRule {
            value_type: value_ty.clone(),
            required: true,
            list_values: None,
            constraints: Default::default(),
            allowed_attrs: Vec::new(),
        };
        let value = validate_field_value(&fake, &field)?;
        out.insert(key.as_string().unwrap_or(entry.key.clone()), value);
    }
    let _ = span;
    Ok(ValidatedValue::Map(out))
}

fn apply_numeric_constraints(
    constraints: &bsol_schema::FieldConstraints,
    value: i64,
    span: bsol_syntax::BsolSpan,
) -> Result<(), BsolError> {
    if let Some(min) = constraints.min {
        if value < min {
            return Err(BsolError::schema_at(span, format!("value {value} below min {min}")));
        }
    }
    if let Some(max) = constraints.max {
        if value > max {
            return Err(BsolError::schema_at(span, format!("value {value} above max {max}")));
        }
    }
    Ok(())
}

fn apply_string_constraints(
    constraints: &bsol_schema::FieldConstraints,
    value: &ValidatedValue,
    span: bsol_syntax::BsolSpan,
) -> Result<(), BsolError> {
    let Some(text) = value.as_string() else {
        return Ok(());
    };
    if let Some(pattern) = &constraints.pattern {
        if !simple_pattern_match(pattern, &text) {
            return Err(BsolError::schema_at(
                span,
                format!("value `{text}` does not match pattern `{pattern}`"),
            ));
        }
    }
    Ok(())
}

fn simple_pattern_match(pattern: &str, text: &str) -> bool {
    if pattern.starts_with('^') && pattern.ends_with('$') {
        let inner = &pattern[1..pattern.len().saturating_sub(1)];
        if inner.contains("[0-9]") {
            return !text.is_empty()
                && text
                    .chars()
                    .all(|c| c.is_ascii_digit() || c == '.' || c == '-');
        }
    }
    text.contains(pattern.trim_matches('^').trim_matches('$'))
}

fn require_quoted(assignment: &BsolAssignment) -> Result<String, BsolError> {
    match &assignment.value {
        BsolValue::QuotedString(q) => Ok(q.value.clone()),
        other => Err(BsolError::schema_at(
            assignment.span,
            format!("expected quoted string, found `{}`", other.preview()),
        )),
    }
}

fn require_ident(assignment: &BsolAssignment) -> Result<String, BsolError> {
    match &assignment.value {
        BsolValue::Ident(i) => Ok(i.clone()),
        BsolValue::QuotedString(q) => Ok(q.value.clone()),
        other => Err(BsolError::schema_at(
            assignment.span,
            format!("expected identifier, found `{}`", other.preview()),
        )),
    }
}

fn require_u32(assignment: &BsolAssignment) -> Result<String, BsolError> {
    match &assignment.value {
        BsolValue::Ident(text) if text.chars().all(|c| c.is_ascii_digit()) => Ok(text.clone()),
        other => Err(BsolError::schema_at(
            assignment.span,
            format!("expected positive integer, found `{}`", other.preview()),
        )),
    }
}

fn require_i64(assignment: &BsolAssignment) -> Result<String, BsolError> {
    match &assignment.value {
        BsolValue::Ident(text) if text.parse::<i64>().is_ok() => Ok(text.clone()),
        other => Err(BsolError::schema_at(
            assignment.span,
            format!("expected integer, found `{}`", other.preview()),
        )),
    }
}

fn require_f64(assignment: &BsolAssignment) -> Result<String, BsolError> {
    match &assignment.value {
        BsolValue::Ident(text) if text.parse::<f64>().is_ok() => Ok(text.clone()),
        other => Err(BsolError::schema_at(
            assignment.span,
            format!("expected float, found `{}`", other.preview()),
        )),
    }
}

fn require_bool(assignment: &BsolAssignment) -> Result<bool, BsolError> {
    match &assignment.value {
        BsolValue::Bool(b) => Ok(*b),
        BsolValue::Ident(i) if i == "true" || i == "false" => Ok(i == "true"),
        other => Err(BsolError::schema_at(
            assignment.span,
            format!("expected bool, found `{}`", other.preview()),
        )),
    }
}

fn loose_string(assignment: &BsolAssignment) -> Result<String, BsolError> {
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

fn require_ref(assignment: &BsolAssignment) -> Result<BsolRef, BsolError> {
    match &assignment.value {
        BsolValue::Ref(r) => Ok(r.clone()),
        other => Err(BsolError::schema_at(
            assignment.span,
            format!("expected reference, found `{}`", other.preview()),
        )),
    }
}

fn require_inline_map(assignment: &BsolAssignment) -> Result<BsolInlineMap, BsolError> {
    match &assignment.value {
        BsolValue::InlineMap(map) => Ok(map.clone()),
        other => Err(BsolError::schema_at(
            assignment.span,
            format!("expected inline map, found `{}`", other.preview()),
        )),
    }
}

fn require_inline_block(assignment: &BsolAssignment) -> Result<BsolBlock, BsolError> {
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

fn extra_field_value(assignment: &BsolAssignment) -> Result<String, BsolError> {
    match &assignment.value {
        BsolValue::QuotedString(q) => Ok(q.value.clone()),
        BsolValue::Ident(i) => Ok(i.clone()),
        BsolValue::Bool(b) => Ok(b.to_string()),
        BsolValue::Ref(r) => Ok(r.display()),
        BsolValue::BracketList(list) => Ok(format_bracket_list_literal(list)),
        BsolValue::InlineMap(map) => Ok(format_map_literal(map)),
    }
}

fn format_map_literal(map: &BsolInlineMap) -> String {
    let mut out = String::from("{");
    for (index, entry) in map.entries.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        out.push_str(&entry.key);
        out.push_str(" = ");
        out.push_str(&entry.value.preview());
    }
    out.push('}');
    out
}

fn format_bracket_list_literal(list: &bsol_syntax::BsolBracketList) -> String {
    let mut out = String::from("[");
    for (index, item) in list.items.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        match item {
            BsolListItem::Default => out.push_str("default"),
            BsolListItem::QuotedString(q) => {
                out.push('"');
                out.push_str(&q.value);
                out.push('"');
            }
            BsolListItem::Ident(i) => out.push_str(i),
            BsolListItem::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            BsolListItem::Ref(r) => out.push_str(&r.display()),
            BsolListItem::InlineMap(m) => out.push_str(&format_map_literal(m)),
            BsolListItem::InlineBlock(b) => {
                out.push_str(&b.kind);
                if let Some(label) = &b.label {
                    out.push('"');
                    out.push_str(&label.value);
                    out.push('"');
                }
                out.push_str(" { ... }");
            }
        }
    }
    out.push(']');
    out
}

fn require_list_strings(assignment: &BsolAssignment) -> Result<Vec<String>, BsolError> {
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

fn require_list_items(assignment: &BsolAssignment) -> Result<Vec<BsolListItem>, BsolError> {
    let BsolValue::BracketList(list) = &assignment.value else {
        return Err(BsolError::schema_at(
            assignment.span,
            "expected bracket list",
        ));
    };
    Ok(list.items.clone())
}

fn enum_or_quoted(assignment: &BsolAssignment, allowed: &[String]) -> Result<String, BsolError> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use bsol_schema::load_profile;

    #[test]
    fn validates_project_v2_dependency_variants() {
        let src = r#"demo {
  name = "demo"
  version = "1.0.0"
  root = "."
}
dependency "core" {
  source = git
  url = "https://example.com"
  rev = "main"
}
"#;
        let doc = bsol_syntax::parse_bsol_document(src).expect("parse");
        let profile = load_profile("project.v2").expect("profile");
        validate(&doc, &profile).expect("validate git dependency");
    }

    #[test]
    fn rejects_git_dependency_missing_rev() {
        let src = r#"demo {
  name = "demo"
  version = "1.0.0"
  root = "."
}
dependency "core" {
  source = git
  url = "https://example.com"
}
"#;
        let doc = bsol_syntax::parse_bsol_document(src).expect("parse");
        let profile = load_profile("project.v2").expect("profile");
        assert!(validate(&doc, &profile).is_err());
    }
}
