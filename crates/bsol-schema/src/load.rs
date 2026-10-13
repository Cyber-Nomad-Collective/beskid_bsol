//! Bootstrap loader: parse declarative schema profile documents into [`SchemaProfile`].

mod defaults;
mod document;
mod embedded_profiles;
mod fields;
mod imports_extends;
mod migrations;
mod rules;
mod value_decode;
mod variants;

pub use document::{
    load_profile_from_document, load_profile_from_path, load_profile_from_source,
    parse_profile_document,
};
pub use embedded_profiles::{embedded_profile_source, load_profile};
pub use imports_extends::parse_import_schema;
pub(crate) use value_decode::parse_value_type_text;

#[cfg(test)]
mod tests;
