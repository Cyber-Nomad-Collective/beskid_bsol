//! Cross-block reference resolution (schema.semantic phase).

use std::collections::HashMap;

use bsol_syntax::BsolRef;

use crate::value::ValidatedValue;
use crate::{BsolError, ValidatedBlock, ValidatedDocument};

/// Resolve `@kind/label` references against top-level block labels.
pub fn resolve_references(document: &mut ValidatedDocument) -> Result<(), BsolError> {
    let index = build_ref_index(document);
    for block in &mut document.blocks {
        resolve_block_refs(block, &index)?;
    }
    Ok(())
}

fn build_ref_index(document: &ValidatedDocument) -> HashMap<(String, String), ()> {
    let mut index = HashMap::new();
    for block in &document.blocks {
        if let Some(label) = &block.label {
            index.insert((block.rule_id.clone(), label.clone()), ());
            index.insert((block.kind.clone(), label.clone()), ());
        }
    }
    index
}

fn resolve_block_refs(
    block: &mut ValidatedBlock,
    index: &HashMap<(String, String), ()>,
) -> Result<(), BsolError> {
    for value in block.values.values_mut() {
        resolve_value_refs(value, index, block.span)?;
    }
    for list in block.lists.values() {
        let _ = list;
    }
    for nested in &mut block.nested {
        resolve_block_refs(nested, index)?;
    }
    Ok(())
}

fn resolve_value_refs(
    value: &mut ValidatedValue,
    index: &HashMap<(String, String), ()>,
    span: bsol_syntax::BsolSpan,
) -> Result<(), BsolError> {
    match value {
        ValidatedValue::Ref(reference) => {
            ensure_ref_resolves(reference, index, span)?;
        }
        ValidatedValue::List(items) => {
            for item in items {
                resolve_value_refs(item, index, span)?;
            }
        }
        ValidatedValue::Map(map) => {
            for item in map.values_mut() {
                resolve_value_refs(item, index, span)?;
            }
        }
        ValidatedValue::Block(block) => {
            for item in block.values.values_mut() {
                resolve_value_refs(item, index, span)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn ensure_ref_resolves(
    reference: &BsolRef,
    index: &HashMap<(String, String), ()>,
    _span: bsol_syntax::BsolSpan,
) -> Result<(), BsolError> {
    let candidates = if let Some(kind) = &reference.rule_kind {
        vec![(kind.clone(), reference.label.clone())]
    } else {
        index
            .keys()
            .filter(|(_, label)| label == &reference.label)
            .cloned()
            .collect()
    };
    if candidates.iter().any(|key| index.contains_key(key)) {
        Ok(())
    } else {
        Err(BsolError::schema_at(
            reference.span,
            format!("unresolved reference `{}`", reference.display()),
        ))
    }
}
