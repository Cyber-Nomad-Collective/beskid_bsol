//! Parse errors for BSOL documents.

use thiserror::Error;

use crate::ast::BsolSpan;
use crate::limits::BsolLimitKind;

/// Failure while parsing a BSOL document.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum BsolError {
    #[error("BSOL parse error at line {line}: {message}")]
    ParseAt {
        line: usize,
        message: String,
        start: Option<usize>,
        end: Option<usize>,
    },
    #[error("BSOL parse error: {0}")]
    Parse(String),
    /// A syntax budget was exceeded; `kind` and `message` match the native parser diagnostic.
    #[error("BSOL {kind} at line {line}: {message}")]
    Limit {
        kind: BsolLimitKind,
        message: &'static str,
        line: usize,
        offset: usize,
    },
}

impl BsolError {
    pub fn parse_at(span: BsolSpan, message: impl Into<String>) -> Self {
        Self::ParseAt {
            line: span.line,
            message: message.into(),
            start: Some(span.start),
            end: Some(span.end),
        }
    }

    pub fn limit(kind: BsolLimitKind, message: &'static str, line: usize, offset: usize) -> Self {
        Self::Limit {
            kind,
            message,
            line,
            offset,
        }
    }

    /// The exceeded budget, when this is a limit diagnostic.
    pub fn limit_kind(&self) -> Option<BsolLimitKind> {
        match self {
            Self::Limit { kind, .. } => Some(*kind),
            _ => None,
        }
    }

    pub fn source_span(&self) -> Option<(usize, usize)> {
        match self {
            Self::ParseAt { start, end, .. } => match (start, end) {
                (Some(s), Some(e)) if *e > *s => Some((*s, *e)),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn source_line(&self) -> Option<usize> {
        match self {
            Self::ParseAt { line, .. } | Self::Limit { line, .. } => Some(*line),
            _ => None,
        }
    }
}
