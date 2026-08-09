use pest::error::InputLocation;

use crate::error::BsolError;
use crate::parser::Rule;

use super::model::BsolSpan;

pub(super) fn span_at(source: &str, start: usize, end: usize) -> BsolSpan {
    let start = start.min(source.len());
    let end = end.max(start.saturating_add(1)).min(source.len());
    let pest_span = pest::Span::new(source, start, end)
        .or_else(|| pest::Span::new(source, start, start.saturating_add(1).min(source.len())))
        .expect("span bounds");
    BsolSpan::from_pest(pest_span, source)
}

pub(super) fn offset_span(source: &str, offset: usize, span: pest::Span<'_>) -> BsolSpan {
    span_at(source, offset + span.start(), offset + span.end())
}

pub(super) fn pest_error_with_offset(
    source: &str,
    offset: usize,
    err: pest::error::Error<Rule>,
) -> BsolError {
    let start = match err.location {
        InputLocation::Pos(pos) => offset + pos,
        InputLocation::Span((start, _)) => offset + start,
    };
    let line = source[..start.min(source.len())].lines().count().max(1);
    BsolError::ParseAt {
        line,
        message: err.to_string(),
        start: Some(start),
        end: source.get(start..).map(|tail| start + tail.len().min(1)),
    }
}
