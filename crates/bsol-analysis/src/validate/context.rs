//! One bounded validation authority for ordinary, nested and inline blocks.
use super::{pattern::Budget, PatternLimits};
use crate::{registry::ValidatorRegistry, BsolError};
use bsol_schema::{BlockRule, SchemaProfile};
use bsol_syntax::BsolSpan;

pub(super) struct ValidationContext<'a> {
    pub profile: &'a SchemaProfile,
    pub registry: &'a ValidatorRegistry,
    depth: usize,
    remaining: usize,
    pub patterns: Budget,
}
impl<'a> ValidationContext<'a> {
    pub fn new(
        profile: &'a SchemaProfile,
        registry: &'a ValidatorRegistry,
        limits: PatternLimits,
    ) -> Self {
        Self {
            profile,
            registry,
            depth: 0,
            remaining: 100_000,
            patterns: Budget::new(limits),
        }
    }
    pub fn charge(&mut self, span: BsolSpan) -> Result<(), BsolError> {
        if self.remaining == 0 {
            return Err(BsolError::schema_at(
                span,
                "schema validation work limit exceeded",
            ));
        }
        self.remaining -= 1;
        Ok(())
    }
    pub fn enter(&mut self, span: BsolSpan) -> Result<(), BsolError> {
        self.charge(span)?;
        if self.depth >= 64 {
            return Err(BsolError::schema_at(
                span,
                "schema validation depth limit exceeded",
            ));
        }
        self.depth += 1;
        Ok(())
    }
    pub fn leave(&mut self) {
        self.depth -= 1;
    }
    pub fn inline_rule(
        &self,
        scope: &BlockRule,
        name: &str,
        span: BsolSpan,
    ) -> Result<BlockRule, BsolError> {
        scope
            .nested
            .get(name)
            .or_else(|| self.profile.rule(name))
            .cloned()
            .ok_or_else(|| BsolError::schema_at(span, format!("unknown inline rule `{name}`")))
    }
}
