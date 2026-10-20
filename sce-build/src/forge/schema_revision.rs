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

use std::path::Path;

use crate::forge::error::{ForgeError, Located, ValidationError};
use crate::forge::model::{
    EventSchemaModel, ForgeDocument, ForgeField, NumericLiteral, ParsedForge, SceType,
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
    /// reads it: the schema, or the refusal's validation error.
    fn parse_schema(root: &str, added: &str) -> Result<EventSchemaModel, ValidationError> {
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
                ForgeError::Validation(validation) => Err(*validation),
                other => panic!("refused, but not by validation: {other:?}"),
            },
        }
    }

    /// The attribute a validation error refuses, and which of the three
    /// shapes refuses it.
    fn refused(error: ValidationError) -> (&'static str, String) {
        match error {
            ValidationError::AttributeRuleViolated { attr, .. } => ("rule", attr),
            ValidationError::InvalidAttribute { attr, .. } => ("one-of", attr),
            ValidationError::MissingAttribute { attr, .. } => ("missing", attr),
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
                refused(error),
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
                refused(error),
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
        assert_eq!(refused(error), ("rule", "sce:default".to_string()));

        let error = parse_schema(
            r#" sce:revision="2""#,
            r#"<data id="urgent" sce:type="bool" sce:direction="in" sce:since="2"/>"#,
        )
        .expect_err("a field a later revision added takes a default");
        assert_eq!(refused(error), ("missing", "sce:default".to_string()));
    }

    #[test]
    fn a_default_that_is_no_value_of_its_field_is_refused_where_the_schema_is_read() {
        let error = parse_schema(
            r#" sce:revision="2""#,
            r#"<data id="urgent" sce:type="bool" sce:direction="in" sce:since="2" sce:default="yes"/>"#,
        )
        .expect_err("a bool is true or false");
        assert_eq!(refused(error), ("one-of", "sce:default".to_string()));

        let error = parse_schema(
            r#" sce:revision="2""#,
            r#"<data id="count" sce:type="uint8" sce:direction="in" sce:since="2" sce:default="300"/>"#,
        )
        .expect_err("300 is past a uint8");
        assert_eq!(refused(error), ("rule", "sce:default".to_string()));
    }

    #[test]
    fn a_revision_on_another_kind_is_read_by_nothing_and_refused() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="enum" name="view_mode" sce:underlying-type="uint8"
       sce:revision="2">
  <datamodel>
    <data id="variants">
      <sce:variant name="month" value="0"/>
    </data>
  </datamodel>
</scxml>"#;
        let label = crate::DocumentLabel {
            identifier: "view_mode",
            diagnostic_label: "view_mode.scxml",
        };
        let refusal = crate::forge::parser::parse_forge_with_imports(xml, label)
            .expect_err("an enum has no revision");
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
}
