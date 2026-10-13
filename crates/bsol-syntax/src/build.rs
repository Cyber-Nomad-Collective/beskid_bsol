//! Build a [`BsolDocument`] from pest pairs and top-level block scanning.

mod block_parser;
mod limit_accounting;
mod model;
mod parse_source;
mod pest_ast_builder;
mod scanner_primitives;
mod spans_errors;
#[cfg(test)]
mod tests;

pub use block_parser::{parse_bsol_document, parse_bsol_document_with_limits};
