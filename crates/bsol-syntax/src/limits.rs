//! Syntax budgets shared with native `Core.Bsol.Limits` / `Core.Bsol.Parser`.
//!
//! One descriptor carries the spec default budgets (`tooling--manifests-and-lockfiles--bsol`):
//! the parser enforces input, nesting, node and scalar budgets; the writer enforces output;
//! the schema import graph enforces import count, import depth and resolved schema bytes.

use std::fmt;

/// Default `maxInputBytes`: 8 MiB.
pub const DEFAULT_MAX_INPUT_BYTES: usize = 8_388_608;
/// Default `maxDepth`.
pub const DEFAULT_MAX_DEPTH: usize = 128;
/// Default `maxNodes` (syntax nodes plus comment trivia).
pub const DEFAULT_MAX_NODES: usize = 100_000;
/// Default `maxScalarBytes`: 1 MiB per identifier, number, type token or decoded string.
pub const DEFAULT_MAX_SCALAR_BYTES: usize = 1_048_576;
/// Default `maxOutputBytes`: 8 MiB.
pub const DEFAULT_MAX_OUTPUT_BYTES: usize = 8_388_608;
/// Default `maxImports`: aggregate `import_schema` declarations in one resolved closure.
pub const DEFAULT_MAX_IMPORTS: usize = 256;
/// Default `maxImportDepth`.
pub const DEFAULT_MAX_IMPORT_DEPTH: usize = 32;
/// Default `maxResolvedSchemaBytes`: 8 MiB of distinct schema source in one closure.
pub const DEFAULT_MAX_RESOLVED_SCHEMA_BYTES: usize = 8_388_608;

/// Checked syntax budgets; `Default` is the normative default policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BsolLimits {
    pub max_input_bytes: usize,
    pub max_depth: usize,
    pub max_nodes: usize,
    pub max_scalar_bytes: usize,
    pub max_output_bytes: usize,
    pub max_imports: usize,
    pub max_import_depth: usize,
    pub max_resolved_schema_bytes: usize,
}

impl Default for BsolLimits {
    fn default() -> Self {
        Self {
            max_input_bytes: DEFAULT_MAX_INPUT_BYTES,
            max_depth: DEFAULT_MAX_DEPTH,
            max_nodes: DEFAULT_MAX_NODES,
            max_scalar_bytes: DEFAULT_MAX_SCALAR_BYTES,
            max_output_bytes: DEFAULT_MAX_OUTPUT_BYTES,
            max_imports: DEFAULT_MAX_IMPORTS,
            max_import_depth: DEFAULT_MAX_IMPORT_DEPTH,
            max_resolved_schema_bytes: DEFAULT_MAX_RESOLVED_SCHEMA_BYTES,
        }
    }
}

impl BsolLimits {
    /// Native `Limits.Valid`: depth and nodes admit at least one root (byte and count budgets
    /// are unsigned here, so their native non-negativity holds by construction).
    pub fn is_valid(&self) -> bool {
        self.max_depth >= 1 && self.max_nodes >= 1
    }

    /// Native `Limits.OutputParseLimits`: written output is reparsed with the output budget as
    /// its input budget.
    pub fn output_parse_limits(&self) -> Self {
        Self {
            max_input_bytes: self.max_output_bytes,
            ..*self
        }
    }
}

/// Native `Core.Bsol.Errors.ErrorKind` values a limit check can report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BsolLimitKind {
    InputLimit,
    OutputLimit,
    DepthLimit,
    NodeLimit,
    ScalarLimit,
    /// Native reports an invalid limit descriptor ("invalid parser limits") and a writer graph
    /// over a non-output budget ("invalid or over-limit syntax graph") as `InvalidSyntax`.
    InvalidSyntax,
}

impl BsolLimitKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InputLimit => "InputLimit",
            Self::OutputLimit => "OutputLimit",
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
