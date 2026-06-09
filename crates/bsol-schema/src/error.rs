//! Schema and validation errors for BSOL documents.

use thiserror::Error;

use bsol_syntax::{BsolError as ParseError, BsolSpan};

/// Failure while parsing, loading, or validating a BSOL document.
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
    #[error("BSOL schema error at line {line}: {message}")]
    SchemaAt {
        line: usize,
        message: String,
        start: Option<usize>,
        end: Option<usize>,
    },
    #[error("BSOL schema error: {0}")]
    Schema(String),
    #[error("unknown schema profile `{0}`")]
    UnknownProfile(String),
    #[error("schema import error: {0}")]
    Import(String),
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

    pub fn schema_at(span: BsolSpan, message: impl Into<String>) -> Self {
        Self::SchemaAt {
            line: span.line,
            message: message.into(),
            start: Some(span.start),
            end: Some(span.end),
        }
    }

    pub fn manifest_source_span(&self) -> Option<(usize, usize)> {
        match self {
            Self::ParseAt { start, end, .. } | Self::SchemaAt { start, end, .. } => {
                match (start, end) {
                    (Some(s), Some(e)) if *e > *s => Some((*s, *e)),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    pub fn manifest_source_line(&self) -> Option<usize> {
        match self {
            Self::ParseAt { line, .. } | Self::SchemaAt { line, .. } => Some(*line),
            _ => None,
        }
    }
}

impl From<ParseError> for BsolError {
    fn from(value: ParseError) -> Self {
        match value {
            ParseError::ParseAt {
                line,
                message,
                start,
                end,
            } => Self::ParseAt {
                line,
                message,
                start,
                end,
            },
            ParseError::Parse(message) => Self::Parse(message),
        }
    }
}
