//! BSOL syntax — parser and generic block AST.

mod ast;
mod build;
mod error;
mod parser;
mod write;
pub use write::{write_bsol_document, write_bsol_value};

pub use ast::{
    BsolAssignment, BsolAttribute, BsolAttributeArg, BsolBlock, BsolBracketList, BsolDocument,
    BsolInlineMap, BsolItem, BsolListItem, BsolMapEntry, BsolNumber, BsolQuotedString, BsolRef,
    BsolSpan, BsolValue,
};
pub use build::parse_bsol_document;
pub use error::BsolError;
pub use parser::{BsolParser, Rule};

/// Alias for [`parse_bsol_document`].
pub use build::parse_bsol_document as parse_document;
