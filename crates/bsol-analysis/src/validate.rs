//! Validate a parsed document against a schema profile.

mod block;
mod coercion;
mod constraints;
mod document;
mod field;
mod formatting;
mod model;

pub use document::{validate, validate_with};
pub use model::{ValidatedBlock, ValidatedDocument};

#[cfg(test)]
mod tests;
