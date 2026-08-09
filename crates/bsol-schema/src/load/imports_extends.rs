use std::collections::HashMap;

use bsol_syntax::{BsolBlock, BsolItem};

use super::{rules::parse_block_rule, value_decode::assignment_string};
use crate::{BsolError, ExtendSpec, ImportSchemaSpec, ImportSource, RuleScope};

pub(super) fn parse_import_schema(block: &BsolBlock) -> Result<ImportSchemaSpec, BsolError> {
    let name = block
        .label
        .as_ref()
        .map(|q| q.value.clone())
        .ok_or_else(|| {
            BsolError::schema_at(
                block.span,
                "`import_schema` block requires a quoted profile name label",
            )
        })?;
    let alias = assignment_string(block, "alias")?;
    let from = assignment_string(block, "from")?
        .ok_or_else(|| BsolError::schema_at(block.span, "`import_schema` requires `from`"))?;

    let source = if from.starts_with("@pckg/") {
        ImportSource::PckgShorthand {
            reference: from.clone(),
        }
    } else {
        match from.as_str() {
            "file" => ImportSource::File {
                path: assignment_string(block, "path")?.ok_or_else(|| {
                    BsolError::schema_at(block.span, "`from = file` requires `path`")
                })?,
            },
            "git" => ImportSource::Git {
                url: assignment_string(block, "url")?.ok_or_else(|| {
                    BsolError::schema_at(block.span, "`from = git` requires `url`")
                })?,
                rev: assignment_string(block, "rev")?.ok_or_else(|| {
                    BsolError::schema_at(block.span, "`from = git` requires `rev`")
                })?,
                path: assignment_string(block, "path")?.ok_or_else(|| {
                    BsolError::schema_at(block.span, "`from = git` requires `path`")
                })?,
            },
            "registry" => ImportSource::Registry {
                package: assignment_string(block, "package")?.ok_or_else(|| {
                    BsolError::schema_at(block.span, "`from = registry` requires `package`")
                })?,
                version: assignment_string(block, "version")?.ok_or_else(|| {
                    BsolError::schema_at(block.span, "`from = registry` requires `version`")
                })?,
                path: assignment_string(block, "path")?.ok_or_else(|| {
                    BsolError::schema_at(block.span, "`from = registry` requires `path`")
                })?,
            },
            other => {
                return Err(BsolError::schema_at(
                    block.span,
                    format!("unknown import source `{other}`"),
                ));
            }
        }
    };

    Ok(ImportSchemaSpec {
        name,
        alias,
        from: source,
        span: block.span,
    })
}

pub(super) fn parse_extend_block(block: &BsolBlock) -> Result<ExtendSpec, BsolError> {
    let base = block
        .label
        .as_ref()
        .map(|q| q.value.clone())
        .or_else(|| assignment_string(block, "base").ok().flatten())
        .ok_or_else(|| {
            BsolError::schema_at(block.span, "`extend` block requires a base profile label")
        })?;
    let mut rules = HashMap::new();
    for item in &block.items {
        let BsolItem::Block(rule_block) = item else {
            continue;
        };
        if rule_block.kind != "rule" {
            continue;
        }
        let rule_id = rule_block
            .label
            .as_ref()
            .map(|q| q.value.clone())
            .ok_or_else(|| BsolError::schema_at(rule_block.span, "extend rule requires label"))?;
        rules.insert(rule_id, parse_block_rule(rule_block, RuleScope::TopLevel)?);
    }
    Ok(ExtendSpec {
        base,
        rules,
        span: block.span,
    })
}
