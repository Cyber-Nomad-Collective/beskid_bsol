//! Validate a parsed document against a schema profile.

mod block;
mod coercion;
mod constraints;
mod context;
mod document;
mod field;
mod formatting;
mod model;
mod pattern;

pub use document::{validate, validate_with, validate_with_pattern_limits};

#[allow(non_snake_case)]
#[derive(Debug, Clone, Copy)]
pub struct PatternLimits {
    pub maxSubjectBytes: usize,
    pub maxWork: usize,
    pub maxDepth: usize,
}
impl Default for PatternLimits {
    fn default() -> Self {
        Self {
            maxSubjectBytes: 1048576,
            maxWork: 16777216,
            maxDepth: 64,
        }
    }
}
impl PatternLimits {
    pub(super) fn valid(&self) -> bool {
        self.maxSubjectBytes <= isize::MAX as usize
            && self.maxWork <= isize::MAX as usize
            && self.maxDepth > 0
            && self.maxDepth <= 64
    }
}
pub use model::{ValidatedBlock, ValidatedDocument};

#[cfg(test)]
mod tests;
