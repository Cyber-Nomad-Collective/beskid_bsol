//! Pest-generated parser for BSOL (`bsol.pest`).

use pest_derive::Parser;

/// Entry type for [`pest::Parser`] over BSOL source ([`Rule::document`]).
#[derive(Parser)]
#[grammar = "bsol.pest"]
pub struct BsolParser;
