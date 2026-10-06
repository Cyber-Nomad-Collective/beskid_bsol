use std::collections::HashMap;

use bsol_schema::{BlockRule, Cardinality, SchemaProfile};
use bsol_syntax::BsolDocument;

use super::{
    block::validate_block, context::ValidationContext, model::ValidatedDocument, PatternLimits,
};
use crate::{registry::ValidatorRegistry, BsolError};

/// Validate `document` against `profile`.
pub fn validate(
    document: &BsolDocument,
    profile: &SchemaProfile,
) -> Result<ValidatedDocument, BsolError> {
    validate_with(document, profile, &ValidatorRegistry::default())
}

/// Validate with optional custom semantic validators.
pub fn validate_with(
    document: &BsolDocument,
    profile: &SchemaProfile,
    registry: &ValidatorRegistry,
) -> Result<ValidatedDocument, BsolError> {
    validate_internal(document, profile, registry, &PatternLimits::default())
}

pub fn validate_with_pattern_limits(
    document: &BsolDocument,
    profile: &SchemaProfile,
    limits: &PatternLimits,
) -> Result<ValidatedDocument, BsolError> {
    validate_internal(document, profile, &ValidatorRegistry::default(), limits)
}

fn validate_internal(
    document: &BsolDocument,
    profile: &SchemaProfile,
    registry: &ValidatorRegistry,
    limits: &PatternLimits,
) -> Result<ValidatedDocument, BsolError> {
    if !limits.valid() {
        return Err(document.blocks.first().map_or_else(
            || BsolError::Schema("InvalidPatternLimits".into()),
            |block| BsolError::schema_at(block.span, "InvalidPatternLimits"),
        ));
    }
    let mut context = ValidationContext::new(profile, registry, *limits);
    let mut validated_top = Vec::new();
    let mut rule_counts: HashMap<String, usize> = HashMap::new();

    for block in &document.blocks {
        let rule = match_top_level_rule(profile, &block.kind).ok_or_else(|| {
            BsolError::schema_at(
                block.span,
                format!("unknown top-level block `{kind}`", kind = block.kind),
            )
        })?;
        let validated = validate_block(block, rule, &mut context)?;
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
    profile
        .top_level_rules()
        .find(|rule| rule.matches_kind(kind))
}
