use std::collections::HashMap;

use bsol_syntax::{BsolBlock, BsolItem, BsolValue};

use super::value_decode::{assignment_string, value_as_string};
use crate::{BsolError, MigrationRewrite, MigrationSpec, MigrationWhenClause};

pub(super) fn parse_migration_block(block: &BsolBlock) -> Result<MigrationSpec, BsolError> {
    let from = assignment_string(block, "from")?
        .ok_or_else(|| BsolError::schema_at(block.span, "`migration` requires `from` profile"))?;
    let mut detect = HashMap::new();
    let mut when_clauses = Vec::new();
    let mut rewrites = Vec::new();
    for item in &block.items {
        let BsolItem::Block(nested) = item else {
            continue;
        };
        match nested.kind.as_str() {
            "detect" => {
                for entry in &nested.items {
                    let BsolItem::Assignment(a) = entry else {
                        continue;
                    };
                    detect.insert(a.key.clone(), value_as_string(&a.value)?);
                }
            }
            "when" => when_clauses.push(parse_migration_when(nested)?),
            "rewrite" => rewrites.extend(parse_migration_rewrites(nested)?),
            _ => {}
        }
    }
    Ok(MigrationSpec {
        from,
        detect,
        when_clauses,
        rewrites,
        span: block.span,
    })
}

fn parse_migration_when(block: &BsolBlock) -> Result<MigrationWhenClause, BsolError> {
    Ok(MigrationWhenClause {
        block_kind: assignment_string(block, "block")?,
        field: assignment_string(block, "field")?,
        field_value: assignment_string(block, "field_value")?,
        missing_field: assignment_string(block, "missing_field")?,
    })
}

fn parse_migration_rewrites(block: &BsolBlock) -> Result<Vec<MigrationRewrite>, BsolError> {
    let mut rewrites = Vec::new();
    for item in &block.items {
        match item {
            BsolItem::Assignment(a) if a.key == "add_field" => {
                if let BsolValue::InlineMap(map) = &a.value {
                    for entry in &map.entries {
                        rewrites.push(MigrationRewrite::AddField {
                            key: entry.key.clone(),
                            value: value_as_string(&entry.value)?,
                        });
                    }
                }
            }
            BsolItem::Assignment(a) if a.key == "rename_field" => {
                if let BsolValue::InlineMap(map) = &a.value {
                    for entry in &map.entries {
                        rewrites.push(MigrationRewrite::RenameField {
                            from: entry.key.clone(),
                            to: value_as_string(&entry.value)?,
                        });
                    }
                }
            }
            BsolItem::Assignment(a) if a.key == "replace_value" => {
                let from = assignment_string_in_block(block, "from")?;
                let to = assignment_string_in_block(block, "to")?;
                if let (Some(from), Some(to)) = (from, to) {
                    rewrites.push(MigrationRewrite::ReplaceValue { from, to });
                }
            }
            BsolItem::Block(nested) if nested.kind == "replace_value" => {
                let from = assignment_string(nested, "from")?;
                let to = assignment_string(nested, "to")?;
                if let (Some(from), Some(to)) = (from, to) {
                    rewrites.push(MigrationRewrite::ReplaceValue { from, to });
                }
            }
            _ => {}
        }
    }
    Ok(rewrites)
}

fn assignment_string_in_block(block: &BsolBlock, key: &str) -> Result<Option<String>, BsolError> {
    assignment_string(block, key)
}
