//! Syntax budgets shared with native `Core.Bsol.Limits` / `Core.Bsol.Parser`.
//!
//! The parser enforces the input, nesting, node and scalar budgets of the spec defaults
//! (`tooling--manifests-and-lockfiles--bsol`). Output, import, import-depth and resolved-schema
//! budgets belong to writers and schema resolvers, not to syntax parsing.

use std::fmt;

/// Default `maxInputBytes`: 8 MiB.
pub const DEFAULT_MAX_INPUT_BYTES: usize = 8_388_608;
/// Default `maxDepth`.
pub const DEFAULT_MAX_DEPTH: usize = 128;
/// Default `maxNodes` (syntax nodes plus comment trivia).
pub const DEFAULT_MAX_NODES: usize = 100_000;
/// Default `maxScalarBytes`: 1 MiB per identifier, number, type token or decoded string.
pub const DEFAULT_MAX_SCALAR_BYTES: usize = 1_048_576;

/// Checked syntax budgets; `Default` is the normative default policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BsolLimits {
    pub max_input_bytes: usize,
    pub max_depth: usize,
    pub max_nodes: usize,
    pub max_scalar_bytes: usize,
}

impl Default for BsolLimits {
    fn default() -> Self {
        Self {
            max_input_bytes: DEFAULT_MAX_INPUT_BYTES,
            max_depth: DEFAULT_MAX_DEPTH,
            max_nodes: DEFAULT_MAX_NODES,
            max_scalar_bytes: DEFAULT_MAX_SCALAR_BYTES,
        }
    }
}

impl BsolLimits {
    /// Native `Limits.Valid` for the syntax budgets: depth and nodes admit at least one root.
    pub fn is_valid(&self) -> bool {
        self.max_depth >= 1 && self.max_nodes >= 1
    }
}

/// Native `Core.Bsol.Errors.ErrorKind` values a limit check can report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BsolLimitKind {
    InputLimit,
    DepthLimit,
    NodeLimit,
    ScalarLimit,
    /// Native reports an invalid limit descriptor as `InvalidSyntax` ("invalid parser limits").
    InvalidSyntax,
}

impl BsolLimitKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InputLimit => "InputLimit",
            Self::DepthLimit => "DepthLimit",
            Self::NodeLimit => "NodeLimit",
            Self::ScalarLimit => "ScalarLimit",
            Self::InvalidSyntax => "InvalidSyntax",
        }
    }
}

impl fmt::Display for BsolLimitKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
