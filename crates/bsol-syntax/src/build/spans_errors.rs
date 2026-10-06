use super::parse_source::ParseSource;
use pest::error::InputLocation;

use crate::error::BsolError;
use crate::parser::Rule;

use super::model::BsolSpan;

pub(super) fn span_at(source: &ParseSource<'_>, start: usize, end: usize) -> BsolSpan {
    source.span(start, end)
}

pub(super) fn offset_span(
    source: &ParseSource<'_>,
    offset: usize,
    span: pest::Span<'_>,
) -> BsolSpan {
    span_at(source, offset + span.start(), offset + span.end())
}

pub(super) fn pest_error_with_offset(
    source: &ParseSource<'_>,
    offset: usize,
    err: pest::error::Error<Rule>,
) -> BsolError {
    let start = match err.location {
        InputLocation::Pos(pos) => offset + pos,
        InputLocation::Span((start, _)) => offset + start,
    };
    let span = span_at(source, start, start.saturating_add(1));
    BsolError::ParseAt {
        line: span.line,
        message: err.to_string(),
        start: Some(span.start),
        end: Some(span.end),
    }
}
