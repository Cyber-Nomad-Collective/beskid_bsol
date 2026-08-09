use std::{collections::HashMap, path::Path};

use bsol_syntax::{BsolDocument, BsolItem, parse_bsol_document};

use super::{
    imports_extends::{parse_extend_block, parse_import_schema},
    migrations::parse_migration_block,
    rules::parse_block_rule,
    value_decode::assignment_string,
};
use crate::{BsolError, RuleScope, SchemaProfile};

/// Parse and load a profile from BSOL source text.
pub fn load_profile_from_source(source: &str) -> Result<SchemaProfile, BsolError> {
    let document = parse_bsol_document(source).map_err(BsolError::from)?;
    load_profile_from_document(&document)
}

/// Parse and load a profile from an already-parsed document.
pub fn load_profile_from_document(document: &BsolDocument) -> Result<SchemaProfile, BsolError> {
    let profile = parse_profile_document(document)?;
    crate::compose::compose_profile(profile)
}

/// Load a profile from a filesystem path.
pub fn load_profile_from_path(path: &Path) -> Result<SchemaProfile, BsolError> {
    let source = std::fs::read_to_string(path)
        .map_err(|e| BsolError::Import(format!("failed to read `{}`: {e}", path.display())))?;
    load_profile_from_source(&source)
}

pub fn parse_profile_document(document: &BsolDocument) -> Result<SchemaProfile, BsolError> {
    let profile_block = document
        .blocks
        .iter()
        .find(|b| b.kind == "profile")
        .ok_or_else(|| {
            BsolError::Schema("schema profile document must contain a `profile` block".into())
        })?;
    let name = profile_block
        .label
        .as_ref()
        .map(|q| q.value.clone())
        .ok_or_else(|| {
            BsolError::schema_at(
                profile_block.span,
                "`profile` block must carry a quoted label (profile name)",
            )
        })?;

    let version = assignment_string(profile_block, "version")?
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);
    let mut imports = Vec::new();
    let mut extends = Vec::new();
    let mut migrations = Vec::new();
    let mut rules = HashMap::new();
    let mut top_level_order = Vec::new();
    for item in &profile_block.items {
        let BsolItem::Block(rule_block) = item else {
            continue;
        };
        match rule_block.kind.as_str() {
            "import_schema" => imports.push(parse_import_schema(rule_block)?),
            "extend" => extends.push(parse_extend_block(rule_block)?),
            "migration" => migrations.push(parse_migration_block(rule_block)?),
            "rule" => {
                let rule_id = rule_block
                    .label
                    .as_ref()
                    .map(|q| q.value.clone())
                    .or_else(|| assignment_string(rule_block, "id").ok().flatten())
                    .ok_or_else(|| {
                        BsolError::schema_at(
                            rule_block.span,
                            "rule block requires a quoted label or `id`",
                        )
                    })?;
                let rule = parse_block_rule(rule_block, RuleScope::TopLevel)?;
                if rule.scope == RuleScope::TopLevel {
                    top_level_order.push(rule_id.clone());
                }
                rules.insert(rule_id, rule);
            }
            other => {
                return Err(BsolError::schema_at(
                    rule_block.span,
                    format!("unexpected item `{other}` inside profile"),
                ));
            }
        }
    }

    Ok(SchemaProfile {
        name,
        version,
        rules,
        top_level_order,
        imports,
        extends,
        migrations,
    })
}
