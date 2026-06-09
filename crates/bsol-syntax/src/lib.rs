//! BSOL syntax — parser and generic block AST.

mod ast;
mod build;
mod error;
mod parser;

pub use ast::{
    BsolAssignment, BsolBlock, BsolBracketList, BsolDocument, BsolItem, BsolListItem,
    BsolQuotedString, BsolSpan, BsolValue,
};
pub use build::parse_bsol_document;
pub use error::BsolError;
pub use parser::{BsolParser, Rule};

/// Alias for [`parse_bsol_document`].
pub use build::parse_bsol_document as parse_document;
