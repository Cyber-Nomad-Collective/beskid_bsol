use std::collections::{HashMap, HashSet};

use bsol_schema::{BlockRule, Cardinality, LabelRequirement, ValueType, VariantRule};
use bsol_syntax::{BsolAttribute, BsolBlock, BsolItem};

use super::{field::validate_field_value, formatting::extra_field_value, model::ValidatedBlock};
use crate::{BsolError, registry::ValidatorRegistry, value::ValidatedValue};

pub(super) fn validate_block(
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

    for nested_rule in rule
        .nested_order
        .iter()
        .filter_map(|id| rule.nested.get(id))
    {
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
            if matches!(
                field_rule.value_type,
                ValueType::List | ValueType::ListOf(_)
            ) {
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
    let active = select_variant(&rule.variants, fields)
        .ok_or_else(|| BsolError::schema_at(span, "no matching variant for block fields"))?;
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
