//! Bootstrap loader: parse declarative schema profile documents into [`SchemaProfile`].

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
pub use embedded_profiles::load_profile;
pub(crate) use value_decode::parse_value_type_text;

#[cfg(test)]
mod tests;
