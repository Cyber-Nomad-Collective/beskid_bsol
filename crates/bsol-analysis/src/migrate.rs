//! Author-defined migration patterns between BSOL profile versions.

use bsol_schema::{
    load_profile, MigrationRewrite, MigrationSpec, MigrationWhenClause, SchemaProfile,
};
use bsol_syntax::{parse_bsol_document, BsolDocument, BsolItem};

use crate::{validate_with, BsolError, ValidatedDocument, ValidatorRegistry};

/// Planned migration route from source profile to target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationPlan {
    pub from_profile: String,
    pub to_profile: String,
    pub rewrites: Vec<MigrationRewrite>,
}

/// One routing strategy inside a migration plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MigrationRoute {
    Heuristic { detect: Vec<(String, String)> },
    When(MigrationWhenClause),
}

/// Select a migration plan for `source` text targeting `target_profile`.
pub fn plan_migration(
    document: &BsolDocument,
    source: &str,
    target: &SchemaProfile,
) -> Result<Option<MigrationPlan>, BsolError> {
    for migration in &target.migrations {
        if route_matches(document, source, migration)? {
            return Ok(Some(MigrationPlan {
                from_profile: migration.from.clone(),
                to_profile: target.name.clone(),
                rewrites: migration.rewrites.clone(),
            }));
        }
    }
    Ok(None)
}

fn route_matches(
    document: &BsolDocument,
    source: &str,
    migration: &MigrationSpec,
) -> Result<bool, BsolError> {
    if migration.detect.get("profile_version_missing") == Some(&"true".to_string())
        && !source.contains("profile_version = ")
        && !source.contains("version = 2")
    {
        return Ok(true);
    }
    for when in &migration.when_clauses {
        if when_clause_matches(document, when) {
            return Ok(true);
        }
    }
    Ok(false)
}

fn when_clause_matches(document: &BsolDocument, when: &MigrationWhenClause) -> bool {
    for block in &document.blocks {
        if let Some(expected) = &when.block_kind {
            if &block.kind != expected && block.kind != *expected {
                continue;
            }
        }
        let mut field_values: Vec<(String, String)> = Vec::new();
        for item in &block.items {
            if let BsolItem::Assignment(a) = item {
                if let Some(text) = crate::value::raw_to_string(&a.value) {
                    field_values.push((a.key.clone(), text));
                }
            }
        }
        if let Some(field) = &when.field {
            if let Some(expected) = &when.field_value {
                if !field_values
                    .iter()
                    .any(|(k, v)| k == field && v == expected)
                {
                    continue;
                }
            }
            if let Some(missing) = &when.missing_field {
                if field_values.iter().any(|(k, _)| k == missing) {
                    continue;
                }
                if field != missing {
                    continue;
                }
            }
        }
        return true;
    }
    false
}

/// Apply rewrite operations to source text.
pub fn apply_migration(source: &str, plan: &MigrationPlan) -> Result<String, BsolError> {
    let mut output = source.to_string();
    for rewrite in &plan.rewrites {
        match rewrite {
            MigrationRewrite::AddField { key, value } => {
                if !output.contains(&format!("{key} =")) {
                    if let Some(pos) = output.rfind('}') {
                        output.insert_str(pos, &format!("\n  {key} = \"{value}\""));
                    }
                }
            }
            MigrationRewrite::RenameField { from, to } => {
                output = output.replace(&format!("{from} ="), &format!("{to} ="));
            }
            MigrationRewrite::ReplaceValue { from, to } => {
                output = output.replace(from, to);
            }
        }
    }
    Ok(output)
}

/// Migrate, parse, and validate against the target profile.
pub fn migrate_document(
    source: &str,
    target_profile_name: &str,
) -> Result<(String, ValidatedDocument), BsolError> {
    let target = load_profile(target_profile_name)?;
    let document = parse_bsol_document(source).map_err(BsolError::from)?;
    let plan = plan_migration(&document, source, &target)?
        .ok_or_else(|| BsolError::Schema("no migration route matched".into()))?;
    let migrated = apply_migration(source, &plan)?;
    let migrated_doc = parse_bsol_document(&migrated).map_err(BsolError::from)?;
    let mut validated = validate_with(&migrated_doc, &target, &ValidatorRegistry::default())?;
    crate::semantic::resolve_references(&mut validated)?;
    Ok((migrated, validated))
}
