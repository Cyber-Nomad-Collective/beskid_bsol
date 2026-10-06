//! Exact shipped Core.Text.Regex language, parsed before bounded matching.
use pest::{iterators::Pair, Parser};
use pest_derive::Parser;
#[derive(Parser)]
#[grammar = "regex.pest"]
struct PatternParser;

#[derive(Debug, Clone)]
pub enum Atom {
    Literal(char),
    Any,
    Digit(bool),
    Word(bool),
    Space(bool),
    Class {
        negated: bool,
        ranges: Vec<(char, char)>,
    },
}
#[derive(Debug, Clone)]
pub enum Expression {
    Atom(Atom),
    Start,
    End,
    Sequence(Vec<Expression>),
    Alternative(Vec<Expression>),
    Repeat {
        inner: Box<Expression>,
        minimum: usize,
        maximum: Option<usize>,
    },
}
#[derive(Debug, Clone, Copy)]
pub enum PatternError {
    InvalidPattern,
    PatternDepthLimit,
}
impl std::fmt::Display for PatternError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
/// Reject excessive grouping before the recursive generated parser runs.
pub fn parse(pattern: &str, max_depth: usize) -> Result<Expression, PatternError> {
    let mut depth = 0usize;
    let mut class = false;
    let mut escaped = false;
    for ch in pattern.chars() {
        if class {
            if ch == ']' {
                class = false;
            }
            continue;
        }

        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        match ch {
            '[' => class = true,
            '(' => {
                depth = depth
                    .checked_add(1)
                    .ok_or(PatternError::PatternDepthLimit)?;
                if depth > max_depth {
                    return Err(PatternError::PatternDepthLimit);
                }
            }
            ')' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    let mut pairs = PatternParser::parse(Rule::full_pattern, pattern)
        .map_err(|_| PatternError::InvalidPattern)?;
    let root = pairs.next().ok_or(PatternError::InvalidPattern)?;
    let pat = root
        .into_inner()
        .next()
        .ok_or(PatternError::InvalidPattern)?;
    expression(pat)
}
fn expression(pair: Pair<'_, Rule>) -> Result<Expression, PatternError> {
    let rule = pair.as_rule();
    let text = pair.as_str();
    match rule {
        Rule::pat => Ok(Expression::Alternative(
            pair.into_inner()
                .map(expression)
                .collect::<Result<_, _>>()?,
        )),
        Rule::pat_branch => Ok(Expression::Sequence(
            pair.into_inner()
                .map(expression)
                .collect::<Result<_, _>>()?,
        )),
        Rule::pat_piece => expression(
            pair.into_inner()
                .next()
                .ok_or(PatternError::InvalidPattern)?,
        ),
        Rule::pat_anchor => Ok(if text == "^" {
            Expression::Start
        } else {
            Expression::End
        }),
        Rule::pat_group_quant
        | Rule::pat_class_quant
        | Rule::pat_escape_quant
        | Rule::pat_dot_quant
        | Rule::pat_lit_quant => {
            let mut children = pair.into_inner();
            let base = match rule {
                Rule::pat_dot_quant => Expression::Atom(Atom::Any),
                Rule::pat_group_quant => {
                    expression(children.next().ok_or(PatternError::InvalidPattern)?)?
                }
                Rule::pat_class_quant => {
                    character_class(children.next().ok_or(PatternError::InvalidPattern)?)?
                }
                Rule::pat_escape_quant => {
                    let esc = children
                        .next()
                        .ok_or(PatternError::InvalidPattern)?
                        .as_str()
                        .chars()
                        .next()
                        .ok_or(PatternError::InvalidPattern)?;
                    Expression::Atom(match esc {
                        'd' => Atom::Digit(false),
                        'D' => Atom::Digit(true),
                        'w' => Atom::Word(false),
                        'W' => Atom::Word(true),
                        's' => Atom::Space(false),
                        'S' => Atom::Space(true),
                        _ => Atom::Literal(esc),
                    })
                }
                _ => Expression::Atom(Atom::Literal(
                    children
                        .next()
                        .ok_or(PatternError::InvalidPattern)?
                        .as_str()
                        .chars()
                        .next()
                        .ok_or(PatternError::InvalidPattern)?,
                )),
            };
            if let Some(quant) = children.next() {
                Ok(Expression::Repeat {
                    inner: Box::new(base),
                    minimum: usize::from(quant.as_str() == "+"),
                    maximum: (quant.as_str() == "?").then_some(1),
                })
            } else {
                Ok(base)
            }
        }
        _ => Err(PatternError::InvalidPattern),
    }
}
fn character_class(pair: Pair<'_, Rule>) -> Result<Expression, PatternError> {
    let mut negated = false;
    let mut ranges = Vec::new();
    for item in pair.into_inner() {
        if item.as_rule() == Rule::pat_class_neg {
            negated = true;
            continue;
        }
        let mut chars = item.into_inner().map(|p| {
            p.as_str()
                .chars()
                .next()
                .ok_or(PatternError::InvalidPattern)
        });
        let first = chars.next().ok_or(PatternError::InvalidPattern)??;
        let last = chars.next().transpose()?.unwrap_or(first);
        ranges.push((first, last));
    }
    Ok(Expression::Atom(Atom::Class { negated, ranges }))
}

#[cfg(test)]
mod tests {
    #[test]
    fn class_backslash_does_not_hide_grouping_depth() {
        let pattern = format!(r"[\]{}a{}", "(".repeat(65), ")".repeat(65));
        assert!(matches!(
            super::parse(&pattern, 64),
            Err(super::PatternError::PatternDepthLimit)
        ));
    }
}
