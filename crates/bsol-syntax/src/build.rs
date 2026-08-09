//! Build a [`BsolDocument`] from pest pairs and top-level block scanning.

mod block_parser;
mod model;
mod pest_ast_builder;
mod scanner_primitives;
mod spans_errors;
#[cfg(test)]
mod tests;

pub use block_parser::parse_bsol_document;
