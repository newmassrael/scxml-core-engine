// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! An event schema that grows: which revision a document is, which of its
//! fields a later revision added, and what a delivery that lacks one takes.
//!
//! A generated machine reads an event's data through its schema, and refuses a
//! delivery that lacks a field. That is right for the fields every sender
//! writes, and wrong for a field added after some sender was built: a host
//! that keeps the events it sent as a durable log and replays them after an
//! upgrade, or two devices on different versions, would see every older
//! delivery refused the day a field is added. So a field added after the first
//! revision says which revision added it (`sce:since`) and what a delivery
//! that lacks it takes (`sce:default`), and the lift reads that default in its
//! place. A field of the first revision declares neither and stays required.
//!
//! # One spelling of a default
//!
//! A default is turned into the JSON a delivery would carry for the field
//! ([`wire_of`]) once, here, and every engine reads it with the reader it
//! reads a delivered value with: the generated lifts take the schema's
//! defaults as one JSON object ([`defaults_wire`]) and read a field the
//! delivery lacks out of it, and the Interpreter's `field` is handed the one
//! value. A default therefore means the same thing everywhere, and a reader
//! cannot drift from it, because there is no second reader.
//!
//! # What is checked where
//!
//! The parser reads `sce:revision`, `sce:since` and `sce:default` and refuses
//! every rule a document can break alone: a revision that is not a positive
//! integer, a `since` outside `2..=revision`, a default on a field of the first
//! revision, a `since` with no default, and a default that is no value of its
//! field ([`wire_of`]). Whether an enum default names a variant needs the enum
//! document, so [`check`] asks it where the schema's imports are read.

use std::collections::BTreeMap;
use std::path::Path;

use crate::forge::error::{ForgeError, Located, ValidationError};
use crate::forge::model::{
    EnumModel, EnumVariant, EventSchemaModel, ForgeDocument, ForgeField, NumericLiteral,
    ParsedForge, SceType,
};

/// Why a default is no value of its field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unfit {
    /// A rule no list states — a range, a length — phrased to stand as a
    /// diagnostic's `expected`.
    Rule(String),
    /// The closed set the value must be one of.
    OneOf(Vec<String>),
}

/// `text` as the JSON value a delivery carries for a field of type `ty`
/// bounded by `max_size`, or why it is no value of that field.
///
/// The spelling is the one each engine's lift reads: a whole number in
/// decimal, a 64-bit one as the text of its digits, a real with a point or an
/// exponent, a truth value, and a text, a byte string or an enum's declared
/// name quoted. An enum's name is quoted here and held to the enum's variants
/// by [`check`], which has the enum document.
pub fn wire_of(ty: &SceType, max_size: Option<u32>, text: &str) -> Result<String, Unfit> {
    match ty {
        SceType::Bool => match text {
            "true" | "false" => Ok(text.to_string()),
            _ => Err(Unfit::OneOf(vec!["true".into(), "false".into()])),
        },
        SceType::String => {
            if let Some(bound) = max_size {
                if text.len() > bound as usize {
                    return Err(Unfit::Rule(format!(
                        "a text of at most {bound} UTF-8 bytes, the sce:max-size the field \
                         declares"
                    )));
                }
            }
            Ok(json_text(text))
        }
        SceType::Bytes => {
            // The one byte-string literal every engine spells alike: printable
            // ASCII with no escape in it.
            let printable = text
                .bytes()
                .all(|b| (0x20..0x7f).contains(&b) && b != b'\\');
            if !printable {
                return Err(Unfit::Rule(
                    "printable ASCII with no backslash, the one byte-string literal every \
                     engine spells alike"
                        .into(),
                ));
            }
            let bound = crate::forge::limits::resolve_bytes_max(max_size);
            if text.len() > bound as usize {
                return Err(Unfit::Rule(format!(
                    "at most {bound} bytes, the bound the field holds a byte string within"
                )));
            }
            Ok(json_text(text))
        }
        SceType::Enum(_) => Ok(json_text(text)),
        numeric => match numeric.numeric_literal(text) {
            // A 64-bit integer crosses as the text of its digits, which every
            // engine reads exactly.
            Ok(NumericLiteral::Int(n)) if matches!(numeric, SceType::Uint64 | SceType::Int64) => {
                Ok(format!("\"{n}\""))
            }
            // A number past the largest single does not fit a `float32` field
            // where a delivery is read, so it does not fit as a default either.
            Ok(NumericLiteral::Float(v))
                if matches!(numeric, SceType::Float32) && v.abs() > f64::from(f32::MAX) =>
            {
                Err(Unfit::Rule(
                    "a finite decimal number within float32's range".into(),
                ))
            }
            Ok(literal) => Ok(literal.to_source()),
            Err(rule) => Err(Unfit::Rule(rule)),
        },
    }
}

/// The JSON a delivery that lacks `field` reads in its place: its default as
/// [`wire_of`] spells it. `None` for a field that declares no default, or one
/// whose default is no value of it (which the parser refuses).
pub fn default_wire(field: &ForgeField) -> Option<String> {
    let text = field.default.as_deref()?;
    wire_of(&field.sce_type, field.max_size, text).ok()
}

/// The defaults of every field of `schema` a later revision added, as one
/// JSON object keyed by field id in the schema's order — what a generated
/// lift reads a field the delivery lacks out of. `None` when no field of the
/// schema was added after the first revision, which leaves the lift as it is.
pub fn defaults_wire(schema: &EventSchemaModel) -> Option<String> {
    let members: Vec<String> = schema
        .fields
        .iter()
        .filter(|f| f.since.is_some())
        .filter_map(|f| Some(format!("{}:{}", json_text(&f.id), default_wire(f)?)))
        .collect();
    if members.is_empty() {
        None
    } else {
        Some(format!("{{{}}}", members.join(",")))
    }
}

/// Refuse an enum default no variant of the field's enum declares.
///
/// `variants_of` answers the variants of the enum a field names by its alias,
/// or `None` when the alias names no readable enum — which the import pass
/// reports in its own words, so it is skipped here.
pub fn check_schema(
    schema: &EventSchemaModel,
    variants_of: &dyn Fn(&str) -> Option<Vec<String>>,
    document: &str,
) -> Result<(), Located<ForgeError>> {
    for field in &schema.fields {
        let (SceType::Enum(reference), Some(value)) = (&field.sce_type, field.default.as_deref())
        else {
            continue;
        };
        let Some(variants) = variants_of(&reference.alias) else {
            continue;
        };
        if variants.iter().any(|v| v == value) {
            continue;
        }
        let at = field
            .default_spelling
            .as_ref()
            .map(|spelling| (spelling.row(), spelling.col()));
        return Err(Located::new(
            ValidationError::InvalidAttribute {
                element: format!("EventSchema field '{}'", field.id),
                attr: "sce:default".into(),
                value: value.to_string(),
                allowed: variants,
            }
            .into(),
            document,
            at.map(|(row, _)| row),
            at.map(|(_, col)| col),
        ));
    }
    Ok(())
}

/// [`check_schema`] for a parsed document, with each enum read from the import
/// that names it beside the document in `base_dir`. A document of any other
/// kind has no default to check.
pub fn check(
    parsed: &ParsedForge,
    base_dir: &Path,
    document: &str,
) -> Result<(), Located<ForgeError>> {
    let ForgeDocument::EventSchema(schema) = &parsed.document else {
        return Ok(());
    };
    check_schema(
        schema,
        &|alias| crate::forge::import_source::enum_variants(parsed, base_dir, alias),
        document,
    )
}

/// One of the two schemas or enums [`divergence`] and [`enum_divergence`] are
/// handed, by its place in the call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Party {
    /// The first argument.
    First,
    /// The second argument.
    Second,
}

/// Where two schemas of one event part, so that they are not revisions of one
/// schema ([`divergence`], [`divergence_with_enums`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Divergence {
    /// Both declare `field`, and declare it differently: another type, bound,
    /// quantity, `sce:since` or `sce:default`.
    Redeclared {
        /// The field's id.
        field: String,
    },
    /// One schema declares `field` as of revision `since` (1 for a field of the
    /// first revision), and `from`, whose revision is `since` or later, does not
    /// declare it: the field was removed or renamed, or the two disagree about
    /// which revision added it.
    Missing {
        /// The field's id.
        field: String,
        /// The revision the schema that declares it says added it.
        since: u32,
        /// The schema that does not declare it.
        from: Party,
    },
    /// Both declare `field` alike, of an enum the two hold under one alias, and
    /// the two enums are not revisions of one enum ([`enum_divergence`]).
    Enum {
        /// The field's id.
        field: String,
        /// The first schema's enum's `sce:revision` (1 when it declares none).
        first_revision: u32,
        /// The second schema's enum's `sce:revision` (1 when it declares none).
        second_revision: u32,
        /// Where the two enums part.
        divergence: EnumDivergence,
    },
}

/// Where two enums part, so that they are not revisions of one enum
/// ([`enum_divergence`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnumDivergence {
    /// The two declare another carrier (`sce:underlying-type`) or another
    /// openness (`sce:strict-variants`): a re-typed enum, not a revision.
    Carrier,
    /// Both declare `variant`, and declare it differently: another value or
    /// another `sce:since`.
    Redeclared {
        /// The variant's name.
        variant: String,
    },
    /// One enum declares `variant` as of revision `since` (1 for a variant of
    /// the first revision), and `from`, whose revision is `since` or later, does
    /// not declare it: the variant was removed or renamed, or the two disagree
    /// about which revision added it.
    Missing {
        /// The variant's name.
        variant: String,
        /// The revision the enum that declares it says added it.
        since: u32,
        /// The enum that does not declare it.
        from: Party,
    },
}

/// The revision a field belongs to: the one its `sce:since` names, and the first
/// for a field that declares none.
pub fn added_at(field: &ForgeField) -> u32 {
    field.since.unwrap_or(1)
}

/// Where two members of one kind that grows by revisions part — the fields
/// of two event schemas, the variants of two enums (docs/adr/0044).
#[derive(Debug, Clone, PartialEq, Eq)]
enum Parting {
    /// Both declare the member, and declare it differently.
    Redeclared(String),
    /// One declares it as of revision `since`, and `from`, whose revision has
    /// it, does not.
    Missing {
        name: String,
        since: u32,
        from: Party,
    },
}

/// The one rule every kind that grows by revisions is held to (docs/adr/0044):
/// two documents are revisions of one when every member both declare is
/// declared alike, and every member only one declares was added after the
/// other's revision. `members` are each document's members with its revision,
/// first then second; `name` keys a member, `added_at` is the revision that
/// added it and `alike` whether two of one name are declared alike.
///
/// `None` when they are revisions of one; otherwise the first parting, the
/// first document's members judged before the second's, each in declaration
/// order, so the same pair always answers the same.
fn parting<T>(
    first: (&[T], u32),
    second: (&[T], u32),
    name: impl Fn(&T) -> &str,
    added_at: impl Fn(&T) -> u32,
    alike: impl Fn(&T, &T) -> bool,
) -> Option<Parting> {
    for ((one, _), (other, other_revision), from) in [
        (first, second, Party::Second),
        (second, first, Party::First),
    ] {
        for member in one {
            match other.iter().find(|theirs| name(theirs) == name(member)) {
                Some(theirs) if !alike(member, theirs) => {
                    return Some(Parting::Redeclared(name(member).to_string()));
                }
                Some(_) => {}
                None if added_at(member) <= other_revision => {
                    return Some(Parting::Missing {
                        name: name(member).to_string(),
                        since: added_at(member),
                        from,
                    });
                }
                // Added after the other's revision: the other is an earlier
                // revision, which a reader of the later one reads through what
                // the later one says of the member.
                None => {}
            }
        }
    }
    None
}

/// Whether `first` and `second`, two schemas of one event, are revisions of one
/// schema, and where they part when they are not (docs/adr/0044).
///
/// They are when every field both declare is declared alike (the field as the
/// forge model serialises it: type, bound, quantity, `since`, default), and every
/// field only one declares was added after the other's revision. So a later
/// revision keeps every field of an earlier one, alike, and adds fields only past
/// the earlier revision; a field removed, renamed or re-typed, two schemas of one
/// revision that declare different fields, and two that disagree about which
/// revision added a field all part. The rule is symmetric: which of the two is the
/// older does not matter, and neither does the order the fields are declared in.
///
/// An enum field is compared by the alias it names; whether the two enums under
/// that alias are revisions of one enum needs the enum documents
/// ([`divergence_with_enums`]).
///
/// `None` when they are revisions of one schema; otherwise the first divergence,
/// the first schema's fields judged before the second's, each in declaration order,
/// so the same pair always answers the same.
pub fn divergence(first: &EventSchemaModel, second: &EventSchemaModel) -> Option<Divergence> {
    parting(
        (first.fields.as_slice(), first.revision()),
        (second.fields.as_slice(), second.revision()),
        |f: &ForgeField| f.id.as_str(),
        added_at,
        declared_alike,
    )
    .map(|parted| match parted {
        Parting::Redeclared(field) => Divergence::Redeclared { field },
        Parting::Missing { name, since, from } => Divergence::Missing {
            field: name,
            since,
            from,
        },
    })
}

/// [`divergence`], and then, for each enum field both schemas declare, whether
/// the enum document each side's schema names for it (`first_enums`,
/// `second_enums`, by field id — [`EnumDocuments::by_field`]) is a revision of
/// the other's ([`enum_divergence`]). An enum is the document its schema
/// imports, not the alias a machine reads it by. A field one side holds no
/// document for is not judged here: the import pass reports it.
pub fn divergence_with_enums(
    first: &EventSchemaModel,
    first_enums: &BTreeMap<String, EnumDocument>,
    second: &EventSchemaModel,
    second_enums: &BTreeMap<String, EnumDocument>,
) -> Option<Divergence> {
    divergence(first, second).or_else(|| {
        first.fields.iter().find_map(|field| {
            let SceType::Enum(_) = &field.sce_type else {
                return None;
            };
            second.fields.iter().find(|theirs| theirs.id == field.id)?;
            let one = &first_enums.get(&field.id)?.model;
            let other = &second_enums.get(&field.id)?.model;
            enum_divergence(one, other).map(|divergence| Divergence::Enum {
                field: field.id.clone(),
                first_revision: one.revision(),
                second_revision: other.revision(),
                divergence,
            })
        })
    })
}

/// An enum document as a saved state and the Mesh check name it
/// (docs/adr/0044): by the document, as every reader of an enum identifies it
/// ([`crate::forge::stdlib::identity`]), not by the alias one document imports
/// it under.
#[derive(Debug, Clone)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct EnumDocument {
    /// The document, spelled so that it is one text on every backend and in
    /// every checkout ([`document_key`]).
    pub key: String,
    /// The enum the document declares.
    pub model: EnumModel,
}

/// The enum documents a statechart holds values of, resolved where it is parsed.
#[derive(Debug, Clone, Default)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct EnumDocuments {
    /// The statechart's own enum imports, by the alias it imports each under —
    /// what an `enum:<alias>` variable holds.
    pub by_alias: BTreeMap<String, EnumDocument>,
    /// For each event schema the statechart imports, by the event it declares,
    /// the document each enum field of it is of, by field id — read from the
    /// schema's own imports, since that is the enum the field's value is.
    pub by_field: BTreeMap<String, BTreeMap<String, EnumDocument>>,
}

/// The document `path` (an import resolved by [`crate::forge::stdlib::resolve`])
/// as a saved state spells it, with `base_dir` the directory of the statechart
/// whose state it is: a standard document by its folded `sce:std/...` name, any
/// other file by its path relative to `base_dir`, `.` and `..` folded and each
/// component separated by `/`. The same text on every backend, since the
/// generator writes it; the same in every checkout, since it holds no absolute
/// path; and one text for one document however it is imported, since both
/// sides are the canonical paths [`crate::forge::stdlib::identity`] compares.
pub fn document_key(base_dir: &Path, path: &Path) -> String {
    let identity = crate::forge::stdlib::identity(path);
    if identity.starts_with(crate::forge::stdlib::SCHEME) {
        return identity;
    }
    let base = std::fs::canonicalize(base_dir).unwrap_or_else(|_| base_dir.to_path_buf());
    relative_key(&base, Path::new(&identity))
}

/// `target` relative to `base`, component by component, `/`-separated.
fn relative_key(base: &Path, target: &Path) -> String {
    use std::path::Component;
    let parts = |path: &Path| -> Vec<String> {
        let mut parts: Vec<String> = Vec::new();
        for component in path.components() {
            match component {
                Component::CurDir => {}
                Component::ParentDir => {
                    parts.pop();
                }
                other => parts.push(other.as_os_str().to_string_lossy().into_owned()),
            }
        }
        parts
    };
    let base = parts(base);
    let target = parts(target);
    let common = base
        .iter()
        .zip(&target)
        .take_while(|(one, other)| one == other)
        .count();
    let mut key: Vec<String> = vec!["..".to_string(); base.len() - common];
    key.extend(target[common..].iter().cloned());
    key.join("/")
}

/// The enum documents `model`, a statechart parsed from `base_dir`, holds values
/// of ([`EnumDocuments`]). An import that cannot be read is left out: the import
/// pass reports it.
pub fn enum_documents(model: &crate::model::SCXMLModel, base_dir: &Path) -> EnumDocuments {
    use crate::forge::import_source::ImportSource;
    use crate::forge::model::ForgeKind;

    let read_enum = |import: &crate::forge::model::ForgeImport, dir: &Path| {
        let source = ImportSource::read(dir, import).ok()?;
        let parsed = source.parse().ok()??;
        match parsed.document {
            ForgeDocument::Enum(enum_model) => Some(EnumDocument {
                key: document_key(base_dir, &source.path),
                model: enum_model,
            }),
            _ => None,
        }
    };
    let mut documents = EnumDocuments::default();
    for import in &model.forge_imports {
        match import.kind {
            ForgeKind::Enum => {
                if let Some(document) = read_enum(import, base_dir) {
                    documents.by_alias.insert(import.alias.clone(), document);
                }
            }
            ForgeKind::EventSchema => {
                let Ok(source) = ImportSource::read(base_dir, import) else {
                    continue;
                };
                let Ok(Some(parsed)) = source.parse() else {
                    continue;
                };
                let ForgeDocument::EventSchema(schema) = &parsed.document else {
                    continue;
                };
                // The schema's own imports are written relative to the schema.
                let schema_dir = source.path.parent().unwrap_or(base_dir);
                let mut fields = BTreeMap::new();
                for field in &schema.fields {
                    let SceType::Enum(reference) = &field.sce_type else {
                        continue;
                    };
                    let Some(enum_import) = parsed
                        .imports
                        .iter()
                        .find(|i| i.alias == reference.alias && i.kind == ForgeKind::Enum)
                    else {
                        continue;
                    };
                    if let Some(document) = read_enum(enum_import, schema_dir) {
                        fields.insert(field.id.clone(), document);
                    }
                }
                documents
                    .by_field
                    .entry(schema.event_name.clone())
                    .or_insert(fields);
            }
            _ => {}
        }
    }
    documents
}

/// Whether `first` and `second` are revisions of one enum, and where they part
/// when they are not (docs/adr/0044) — the rule an event schema's fields are
/// held to ([`divergence`]), over variants.
///
/// They are when they declare one carrier and one openness, every variant both
/// declare has one value and one `sce:since`, and every variant only one
/// declares was added after the other's revision. So a later revision keeps
/// every variant of an earlier one, with its value, and adds variants only past
/// the earlier revision; a variant removed, renamed or re-valued, two enums of
/// one revision that declare different variants, and two that disagree about
/// which revision added a variant all part. Declaration order is not part of an
/// enum: a variant is told by its name and carries its value.
pub fn enum_divergence(first: &EnumModel, second: &EnumModel) -> Option<EnumDivergence> {
    if first.underlying_type != second.underlying_type
        || first.strict_variants != second.strict_variants
    {
        return Some(EnumDivergence::Carrier);
    }
    parting(
        (first.variants.as_slice(), first.revision()),
        (second.variants.as_slice(), second.revision()),
        |v: &EnumVariant| v.name.as_str(),
        EnumVariant::added_at,
        |one: &EnumVariant, other: &EnumVariant| {
            one.value == other.value && one.since == other.since
        },
    )
    .map(|parted| match parted {
        Parting::Redeclared(variant) => EnumDivergence::Redeclared { variant },
        Parting::Missing { name, since, from } => EnumDivergence::Missing {
            variant: name,
            since,
            from,
        },
    })
}

/// Whether two fields of one id are declared alike: the whole field, as the forge
/// model serialises it (positions are not serialised, so where it is written does
/// not count).
fn declared_alike(one: &ForgeField, other: &ForgeField) -> bool {
    match (serde_json::to_value(one), serde_json::to_value(other)) {
        (Ok(one), Ok(other)) => one == other,
        _ => false,
    }
}

/// A text as a JSON string.
fn json_text(text: &str) -> String {
    serde_json::to_string(text).expect("a string is JSON")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::model::{Direction, EnumRef};

    fn field(id: &str, ty: SceType, since: Option<u32>, default: Option<&str>) -> ForgeField {
        ForgeField {
            id: id.to_string(),
            sce_type: ty,
            direction: Direction::In,
            expr: None,
            expr_spelling: None,
            expr_splices: None,
            quantity: None,
            max_size: None,
            default_covers: Vec::new(),
            retain: None,
            initial: None,
            initial_spelling: None,
            since,
            default: default.map(str::to_string),
            default_spelling: None,
        }
    }

    fn schema(fields: Vec<ForgeField>) -> EventSchemaModel {
        EventSchemaModel {
            name: "schema_revised".to_string(),
            event_name: "reading.revised".to_string(),
            fields,
            payload_none: false,
            datamodel_markers: Vec::new(),
            revision: Some(3),
            source_location: None,
        }
    }

    fn view_mode() -> SceType {
        SceType::Enum(EnumRef {
            alias: "ViewMode".to_string(),
        })
    }

    #[test]
    fn each_type_spells_its_default_as_the_json_a_delivery_carries() {
        assert_eq!(wire_of(&SceType::Uint8, None, "7"), Ok("7".into()));
        assert_eq!(wire_of(&SceType::Uint8, None, "0x10"), Ok("16".into()));
        assert_eq!(wire_of(&SceType::Int16, None, "-1"), Ok("-1".into()));
        assert_eq!(
            wire_of(&SceType::Uint64, None, "18446744073709551615"),
            Ok("\"18446744073709551615\"".into())
        );
        assert_eq!(wire_of(&SceType::Int64, None, "-5"), Ok("\"-5\"".into()));
        assert_eq!(wire_of(&SceType::Float64, None, "10"), Ok("10.0".into()));
        assert_eq!(wire_of(&SceType::Float32, None, "0.5"), Ok("0.5".into()));
        assert_eq!(wire_of(&SceType::Bool, None, "true"), Ok("true".into()));
        assert_eq!(
            wire_of(&SceType::String, Some(16), "say \"hi\""),
            Ok("\"say \\\"hi\\\"\"".into())
        );
        assert_eq!(
            wire_of(&SceType::Bytes, Some(4), "ack"),
            Ok("\"ack\"".into())
        );
        assert_eq!(wire_of(&view_mode(), None, "week"), Ok("\"week\"".into()));
    }

    #[test]
    fn a_default_that_is_no_value_of_its_field_is_unfit() {
        assert!(matches!(
            wire_of(&SceType::Uint8, None, "256"),
            Err(Unfit::Rule(_))
        ));
        assert!(matches!(
            wire_of(&SceType::Int8, None, "many"),
            Err(Unfit::Rule(_))
        ));
        assert!(matches!(
            wire_of(&SceType::Float32, None, "1e39"),
            Err(Unfit::Rule(_))
        ));
        assert!(matches!(
            wire_of(&SceType::Float64, None, "NaN"),
            Err(Unfit::Rule(_))
        ));
        assert_eq!(
            wire_of(&SceType::Bool, None, "yes"),
            Err(Unfit::OneOf(vec!["true".into(), "false".into()]))
        );
        assert!(matches!(
            wire_of(&SceType::String, Some(3), "four"),
            Err(Unfit::Rule(_))
        ));
        // A string with no bound takes a text of any length.
        assert!(wire_of(&SceType::String, None, "a longer text").is_ok());
        assert!(matches!(
            wire_of(&SceType::Bytes, Some(2), "ack"),
            Err(Unfit::Rule(_))
        ));
        assert!(matches!(
            wire_of(&SceType::Bytes, None, "a\\b"),
            Err(Unfit::Rule(_))
        ));
    }

    #[test]
    fn the_defaults_of_a_schema_are_one_object_of_the_fields_a_later_revision_added() {
        let revised = schema(vec![
            field("level", SceType::Uint8, None, None),
            field("layout", view_mode(), Some(2), Some("week")),
            field("urgent", SceType::Bool, Some(3), Some("true")),
        ]);
        assert_eq!(
            defaults_wire(&revised).as_deref(),
            Some(r#"{"layout":"week","urgent":true}"#)
        );
        let first = schema(vec![field("level", SceType::Uint8, None, None)]);
        assert_eq!(defaults_wire(&first), None);
    }

    /// The model a payload builder is handed: one imported schema for
    /// `reading.revised`, whose channel the event itself opens (as a host
    /// action reading a field of it would).
    fn model_importing(revised: EventSchemaModel) -> crate::model::SCXMLModel {
        let mut model = crate::model::SCXMLModel {
            name: "revised".to_string(),
            ..Default::default()
        };
        model
            .imported_event_schemas
            .insert("reading.revised".to_string(), revised);
        model
    }

    #[test]
    fn every_generated_lift_is_handed_the_defaults_of_a_schema_that_grew() {
        use crate::forge::generator::{
            build_c11_event_payload, build_cpp_event_payload, build_go_event_payload,
            build_kotlin_event_payload, build_python_event_payload, build_rust_event_payload,
        };
        let events: std::collections::BTreeSet<String> =
            std::iter::once("reading.revised".to_string()).collect();
        let model = model_importing(schema(vec![
            field("level", SceType::Uint8, None, None),
            field("urgent", SceType::Bool, Some(3), Some("true")),
        ]));
        let defaults = r#""{\"urgent\":true}""#;

        let rust = build_rust_event_payload(&model, "Revised", &events, "", "", false, &[]).lift;
        assert!(
            rust.contains(&format!(
                "PayloadFields::decode_with_defaults(data, {defaults})?"
            )),
            "{rust}"
        );
        let c11 = build_c11_event_payload(&model, &events, "", &[]).lift_def;
        assert!(
            c11.contains(&format!(
                "sce_payload_decode_with_defaults(_data, {defaults}, &_fields)"
            )),
            "{c11}"
        );
        let go = build_go_event_payload(&model, &events, &[]).lift;
        assert!(
            go.contains(&format!(
                "sce.LiftPayloadWithDefaults(meta.Data, {defaults})"
            )),
            "{go}"
        );
        let cpp = build_cpp_event_payload(&model, &events, &[]).policy_members;
        assert!(
            cpp.contains(&format!(
                "EventPayloadFields::decode(meta.data, {defaults}, fields)"
            )),
            "{cpp}"
        );
        let kotlin = build_kotlin_event_payload(&model, &events, &[]).populate;
        assert!(
            kotlin.contains(&format!("EventPayload.decode(metadata.data, {defaults})")),
            "{kotlin}"
        );
        let python = build_python_event_payload(&model, &events, &[]).populate;
        assert!(python.contains(&format!("), {defaults}))")), "{python}");

        // A schema no later revision added a field to keeps the lift it had.
        let first = model_importing(schema(vec![field("level", SceType::Uint8, None, None)]));
        let rust = build_rust_event_payload(&first, "Revised", &events, "", "", false, &[]).lift;
        assert!(rust.contains("PayloadFields::decode(data)?"), "{rust}");
        assert!(!rust.contains("decode_with_defaults"), "{rust}");
        let python = build_python_event_payload(&first, &events, &[]).populate;
        assert!(python.contains("), )))"), "{python}");
    }

    /// `schema_revised` at `revision`, declaring `fields`.
    fn at_revision(revision: u32, fields: Vec<ForgeField>) -> EventSchemaModel {
        EventSchemaModel {
            revision: Some(revision),
            ..schema(fields)
        }
    }

    fn level() -> ForgeField {
        field("level", SceType::Uint8, None, None)
    }

    fn layout() -> ForgeField {
        field("layout", view_mode(), Some(2), Some("week"))
    }

    fn urgent() -> ForgeField {
        field("urgent", SceType::Bool, Some(3), Some("true"))
    }

    #[test]
    fn every_revision_of_a_schema_is_one_schema_with_every_other() {
        let first = at_revision(1, vec![level()]);
        let second = at_revision(2, vec![level(), layout()]);
        let third = at_revision(3, vec![level(), layout(), urgent()]);
        for (one, other) in [
            (&first, &second),
            (&first, &third),
            (&second, &third),
            (&third, &third),
        ] {
            assert_eq!(divergence(one, other), None, "{one:?} / {other:?}");
            assert_eq!(divergence(other, one), None, "{other:?} / {one:?}");
        }
        // The order fields are declared in is not part of the schema.
        let reordered = at_revision(3, vec![urgent(), level(), layout()]);
        assert_eq!(divergence(&third, &reordered), None);
    }

    #[test]
    fn a_field_declared_otherwise_parts_two_schemas() {
        let third = at_revision(3, vec![level(), layout(), urgent()]);
        // Re-typed, at the first revision.
        let wider = at_revision(
            3,
            vec![
                field("level", SceType::Uint16, None, None),
                layout(),
                urgent(),
            ],
        );
        assert_eq!(
            divergence(&third, &wider),
            Some(Divergence::Redeclared {
                field: "level".into()
            })
        );
        // Another default: a receiver reads it for every older delivery.
        let other_default = at_revision(
            3,
            vec![
                level(),
                layout(),
                field("urgent", SceType::Bool, Some(3), Some("false")),
            ],
        );
        assert_eq!(
            divergence(&third, &other_default),
            Some(Divergence::Redeclared {
                field: "urgent".into()
            })
        );
        // Added by another revision.
        let earlier_since = at_revision(
            3,
            vec![
                level(),
                layout(),
                field("urgent", SceType::Bool, Some(2), Some("true")),
            ],
        );
        assert_eq!(
            divergence(&third, &earlier_since),
            Some(Divergence::Redeclared {
                field: "urgent".into()
            })
        );
    }

    #[test]
    fn a_field_one_schema_lacks_at_a_revision_that_has_it_parts_two_schemas() {
        let first = at_revision(1, vec![level()]);
        let third = at_revision(3, vec![level(), layout(), urgent()]);

        // A field of the first revision removed (or renamed) later.
        let without_level = at_revision(3, vec![layout(), urgent()]);
        assert_eq!(
            divergence(&first, &without_level),
            Some(Divergence::Missing {
                field: "level".into(),
                since: 1,
                from: Party::Second,
            })
        );
        // A field a later revision added, removed again by a later one.
        let second = at_revision(2, vec![level(), layout()]);
        let without_layout = at_revision(3, vec![level(), urgent()]);
        assert_eq!(
            divergence(&second, &without_layout),
            Some(Divergence::Missing {
                field: "layout".into(),
                since: 2,
                from: Party::Second,
            })
        );
        // Two schemas that say they are revision 3 and declare different fields.
        let third_without_layout = at_revision(3, vec![level(), urgent()]);
        assert_eq!(
            divergence(&third_without_layout, &third),
            Some(Divergence::Missing {
                field: "layout".into(),
                since: 2,
                from: Party::First,
            })
        );
        // A revision number moved on without the field the other says it added.
        let bumped = at_revision(3, vec![level(), layout()]);
        assert_eq!(
            divergence(&bumped, &third),
            Some(Divergence::Missing {
                field: "urgent".into(),
                since: 3,
                from: Party::First,
            })
        );
        // A field the first revision has, which an older schema lacks.
        let empty_first = at_revision(1, vec![field("other", SceType::Bool, None, None)]);
        assert_eq!(
            divergence(&first, &empty_first),
            Some(Divergence::Missing {
                field: "level".into(),
                since: 1,
                from: Party::Second,
            })
        );
    }

    // ── An enum that grows (docs/adr/0044) ──

    fn variant(name: &str, value: i128, since: Option<u32>) -> EnumVariant {
        EnumVariant {
            name: name.to_string(),
            value,
            value_text: value.to_string(),
            text: None,
            since,
            source_line: None,
        }
    }

    /// `view_mode` at `revision`, declaring `variants`.
    fn view_mode_at(revision: u32, variants: Vec<EnumVariant>) -> EnumModel {
        EnumModel {
            name: "view_mode".to_string(),
            underlying_type: SceType::Uint8,
            variants,
            strict_variants: true,
            revision: Some(revision),
            source_location: None,
        }
    }

    fn month() -> EnumVariant {
        variant("month", 0, None)
    }

    fn week() -> EnumVariant {
        variant("week", 1, None)
    }

    fn agenda() -> EnumVariant {
        variant("agenda", 2, Some(2))
    }

    #[test]
    fn every_revision_of_an_enum_is_one_enum_with_every_other() {
        let first = view_mode_at(1, vec![month(), week()]);
        let second = view_mode_at(2, vec![month(), week(), agenda()]);
        assert_eq!(enum_divergence(&first, &second), None);
        assert_eq!(enum_divergence(&second, &first), None);
        assert_eq!(enum_divergence(&second, &second), None);
        // The order variants are declared in is not part of an enum.
        let reordered = view_mode_at(2, vec![agenda(), week(), month()]);
        assert_eq!(enum_divergence(&second, &reordered), None);
    }

    #[test]
    fn a_variant_removed_renamed_or_revalued_parts_two_enums() {
        let first = view_mode_at(1, vec![month(), week()]);
        let second = view_mode_at(2, vec![month(), week(), agenda()]);
        // Re-valued.
        let revalued = view_mode_at(2, vec![month(), variant("week", 7, None), agenda()]);
        assert_eq!(
            enum_divergence(&second, &revalued),
            Some(EnumDivergence::Redeclared {
                variant: "week".into()
            })
        );
        // Removed (or renamed: the old name is missing, the new one is too).
        let without_week = view_mode_at(2, vec![month(), agenda()]);
        assert_eq!(
            enum_divergence(&first, &without_week),
            Some(EnumDivergence::Missing {
                variant: "week".into(),
                since: 1,
                from: Party::Second,
            })
        );
        // Two enums that say they are the second revision and disagree on it.
        let second_without_agenda = view_mode_at(2, vec![month(), week()]);
        assert_eq!(
            enum_divergence(&second_without_agenda, &second),
            Some(EnumDivergence::Missing {
                variant: "agenda".into(),
                since: 2,
                from: Party::First,
            })
        );
        // Another carrier, or another openness, is another enum.
        let wider = EnumModel {
            underlying_type: SceType::Uint16,
            ..second.clone()
        };
        assert_eq!(
            enum_divergence(&second, &wider),
            Some(EnumDivergence::Carrier)
        );
        let open = EnumModel {
            strict_variants: false,
            ..second.clone()
        };
        assert_eq!(
            enum_divergence(&second, &open),
            Some(EnumDivergence::Carrier)
        );
    }

    #[test]
    fn an_enum_field_of_two_schemas_is_held_to_the_enum_rule() {
        let layout = field("layout", view_mode(), None, None);
        let sender = at_revision(1, vec![level(), layout.clone()]);
        let receiver = at_revision(1, vec![level(), layout]);
        // The document each side's schema names for the field, by field id: the
        // alias a machine reads it by plays no part.
        let enums_at = |model: EnumModel| -> BTreeMap<String, EnumDocument> {
            std::iter::once((
                "layout".to_string(),
                EnumDocument {
                    key: "enum_view_mode.scxml".to_string(),
                    model,
                },
            ))
            .collect()
        };
        let first = enums_at(view_mode_at(1, vec![month(), week()]));
        let second = enums_at(view_mode_at(2, vec![month(), week(), agenda()]));
        let without_week = enums_at(view_mode_at(2, vec![month(), agenda()]));
        // Two revisions of one enum under the field: one schema.
        assert_eq!(
            divergence_with_enums(&sender, &first, &receiver, &second),
            None
        );
        // Two enums under the field: the field and how they part.
        assert_eq!(
            divergence_with_enums(&sender, &first, &receiver, &without_week),
            Some(Divergence::Enum {
                field: "layout".into(),
                first_revision: 1,
                second_revision: 2,
                divergence: EnumDivergence::Missing {
                    variant: "week".into(),
                    since: 1,
                    from: Party::Second,
                },
            })
        );
        // A field one side holds no document for is the import pass's to report.
        assert_eq!(
            divergence_with_enums(&sender, &first, &receiver, &BTreeMap::new()),
            None
        );
    }

    #[test]
    fn a_document_key_is_relative_to_the_machine_and_names_a_standard_document_by_its_name() {
        // Lexical, as the canonical paths it is handed are.
        assert_eq!(
            relative_key(
                Path::new("/w/app/machines"),
                Path::new("/w/app/machines/enum_a.scxml")
            ),
            "enum_a.scxml"
        );
        assert_eq!(
            relative_key(
                Path::new("/w/app/machines"),
                Path::new("/w/app/enums/level.scxml")
            ),
            "../enums/level.scxml"
        );
        assert_eq!(
            relative_key(
                Path::new("/w/app/./machines"),
                Path::new("/w/app/x/../machines/a.scxml")
            ),
            "a.scxml"
        );
        // A standard document is its folded library name wherever it is read from.
        assert_eq!(
            document_key(
                Path::new("/anywhere"),
                Path::new("sce:std/merge/../merge/lww_cell.scxml")
            ),
            "sce:std/merge/lww_cell.scxml"
        );
    }

    #[test]
    fn the_committed_machine_names_each_enum_by_its_document() {
        let statechart = fixtures().join("static_schema_pending.scxml");
        let model = crate::parser::SCXMLParser::new()
            .parse_file(statechart.to_str().expect("a UTF-8 path"))
            .unwrap_or_else(|e| panic!("the statechart parses: {e:?}"));
        // The statechart's own import, and the schema's import for its field:
        // two aliases or paths, one document, one key.
        let own = &model.enum_documents.by_alias["SignalLevel"];
        let field = &model.enum_documents.by_field["signal.raised"]["level"];
        assert_eq!(own.key, "enum_signal_level.scxml");
        assert_eq!(field.key, "enum_signal_level.scxml");
        assert_eq!(field.model.revision(), 2);
    }

    /// An enum document whose root carries `root` and whose variants are
    /// `month` and `week` of the first revision and then `added`, read as the
    /// forge parser reads it: the enum, or the refusal's validation error,
    /// boxed as the parser hands it over (the error is past
    /// `clippy::result_large_err`).
    fn parse_enum_doc(root: &str, added: &str) -> Result<EnumModel, Box<ValidationError>> {
        let xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="enum" name="view_mode" sce:underlying-type="uint8"{root}>
  <datamodel>
    <data id="variants">
      <sce:variant name="month" value="0"/>
      <sce:variant name="week" value="1"/>
      {added}
    </data>
  </datamodel>
</scxml>"#
        );
        let label = crate::DocumentLabel {
            identifier: "view_mode",
            diagnostic_label: "view_mode.scxml",
        };
        match crate::forge::parser::parse_forge_with_imports(&xml, label) {
            Ok(Some(parsed)) => match parsed.document {
                ForgeDocument::Enum(model) => Ok(model),
                other => panic!("read as another kind: {other:?}"),
            },
            Ok(None) => panic!("not read as a forge document"),
            Err(located) => match located.error {
                ForgeError::Validation(validation) => Err(validation),
                other => panic!("refused, but not by validation: {other:?}"),
            },
        }
    }

    #[test]
    fn an_enum_reads_its_revision_and_the_variants_a_later_one_added() {
        let grown = parse_enum_doc(
            r#" sce:revision="2""#,
            r#"<sce:variant name="agenda" value="2" sce:since="2"/>"#,
        )
        .expect("an enum that grew reads");
        assert_eq!(grown.revision, Some(2));
        let since: Vec<(&str, Option<u32>)> = grown
            .variants
            .iter()
            .map(|v| (v.name.as_str(), v.since))
            .collect();
        assert_eq!(
            since,
            vec![("month", None), ("week", None), ("agenda", Some(2))]
        );
        let first = parse_enum_doc("", "").expect("an enum of one revision reads");
        assert_eq!(first.revision, None);
        assert_eq!(first.revision(), 1);
    }

    #[test]
    fn a_variant_since_outside_the_revisions_the_enum_has_is_refused() {
        for (root, since) in [
            (r#" sce:revision="2""#, "3"),
            (r#" sce:revision="2""#, "1"),
            (r#" sce:revision="2""#, "later"),
            ("", "2"),
        ] {
            let added = format!(r#"<sce:variant name="agenda" value="2" sce:since="{since}"/>"#);
            let error = parse_enum_doc(root, &added).expect_err("no such revision added it");
            assert_eq!(
                refused(&error),
                ("rule", "sce:since".to_string()),
                "{root} {since}"
            );
        }
        let error = parse_enum_doc(r#" sce:revision="zero""#, "").expect_err("no such revision");
        assert_eq!(refused(&error), ("rule", "sce:revision".to_string()));
    }

    #[test]
    fn an_enum_default_must_name_a_variant_of_its_enum() {
        let variants = |alias: &str| {
            (alias == "ViewMode").then(|| vec!["month".to_string(), "week".to_string()])
        };
        let named = schema(vec![field("layout", view_mode(), Some(2), Some("week"))]);
        assert!(check_schema(&named, &variants, "schema_revised.scxml").is_ok());

        let unnamed = schema(vec![field(
            "layout",
            view_mode(),
            Some(2),
            Some("fortnight"),
        )]);
        let refusal = check_schema(&unnamed, &variants, "schema_revised.scxml")
            .expect_err("a name the enum does not declare is no value of it");
        let validation = match refusal.error {
            ForgeError::Validation(validation) => *validation,
            other => panic!("refused as {other:?}"),
        };
        match validation {
            ValidationError::InvalidAttribute {
                attr,
                value,
                allowed,
                ..
            } => {
                assert_eq!(attr, "sce:default");
                assert_eq!(value, "fortnight");
                assert_eq!(allowed, vec!["month".to_string(), "week".to_string()]);
            }
            other => panic!("refused as {other:?}"),
        }

        // An enum the import pass could not read is that pass's to report.
        let unread = |_: &str| None;
        assert!(check_schema(&unnamed, &unread, "schema_revised.scxml").is_ok());
    }

    /// An event schema whose root carries `root` and whose datamodel holds a
    /// `level` of the first revision and then `added`, read as the parser
    /// reads it: the schema, or the refusal's validation error, boxed as the
    /// parser hands it over (the error is past `clippy::result_large_err`).
    fn parse_schema(root: &str, added: &str) -> Result<EventSchemaModel, Box<ValidationError>> {
        let xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="schema_revised"
       sce:event-name="reading.revised"{root}>
  <datamodel>
    <data id="level" sce:type="uint8" sce:direction="in"/>
    {added}
  </datamodel>
</scxml>"#
        );
        let label = crate::DocumentLabel {
            identifier: "schema_revised",
            diagnostic_label: "schema_revised.scxml",
        };
        match crate::forge::parser::parse_forge_with_imports(&xml, label) {
            Ok(Some(parsed)) => match parsed.document {
                ForgeDocument::EventSchema(schema) => Ok(schema),
                other => panic!("read as another kind: {other:?}"),
            },
            Ok(None) => panic!("not read as a forge document"),
            Err(located) => match located.error {
                ForgeError::Validation(validation) => Err(validation),
                other => panic!("refused, but not by validation: {other:?}"),
            },
        }
    }

    /// The attribute a validation error refuses, and which of the three
    /// shapes refuses it.
    fn refused(error: &ValidationError) -> (&'static str, String) {
        match error {
            ValidationError::AttributeRuleViolated { attr, .. } => ("rule", attr.clone()),
            ValidationError::InvalidAttribute { attr, .. } => ("one-of", attr.clone()),
            ValidationError::MissingAttribute { attr, .. } => ("missing", attr.clone()),
            other => panic!("refused as {other:?}"),
        }
    }

    #[test]
    fn a_schema_reads_its_revision_and_the_fields_a_later_one_added() {
        let schema = parse_schema(
            r#" sce:revision="3""#,
            r#"<data id="urgent" sce:type="bool" sce:direction="in" sce:since="3" sce:default="true"/>"#,
        )
        .expect("a revised schema reads");
        assert_eq!(schema.revision, Some(3));
        assert_eq!(schema.revision(), 3);
        assert_eq!(schema.fields[0].since, None);
        assert_eq!(schema.fields[0].default, None);
        assert_eq!(schema.fields[1].since, Some(3));
        assert_eq!(schema.fields[1].default.as_deref(), Some("true"));
        assert!(schema.fields[1].default_spelling.is_some());

        let first = parse_schema("", "").expect("a schema of one revision reads");
        assert_eq!(first.revision, None);
        assert_eq!(first.revision(), 1);
    }

    #[test]
    fn a_revision_that_is_not_a_positive_integer_is_refused() {
        for written in ["0", "two", "-1"] {
            let error = parse_schema(&format!(r#" sce:revision="{written}""#), "")
                .expect_err("no such revision");
            assert_eq!(
                refused(&error),
                ("rule", "sce:revision".to_string()),
                "{written}"
            );
        }
    }

    #[test]
    fn a_since_outside_the_revisions_the_schema_has_is_refused() {
        let added = |since: &str| {
            format!(
                r#"<data id="urgent" sce:type="bool" sce:direction="in" sce:since="{since}" sce:default="true"/>"#
            )
        };
        for (root, since) in [
            (r#" sce:revision="3""#, "4"),
            (r#" sce:revision="3""#, "1"),
            (r#" sce:revision="3""#, "soon"),
            ("", "2"),
        ] {
            let error = parse_schema(root, &added(since)).expect_err("no such revision added it");
            assert_eq!(
                refused(&error),
                ("rule", "sce:since".to_string()),
                "{root} {since}"
            );
        }
    }

    #[test]
    fn a_default_and_a_since_each_need_the_other() {
        let error = parse_schema(
            r#" sce:revision="2""#,
            r#"<data id="urgent" sce:type="bool" sce:direction="in" sce:default="true"/>"#,
        )
        .expect_err("a field of the first revision takes no default");
        assert_eq!(refused(&error), ("rule", "sce:default".to_string()));

        let error = parse_schema(
            r#" sce:revision="2""#,
            r#"<data id="urgent" sce:type="bool" sce:direction="in" sce:since="2"/>"#,
        )
        .expect_err("a field a later revision added takes a default");
        assert_eq!(refused(&error), ("missing", "sce:default".to_string()));
    }

    #[test]
    fn a_default_that_is_no_value_of_its_field_is_refused_where_the_schema_is_read() {
        let error = parse_schema(
            r#" sce:revision="2""#,
            r#"<data id="urgent" sce:type="bool" sce:direction="in" sce:since="2" sce:default="yes"/>"#,
        )
        .expect_err("a bool is true or false");
        assert_eq!(refused(&error), ("one-of", "sce:default".to_string()));

        let error = parse_schema(
            r#" sce:revision="2""#,
            r#"<data id="count" sce:type="uint8" sce:direction="in" sce:since="2" sce:default="300"/>"#,
        )
        .expect_err("300 is past a uint8");
        assert_eq!(refused(&error), ("rule", "sce:default".to_string()));
    }

    #[test]
    fn a_revision_on_another_kind_is_read_by_nothing_and_refused() {
        // An event schema and an enum grow by revisions (docs/adr/0044); a
        // filter does not, and nothing of its reads `sce:revision`.
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="filter" sce:revision="2">
  <datamodel>
    <data id="rawButton" sce:type="bool" sce:direction="in"/>
    <data id="stable" sce:type="bool" sce:direction="out"
          sce:filter="debounce" sce:window="3"/>
  </datamodel>
</scxml>"#;
        let label = crate::DocumentLabel {
            identifier: "debounce",
            diagnostic_label: "debounce.scxml",
        };
        let refusal = crate::forge::parser::parse_forge_with_imports(xml, label)
            .expect_err("a filter has no revision");
        assert!(
            matches!(
                &refusal.error,
                ForgeError::Validation(validation)
                    if matches!(**validation, ValidationError::UnreadSceAttributes(_))
            ),
            "{:?}",
            refusal.error
        );
    }

    /// A statechart of data model `datamodel` that imports `schema` from beside
    /// it, parsed as a file is; the import is on row 4.
    fn parse_importing(
        datamodel: &str,
        schema: &str,
    ) -> Result<crate::model::SCXMLModel, Located<ForgeError>> {
        let dir = tempfile::tempdir().expect("a temporary directory");
        std::fs::write(dir.path().join("schema_grown.scxml"), schema).expect("the schema");
        let statechart = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="reading" datamodel="{datamodel}" name="importer">
  <sce:import kind="event-schema" src="schema_grown.scxml" as="Grown"/>
  <state id="reading"/>
</scxml>"#
        );
        let path = dir.path().join("importer.scxml");
        std::fs::write(&path, statechart).expect("the statechart");
        crate::parser::SCXMLParser::new().parse_file(path.to_str().expect("a UTF-8 path"))
    }

    fn grown_schema(root: &str, added: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="schema_grown"
       sce:event-name="reading.grown"{root}>
  <datamodel>
    <data id="level" sce:type="uint8" sce:direction="in"/>
    {added}
  </datamodel>
</scxml>"#
        )
    }

    #[test]
    fn a_schema_that_grew_is_refused_at_its_import_outside_sce_static() {
        let added = r#"<data id="urgent" sce:type="bool" sce:direction="in" sce:since="2" sce:default="true"/>"#;
        let grown = grown_schema(r#" sce:revision="2""#, added);
        // A revision past the first alone grows a schema, as an added field does.
        let only_revised = grown_schema(r#" sce:revision="2""#, "");
        for schema in [&grown, &only_revised] {
            let refusal = parse_importing("ecmascript", schema)
                .expect_err("one event would be read two ways by two engines");
            assert_eq!(refusal.location.line, Some(4));
            match refusal.error {
                ForgeError::Validation(validation) => match *validation {
                    ValidationError::AttributeRuleViolated { attr, value, .. } => {
                        assert_eq!(attr, "src");
                        assert_eq!(value, "schema_grown.scxml");
                    }
                    other => panic!("refused as {other:?}"),
                },
                other => panic!("refused as {other:?}"),
            }
        }

        // A sce-static machine reads every field through the lift, the default
        // included, and a schema of one revision is read under any data model.
        assert!(parse_importing("sce-static", &grown).is_ok());
        assert!(parse_importing("ecmascript", &grown_schema("", "")).is_ok());
    }

    // ── The committed fixtures, through the entry points the generator uses ──
    //
    // The cases above build their documents here; these read the ones the
    // regeneration scripts and the scenario drivers read, through the same
    // calls `sce-codegen check` and `generate` make: the statechart parse
    // (which parses the imported schema as a sibling), the forge parse and
    // compile of the schema alone, and the orchestrator for every backend that
    // lowers the data model. Each walks every check a document meets on its
    // way to generated code, the forge roster of known `sce:` names included,
    // so a check none of the cases above reaches cannot refuse the fixtures
    // while those cases pass.

    fn fixtures() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/static_datamodel")
    }

    /// The defaults of `schema_revised.scxml` as every backend's string
    /// literal spells them.
    const FIXTURE_DEFAULTS: &str = r#""{\"layout\":\"week\",\"urgent\":true}""#;

    #[test]
    fn the_committed_schema_that_grew_is_read_where_check_reads_it() {
        let statechart = fixtures().join("static_schema_revision.scxml");
        let model = crate::parser::SCXMLParser::new()
            .parse_file(statechart.to_str().expect("a UTF-8 path"))
            .unwrap_or_else(|e| panic!("the statechart parses: {e:?}"));
        let schema = model
            .imported_event_schemas
            .get("reading.revised")
            .expect("the statechart resolves the schema it imports");
        assert_eq!(schema.revision, Some(3));
        let added: Vec<(&str, Option<u32>, Option<&str>)> = schema
            .fields
            .iter()
            .map(|f| (f.id.as_str(), f.since, f.default.as_deref()))
            .collect();
        assert_eq!(
            added,
            vec![
                ("level", None, None),
                ("layout", Some(2), Some("week")),
                ("urgent", Some(3), Some("true")),
            ]
        );
        assert_eq!(
            defaults_wire(schema).map(|d| serde_json::to_string(&d).expect("JSON")),
            Some(FIXTURE_DEFAULTS.to_string())
        );

        let path = fixtures().join("schema_revised.scxml");
        let content = std::fs::read_to_string(&path).expect("the schema");
        let label = crate::DocumentLabel {
            identifier: "schema_revised",
            diagnostic_label: "schema_revised.scxml",
        };
        let parsed = crate::forge::parser::parse_forge_with_imports(&content, label)
            .unwrap_or_else(|e| panic!("the schema parses as a forge document: {e:?}"))
            .expect("the schema is a forge document");
        crate::compile_forge_from_parsed(
            &parsed,
            label,
            crate::generator::Language::Rust,
            &fixtures(),
            &crate::ForgeCompileOptions::default(),
        )
        .unwrap_or_else(|e| panic!("the schema compiles as a document of its own: {e:?}"));
    }

    #[test]
    fn the_committed_machine_is_generated_with_its_defaults_on_every_backend_that_lowers_it() {
        use crate::generator::Language;
        let statechart = fixtures().join("static_schema_revision.scxml");
        let schema = fixtures().join("schema_revised.scxml");
        let view_mode = fixtures().join("enum_view_mode.scxml");
        let mut generated = 0;
        for &language in Language::ALL {
            if !crate::generator::lowers_static_datamodel(language) {
                continue;
            }
            let lift = match language {
                Language::Rust => "PayloadFields::decode_with_defaults(data, ",
                Language::C11 => "sce_payload_decode_with_defaults(_data, ",
                Language::Go => "sce.LiftPayloadWithDefaults(meta.Data, ",
                Language::Cpp => "EventPayloadFields::decode(meta.data, ",
                Language::Kotlin => "EventPayload.decode(metadata.data, ",
                Language::Python => "_lift_typed_payload(",
                Language::Swift => {
                    unreachable!("swift lowers no sce-static, so it is skipped above")
                }
            };
            // Go has no bare import form: it asks where the generated packages live.
            let options = crate::ForgeCompileOptions {
                go_module_prefix: matches!(language, Language::Go)
                    .then(|| "example.com/consumer/generated".to_string()),
                ..crate::ForgeCompileOptions::default()
            };
            let outputs = crate::compile_scxml_with_imports(
                &[statechart.as_path()],
                &[schema.as_path(), view_mode.as_path()],
                &crate::find_template_dir_for(language),
                language,
                &options,
                None,
            )
            .unwrap_or_else(|e| panic!("{language:?}: the machine generates: {e:?}"));
            let text: String = outputs
                .iter()
                .flat_map(|(_, output)| output.files.iter().map(|(_, content)| content.as_str()))
                .collect();
            assert!(text.contains(lift), "{language:?}: no lift `{lift}`");
            assert!(
                text.contains(FIXTURE_DEFAULTS),
                "{language:?}: the lift is not handed {FIXTURE_DEFAULTS}"
            );
            generated += 1;
        }
        assert!(
            generated >= 6,
            "only {generated} backends generated the machine"
        );
    }

    #[test]
    fn the_committed_machine_is_lowered_for_the_interpreter_with_its_defaults() {
        let statechart = fixtures().join("static_schema_revision.scxml");
        let lowered = crate::forge::static_js::lower_file_set(
            statechart.to_str().expect("a UTF-8 path"),
            Vec::new(),
        )
        .unwrap_or_else(|e| panic!("the machine lowers for the Interpreter: {e:?}"));
        // The attributes as the Interpreter reads them, entities decoded.
        let mut expressions = String::new();
        for document in &lowered {
            let parsed = roxmltree::Document::parse(&document.text).expect("lowered XML");
            for node in parsed.descendants().filter(|n| n.is_element()) {
                for attribute in node.attributes() {
                    expressions.push_str(attribute.value());
                    expressions.push('\n');
                }
            }
        }
        for read in [
            "SceStatic.field(_event.data, 'layout', ['month', 'week', 'day', 'agenda_list'], \"week\")",
            "SceStatic.field(_event.data, 'urgent', 'bool', true)",
            "SceStatic.field(_event.data, 'level', 'uint8')",
        ] {
            assert!(expressions.contains(read), "no `{read}` in:\n{expressions}");
        }
    }

    // ── An enum that grew, through the entry points the generator uses ──
    //
    // `sce:revision` on an enum's root and `sce:since` on its `<sce:variant>`
    // pass four gates before a reader sees them: the grammar
    // (`schemas/sce-forge-ext.xsd`), the forge roster of known `sce:` names, the
    // ledger of what a parse read, and the reader itself. A case that builds its
    // own document can pass while one of the first three refuses every real one,
    // which is how the roster refused `sce:revision` on another branch while its
    // cases passed. These read the committed fixtures the regeneration scripts
    // read, through the calls `sce-codegen check` and `generate` make.

    #[test]
    fn the_committed_enum_that_grew_is_valid_where_the_grammar_is_checked() {
        let path = fixtures().join("enum_signal_level.scxml");
        let content = std::fs::read_to_string(&path).expect("the enum");
        // The grammar on its own, so a skipped validation cannot pass as one.
        #[cfg(feature = "xsd")]
        assert!(
            matches!(
                crate::forge::xsd_validator::validate_or_skip(&content, "enum_signal_level.scxml"),
                Ok(crate::forge::xsd_validator::XsdOutcome::Validated)
            ),
            "the grammar refused, or did not check, the enum that grew"
        );
        let label = crate::DocumentLabel {
            identifier: "enum_signal_level",
            diagnostic_label: "enum_signal_level.scxml",
        };
        let parsed = crate::forge::parser::parse_forge_with_imports(&content, label)
            .unwrap_or_else(|e| panic!("the enum parses as a forge document: {e:?}"))
            .expect("the enum is a forge document");
        let ForgeDocument::Enum(model) = &parsed.document else {
            panic!("read as another kind: {:?}", parsed.document);
        };
        assert_eq!(model.revision, Some(2));
        let since: Vec<(&str, Option<u32>)> = model
            .variants
            .iter()
            .map(|v| (v.name.as_str(), v.since))
            .collect();
        assert_eq!(
            since,
            vec![("low", None), ("high", None), ("critical", Some(2))]
        );
        crate::compile_forge_from_parsed(
            &parsed,
            label,
            crate::generator::Language::Rust,
            &fixtures(),
            &crate::ForgeCompileOptions::default(),
        )
        .unwrap_or_else(|e| panic!("the enum compiles as a document of its own: {e:?}"));
    }

    #[test]
    fn the_committed_machine_of_an_enum_that_grew_is_generated_on_every_backend_that_lowers_it() {
        use crate::generator::Language;
        let statechart = fixtures().join("static_schema_pending.scxml");
        // The statechart parse, which reads the imported schema and enum as
        // siblings, as `check` does.
        let model = crate::parser::SCXMLParser::new()
            .parse_file(statechart.to_str().expect("a UTF-8 path"))
            .unwrap_or_else(|e| panic!("the statechart parses: {e:?}"));
        let level = model
            .imported_enums
            .get("SignalLevel")
            .expect("the statechart resolves the enum it imports");
        assert_eq!(level.revision(), 2);
        let mut generated = 0;
        for &language in Language::ALL {
            if !crate::generator::lowers_static_datamodel(language) {
                continue;
            }
            // Go has no bare import form: it asks where the generated packages live.
            let options = crate::ForgeCompileOptions {
                go_module_prefix: matches!(language, Language::Go)
                    .then(|| "example.com/consumer/generated".to_string()),
                ..crate::ForgeCompileOptions::default()
            };
            crate::compile_scxml_with_imports(
                &[statechart.as_path()],
                &[
                    fixtures().join("schema_signal.scxml").as_path(),
                    fixtures().join("enum_signal_level.scxml").as_path(),
                ],
                &crate::find_template_dir_for(language),
                language,
                &options,
                None,
            )
            .unwrap_or_else(|e| panic!("{language:?}: the machine generates: {e:?}"));
            generated += 1;
        }
        assert!(
            generated >= 6,
            "only {generated} backends generated the machine"
        );
        // What a saved state of it names: the first revisions in the shape, the
        // later ones by the schema's event and the enum's key.
        let mut model = model;
        crate::analyzer::analyze(&mut model, "static_schema_pending.scxml");
        let shape = crate::forge::static_lowering::lower_rust(&mut model, "StaticSchemaPending")
            .expect("lowers")
            .saved_shape
            .expect("the machine saves");
        assert_eq!(
            shape.first, "f7e359f42783c9931d2c1dbf44f80fee16a403f0e56b70bbc26ff6874c901834",
            "the first shape the shared fixture `saved/format1/static_schema_pending_first.json` names"
        );
        let records: Vec<(&str, u32, usize)> = shape
            .records
            .iter()
            .map(|r| (r.event.as_str(), r.revision, r.revisions.len()))
            .collect();
        assert_eq!(records, [("signal.raised", 2, 1)]);
        let enums: Vec<(&str, u32, usize, Vec<&str>)> = shape
            .enums
            .iter()
            .map(|e| {
                (
                    e.key.as_str(),
                    e.revision,
                    e.revisions.len(),
                    e.added.iter().map(|v| v.name.as_str()).collect(),
                )
            })
            .collect();
        // The enum by its document, relative to the machine's own: the alias the
        // machine and the schema read it by plays no part.
        assert_eq!(
            enums,
            [("enum:enum_signal_level.scxml", 2, 1, vec!["critical"])]
        );
    }

    /// The text of the first string literal in `text` after `marker`, which
    /// must occur: what a generated constant introduced by `marker` holds.
    fn literal_after(text: &str, marker: &str, language: &str) -> String {
        let at = text
            .find(marker)
            .unwrap_or_else(|| panic!("{language}: no `{marker}` in the generated machine"));
        let rest = &text[at + marker.len()..];
        let open = rest.find('"').expect("a string literal follows") + 1;
        let close = open + rest[open..].find('"').expect("the literal closes");
        rest[open..close].to_string()
    }

    /// Every string literal in `text` between `start` and the first `end` after
    /// it: the elements of a generated list constant.
    fn literals_between(text: &str, start: &str, end: &str, language: &str) -> Vec<String> {
        let at = text
            .find(start)
            .unwrap_or_else(|| panic!("{language}: no `{start}` in the generated machine"));
        let rest = &text[at + start.len()..];
        let region = &rest[..rest.find(end).expect("the list ends")];
        region
            .split('"')
            .skip(1)
            .step_by(2)
            .map(str::to_string)
            .collect()
    }

    #[test]
    fn every_backend_declares_the_one_format_2_shape_and_enum_keys_the_generator_computed() {
        use crate::generator::Language;
        let statechart = fixtures().join("static_schema_pending.scxml");
        let mut model = crate::parser::SCXMLParser::new()
            .parse_file(statechart.to_str().expect("a UTF-8 path"))
            .unwrap_or_else(|e| panic!("the statechart parses: {e:?}"));
        crate::analyzer::analyze(&mut model, "static_schema_pending.scxml");
        let shape = crate::forge::static_lowering::lower_rust(&mut model, "StaticSchemaPending")
            .expect("lowers")
            .saved_shape
            .expect("the machine saves");
        // The machine holds an enum, so its format-2 shape is not its format-1 one:
        // a backend that wrote the format-1 text under the format-2 name is seen.
        assert_ne!(shape.format2_first, shape.first);
        let expected = (
            shape.format2_first.clone(),
            shape.format2_revised.clone(),
            vec!["enum:enum_signal_level.scxml".to_string()],
        );

        // Per backend: what introduces the format-2 shape, the revised one, and
        // the list of enum keys, with what ends that list.
        type Markers = (&'static str, &'static str, &'static str, &'static str);
        let markers = |language: Language| -> Markers {
            match language {
                Language::Rust => (
                    "format2_first: ",
                    "format2_revised: ",
                    "enums: &[",
                    "const HISTORIES",
                ),
                Language::Kotlin => (
                    "format2First = ",
                    "format2Revised = ",
                    "enums = listOf(",
                    "fun save(",
                ),
                Language::Go => (
                    "SavedShapeFormat2 ",
                    "SavedShapeFormat2Revised ",
                    "SavedEnumKeys = [...]string{",
                    "}",
                ),
                Language::Python => (
                    "SAVED_SHAPE_FORMAT2 = ",
                    "SAVED_SHAPE_FORMAT2_REVISED = ",
                    "SAVED_ENUM_KEYS = (",
                    ")",
                ),
                Language::Cpp => (
                    "SAVED_SHAPE_FORMAT2 = ",
                    "SAVED_SHAPE_FORMAT2_REVISED = ",
                    "SAVED_ENUM_KEYS = {",
                    "};",
                ),
                Language::C11 => (
                    "_SAVED_SHAPE_FORMAT2 ",
                    "_SAVED_SHAPE_FORMAT2_REVISED ",
                    "_SAVED_ENUM_KEY_COUNT",
                    "\n\n",
                ),
            }
        };
        let mut declared = 0;
        for &language in Language::ALL {
            if !crate::generator::lowers_static_datamodel(language) {
                continue;
            }
            let name = format!("{language:?}");
            // Go has no bare import form: it asks where the generated packages live.
            let options = crate::ForgeCompileOptions {
                go_module_prefix: matches!(language, Language::Go)
                    .then(|| "example.com/consumer/generated".to_string()),
                ..crate::ForgeCompileOptions::default()
            };
            let outputs = crate::compile_scxml_with_imports(
                &[statechart.as_path()],
                &[
                    fixtures().join("schema_signal.scxml").as_path(),
                    fixtures().join("enum_signal_level.scxml").as_path(),
                ],
                &crate::find_template_dir_for(language),
                language,
                &options,
                None,
            )
            .unwrap_or_else(|e| panic!("{name}: the machine generates: {e:?}"));
            let text: String = outputs
                .iter()
                .flat_map(|(_, output)| output.files.iter().map(|(_, content)| content.as_str()))
                .collect();
            let (first, revised, keys_start, keys_end) = markers(language);
            let mut keys: Vec<String> = literals_between(&text, keys_start, keys_end, &name)
                .into_iter()
                .filter(|literal| literal.starts_with("enum:"))
                .collect();
            keys.dedup();
            let found = (
                literal_after(&text, first, &name),
                literal_after(&text, revised, &name),
                keys,
            );
            assert_eq!(
                found, expected,
                "{name}: the format-2 shape and enum keys are not the generator's text"
            );
            declared += 1;
        }
        assert!(declared >= 6, "only {declared} backends declared the texts");
    }
}
