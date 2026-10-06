use crate::{value::ValidatedValue, BsolError};

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
    context: &mut super::context::ValidationContext<'_>,
) -> Result<(), BsolError> {
    let Some(text) = value.as_string() else {
        return Ok(());
    };
    if let Some(pattern) = &constraints.pattern {
        let matched = super::pattern::matches(pattern, &text, &mut context.patterns)
            .map_err(|code| BsolError::schema_at(span, code))?;
        if !matched {
            return Err(BsolError::schema_at(span, "PatternMismatch"));
        }
    }
    Ok(())
}
