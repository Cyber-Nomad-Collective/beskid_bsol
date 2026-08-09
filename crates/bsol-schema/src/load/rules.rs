use std::collections::HashMap;

use bsol_syntax::{BsolBlock, BsolItem};

use super::{
    fields::parse_field_rule,
    value_decode::{assignment_list, assignment_string, parse_bool},
    variants::parse_variant_rule,
};
use crate::{BlockRule, BsolError, Cardinality, KindMatch, LabelRequirement, RuleScope};

pub(super) fn parse_block_rule(
    block: &BsolBlock,
    default_scope: RuleScope,
) -> Result<BlockRule, BsolError> {
    let id = block
        .label
        .as_ref()
        .map(|q| q.value.clone())
        .or_else(|| assignment_string(block, "id").ok().flatten())
        .unwrap_or_else(|| block.kind.clone());
    let scope = parse_scope(block)?.unwrap_or(default_scope);
    let kind_match = parse_kind_match(block)?;
    let label = parse_label(block)?;
    let cardinality = parse_cardinality(block)?;
    let allow_extra_fields = parse_bool(block, "extras").unwrap_or(false);
    let allow_extra_nested = parse_bool(block, "nested_extras").unwrap_or(false);
    let schemaless = parse_bool(block, "schemaless").unwrap_or(false);
    let extends = assignment_string(block, "extends")?;
    let mixes = assignment_list(block, "mixes").unwrap_or_default();
    let allowed_attrs = assignment_list(block, "allowed_attrs").unwrap_or_default();

    let mut fields = HashMap::new();
    let mut nested = HashMap::new();
    let mut nested_order = Vec::new();
    let mut variants = Vec::new();
    for item in &block.items {
        match item {
            BsolItem::Block(nested_block) if nested_block.kind == "field" => {
                let field_name = nested_block
                    .label
                    .as_ref()
                    .map(|q| q.value.clone())
                    .or_else(|| assignment_string(nested_block, "name").ok().flatten())
                    .ok_or_else(|| {
                        BsolError::schema_at(
                            nested_block.span,
                            "`field` block requires a quoted label or `name`",
                        )
                    })?;
                let field_rule = parse_field_rule(nested_block)?;
                fields.insert(field_name, field_rule);
            }
            BsolItem::Block(nested_block) if nested_block.kind == "nested" => {
                let nested_id = nested_block
                    .label
                    .as_ref()
                    .map(|q| q.value.clone())
                    .or_else(|| assignment_string(nested_block, "id").ok().flatten())
                    .ok_or_else(|| {
                        BsolError::schema_at(
                            nested_block.span,
                            "`nested` block requires a quoted label or `id`",
                        )
                    })?;
                let nested_rule = parse_block_rule(nested_block, RuleScope::Nested)?;
                nested_order.push(nested_id.clone());
                nested.insert(nested_id, nested_rule);
            }
            BsolItem::Block(nested_block) if nested_block.kind == "variant" => {
                variants.push(parse_variant_rule(nested_block)?);
            }
            BsolItem::Assignment(_) => {}
            BsolItem::Block(other) => {
                return Err(BsolError::schema_at(
                    other.span,
                    format!("unexpected `{}` inside rule", other.kind),
                ));
            }
        }
    }

    Ok(BlockRule {
        id,
        scope,
        kind_match,
        label,
        cardinality,
        fields,
        nested,
        nested_order,
        allow_extra_fields,
        allow_extra_nested,
        schemaless,
        extends,
        mixes,
        variants,
        allowed_attrs,
    })
}

fn parse_kind_match(block: &BsolBlock) -> Result<KindMatch, BsolError> {
    let match_kind = assignment_string(block, "match")?.unwrap_or_else(|| "keyword".to_string());
    match match_kind.as_str() {
        "keyword" => {
            let keyword = assignment_string(block, "keyword")?.ok_or_else(|| {
                BsolError::schema_at(block.span, "`match = keyword` requires `keyword`")
            })?;
            Ok(KindMatch::Keyword(keyword))
        }
        "keywords" => {
            let keywords = assignment_list(block, "keywords")?;
            Ok(KindMatch::Keywords(keywords))
        }
        "free_ident" => {
            let except = assignment_list(block, "except").unwrap_or_default();
            Ok(KindMatch::FreeIdent { except })
        }
        other => Err(BsolError::schema_at(
            block.span,
            format!("unknown kind match `{other}`"),
        )),
    }
}

fn parse_scope(block: &BsolBlock) -> Result<Option<RuleScope>, BsolError> {
    let Some(scope) = assignment_string(block, "scope")? else {
        return Ok(None);
    };
    Ok(Some(match scope.as_str() {
        "top" => RuleScope::TopLevel,
        "nested" => RuleScope::Nested,
        "any" => RuleScope::Any,
        other => {
            return Err(BsolError::schema_at(
                block.span,
                format!("unknown scope `{other}`"),
            ));
        }
    }))
}

fn parse_label(block: &BsolBlock) -> Result<LabelRequirement, BsolError> {
    let Some(label) = assignment_string(block, "label")? else {
        return Ok(LabelRequirement::Optional);
    };
    Ok(match label.as_str() {
        "required" => LabelRequirement::Required,
        "forbidden" => LabelRequirement::Forbidden,
        "optional" => LabelRequirement::Optional,
        other => {
            return Err(BsolError::schema_at(
                block.span,
                format!("unknown label requirement `{other}`"),
            ));
        }
    })
}

fn parse_cardinality(block: &BsolBlock) -> Result<Cardinality, BsolError> {
    let Some(card) = assignment_string(block, "cardinality")? else {
        return Ok(Cardinality::Many);
    };
    Ok(match card.as_str() {
        "one" => Cardinality::One,
        "many" => Cardinality::Many,
        "zero_or_one" => Cardinality::ZeroOrOne,
        other => {
            return Err(BsolError::schema_at(
                block.span,
                format!("unknown cardinality `{other}`"),
            ));
        }
    })
}
