use crate::{BsolError, value::ValidatedValue};

pub(super) fn apply_numeric_constraints(
    constraints: &bsol_schema::FieldConstraints,
    value: i64,
    span: bsol_syntax::BsolSpan,
) -> Result<(), BsolError> {
    if let Some(min) = constraints.min {
        if value < min {
            return Err(BsolError::schema_at(
                span,
                format!("value {value} below min {min}"),
            ));
        }
    }
    if let Some(max) = constraints.max {
        if value > max {
            return Err(BsolError::schema_at(
                span,
                format!("value {value} above max {max}"),
            ));
        }
    }
    Ok(())
}

pub(super) fn apply_string_constraints(
    constraints: &bsol_schema::FieldConstraints,
    value: &ValidatedValue,
    span: bsol_syntax::BsolSpan,
) -> Result<(), BsolError> {
    let Some(text) = value.as_string() else {
        return Ok(());
    };
    if let Some(pattern) = &constraints.pattern {
        if !simple_pattern_match(pattern, &text) {
            return Err(BsolError::schema_at(
                span,
                format!("value `{text}` does not match pattern `{pattern}`"),
            ));
        }
    }
    Ok(())
}

fn simple_pattern_match(pattern: &str, text: &str) -> bool {
    if pattern.starts_with('^') && pattern.ends_with('$') {
        let inner = &pattern[1..pattern.len().saturating_sub(1)];
        if inner.contains("[0-9]") {
            return !text.is_empty()
                && text
                    .chars()
                    .all(|c| c.is_ascii_digit() || c == '.' || c == '-');
        }
    }
    text.contains(pattern.trim_matches('^').trim_matches('$'))
}
