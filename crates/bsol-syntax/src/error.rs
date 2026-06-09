//! Parse errors for BSOL documents.

use thiserror::Error;

use crate::ast::BsolSpan;

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
            Self::ParseAt { line, .. } => Some(*line),
            _ => None,
        }
    }
}
