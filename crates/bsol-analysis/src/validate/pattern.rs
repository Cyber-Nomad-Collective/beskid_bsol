//! Bounded Thompson matching: sequence-aware quantifiers, no recursion over subjects.
use super::PatternLimits;
use bsol_schema::pattern::{Atom, Expression};

#[derive(Clone)]
enum State {
    Accept,
    Atom(Atom, usize),
    Start(usize),
    End(usize),
    Split(usize, usize),
}
pub(super) struct Budget {
    pub limits: PatternLimits,
    remaining: usize,
}
impl Budget {
    pub fn new(limits: PatternLimits) -> Self {
        Self {
            remaining: limits.maxWork,
            limits,
        }
    }
    fn charge(&mut self) -> Result<(), &'static str> {
        if self.remaining == 0 {
            return Err("PatternWorkLimit");
        }
        self.remaining -= 1;
        Ok(())
    }
}
fn add(states: &mut Vec<State>, state: State, budget: &mut Budget) -> Result<usize, &'static str> {
    budget.charge()?;
    let index = states.len();
    states.push(state);
    Ok(index)
}
fn compile(
    expression: &Expression,
    next: usize,
    states: &mut Vec<State>,
    budget: &mut Budget,
) -> Result<usize, &'static str> {
    match expression {
        Expression::Atom(atom) => add(states, State::Atom(atom.clone(), next), budget),
        Expression::Start => add(states, State::Start(next), budget),
        Expression::End => add(states, State::End(next), budget),
        Expression::Sequence(items) => {
            let mut head = next;
            for item in items.iter().rev() {
                head = compile(item, head, states, budget)?;
            }
            Ok(head)
        }
        Expression::Alternative(items) => {
            let mut heads = Vec::new();
            for item in items {
                heads.push(compile(item, next, states, budget)?);
            }
            let mut head = *heads.last().ok_or("InvalidPattern")?;
            for other in heads[..heads.len() - 1].iter().rev() {
                head = add(states, State::Split(*other, head), budget)?;
            }
            Ok(head)
        }
        Expression::Repeat {
            inner,
            minimum,
            maximum,
        } => {
            if *maximum == Some(1) {
                let head = compile(inner, next, states, budget)?;
                return add(states, State::Split(head, next), budget);
            }
            let loop_index = add(states, State::Accept, budget)?;
            let head = compile(inner, loop_index, states, budget)?;
            states[loop_index] = State::Split(head, next);
            Ok(if *minimum == 0 { loop_index } else { head })
        }
    }
}
fn matches_atom(atom: &Atom, ch: char, budget: &mut Budget) -> Result<bool, &'static str> {
    Ok(match atom {
        Atom::Literal(literal) => ch == *literal,
        Atom::Any => true,
        Atom::Digit(negated) => ch.is_ascii_digit() != *negated,
        Atom::Word(negated) => (ch.is_ascii_alphanumeric() || ch == '_') != *negated,
        Atom::Space(negated) => (ch == ' ') != *negated,
        Atom::Class { negated, ranges } => {
            let mut accepted = false;
            for (first, last) in ranges {
                budget.charge()?;
                if *first <= ch && ch <= *last {
                    accepted = true;
                    break;
                }
            }
            accepted != *negated
        }
    })
}
pub(super) fn matches(
    pattern: &str,
    text: &str,
    budget: &mut Budget,
) -> Result<bool, &'static str> {
    if text.len() > budget.limits.maxSubjectBytes {
        return Err("PatternSubjectLimit");
    }
    // Parsing has bounded nesting; debit owned pattern input before grammar work.
    if pattern.len() > budget.remaining {
        return Err("PatternWorkLimit");
    }
    budget.remaining -= pattern.len();
    let expression = bsol_schema::pattern::parse(pattern, budget.limits.maxDepth).map_err(
        |error| match error {
            bsol_schema::pattern::PatternError::InvalidPattern => "InvalidPattern",
            bsol_schema::pattern::PatternError::PatternDepthLimit => "PatternDepthLimit",
        },
    )?;
    let mut states = Vec::new();
    add(&mut states, State::Accept, budget)?;
    let head = compile(&expression, 0, &mut states, budget)?;
    let mut active = vec![head];
    let mut seen = vec![None; states.len()];
    let mut position = 0usize;
    let mut characters = text.chars();
    loop {
        let mut stack = std::mem::take(&mut active);
        let mut atoms = Vec::new();
        while let Some(index) = stack.pop() {
            budget.charge()?;
            if seen[index] == Some(position) {
                continue;
            }
            seen[index] = Some(position);
            match &states[index] {
                State::Accept => {
                    if position > 0 {
                        return Ok(true);
                    }
                }
                State::Atom(atom, next) => atoms.push((atom, *next)),
                State::Start(next) => {
                    if position == 0 {
                        stack.push(*next);
                    }
                }
                State::End(next) => {
                    if position == text.len() {
                        stack.push(*next);
                    }
                }
                State::Split(first, second) => {
                    budget.charge()?;
                    stack.push(*second);
                    stack.push(*first);
                }
            }
        }
        let Some(ch) = characters.next() else {
            return Ok(false);
        };
        for (atom, next) in atoms {
            budget.charge()?;
            if matches_atom(atom, ch, budget)? {
                active.push(next);
            }
        }
        if active.is_empty() {
            return Ok(false);
        }
        position = position
            .checked_add(ch.len_utf8())
            .ok_or("InvalidPatternLimits")?;
    }
}
