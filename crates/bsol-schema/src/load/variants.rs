use bsol_syntax::BsolBlock;

use super::value_decode::{assignment_list, assignment_string};
use crate::{BsolError, VariantRule};

pub(super) fn parse_variant_rule(block: &BsolBlock) -> Result<VariantRule, BsolError> {
    let name = block
        .label
        .as_ref()
        .map(|q| q.value.clone())
        .or_else(|| assignment_string(block, "name").ok().flatten())
        .ok_or_else(|| BsolError::schema_at(block.span, "`variant` block requires a name label"))?;
    Ok(VariantRule {
        name,
        require: assignment_list(block, "require").unwrap_or_default(),
        forbid: assignment_list(block, "forbid").unwrap_or_default(),
    })
}
