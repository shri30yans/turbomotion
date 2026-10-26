//! The schema's own declaration, as data: every struct and enum the parser deserialises, with each field's serde key,
//! Rust type and doc comment. The table is written by `build.rs` from the source of this folder, so what
//! `comp catalogue layer` prints is what the parser accepts.

/// One key of a struct, as serde reads it.
#[derive(Debug)]
pub struct SchemaField {
    pub key: &'static str,
    /// The type as written: `Option<P>`, `[f64; 2]`.
    pub ty: &'static str,
    pub doc: &'static str,
    /// No default and not an `Option`: leaving it out is an error.
    pub required: bool,
}

#[derive(Debug)]
pub struct SchemaStruct {
    pub name: &'static str,
    pub doc: &'static str,
    pub fields: &'static [SchemaField],
}

#[derive(Debug)]
pub struct SchemaVariant {
    pub name: &'static str,
    pub doc: &'static str,
}

#[derive(Debug)]
pub struct SchemaEnum {
    pub name: &'static str,
    pub doc: &'static str,
    pub variants: &'static [SchemaVariant],
}

include!(concat!(env!("OUT_DIR"), "/schema_docs.rs"));

pub fn struct_named(name: &str) -> Option<&'static SchemaStruct> {
    SCHEMA_STRUCTS.iter().find(|s| s.name == name)
}

pub fn enum_named(name: &str) -> Option<&'static SchemaEnum> {
    SCHEMA_ENUMS.iter().find(|e| e.name == name)
}
