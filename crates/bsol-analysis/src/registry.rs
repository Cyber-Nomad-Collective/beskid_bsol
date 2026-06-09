//! Custom semantic validator registration.

use std::collections::HashMap;

use crate::validate::ValidatedBlock;
use crate::BsolError;

type ValidatorFn = Box<dyn Fn(&ValidatedBlock) -> Result<(), BsolError> + Send + Sync>;

/// Registry of per-rule semantic validators run after structural validation.
#[derive(Default)]
pub struct ValidatorRegistry {
    by_rule: HashMap<String, Vec<ValidatorFn>>,
}

impl ValidatorRegistry {
    pub fn on_rule(
        &mut self,
        rule_id: &str,
        validator: impl Fn(&ValidatedBlock) -> Result<(), BsolError> + Send + Sync + 'static,
    ) {
        self.by_rule
            .entry(rule_id.to_string())
            .or_default()
            .push(Box::new(validator));
    }

    pub fn run(&self, block: &ValidatedBlock) -> Result<(), BsolError> {
        if let Some(validators) = self.by_rule.get(&block.rule_id) {
            for validator in validators {
                validator(block)?;
            }
        }
        Ok(())
    }
}
