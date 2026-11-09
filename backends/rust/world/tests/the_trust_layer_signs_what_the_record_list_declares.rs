// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The trust layer (sce-rust-trust) secures the standard shared record list
// (sce:std/shared/record_list.scxml, docs/adr/0042) and no other document: the event a signature is
// carried by, a change of the list (`record_list.change`, docs/adr/0085, section 1), and the fields
// it is signed with, each by its type, are the ones the list's event schemas declare: `record_change`,
// and `record_op` for each of its operations. The layer states them in a table of its own
// (`message::SIGNED`), since a host reads no document when it signs; this test holds that table to
// the schemas, so that a field the part adds, moves, renames or retypes is a failure here and not a
// signature that leaves it out or writes it otherwise.
//
// It lives in this crate because this crate reads both: the schemas through `sce-build`, which
// parses them as every generator does, and the table through `sce-rust-trust`.

use std::path::Path;

use sce_build::forge::model::{ForgeDocument, ForgeImport, SceType, SchemaField};
use sce_build::forge::parser::parse_forge_with_imports;
use sce_build::forge::value_type::TypeOf;
use sce_build::DocumentLabel;
use sce_rust_trust::cbor::{Integer, RecordType, Shape, ValueType};
use sce_rust_trust::message;

/// What a standard schema of the shared list declares: its event name, its fields with their
/// types, and the imports a record of it names by alias.
struct Declared {
    event_name: String,
    fields: Vec<SchemaField>,
    imports: Vec<ForgeImport>,
}

fn declared(name: &str) -> Declared {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../stdlib/shared")
        .join(format!("{name}.scxml"));
    let text =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let parsed = parse_forge_with_imports(&text, DocumentLabel::symmetric(name))
        .unwrap_or_else(|e| panic!("{name} is read: {e:?}"))
        .unwrap_or_else(|| panic!("{name} is a forge document"));
    let ForgeDocument::EventSchema(schema) = parsed.document else {
        panic!("{name} is an event schema");
    };
    Declared {
        event_name: schema.event_name,
        fields: schema.fields,
        imports: parsed.imports,
    }
}

/// The integer of the trust layer's table that a scalar of the schema is, if it is one.
fn integer_of(scalar: &SceType) -> Option<Integer> {
    Some(match scalar {
        SceType::Uint8 => Integer::Uint8,
        SceType::Uint16 => Integer::Uint16,
        SceType::Uint32 => Integer::Uint32,
        SceType::Uint64 => Integer::Uint64,
        SceType::Int8 => Integer::Int8,
        SceType::Int16 => Integer::Int16,
        SceType::Int32 => Integer::Int32,
        SceType::Int64 => Integer::Int64,
        _ => return None,
    })
}

/// `signed`, the type the table signs a field with, is the type the schema gives it: the same
/// integer, a text, a list of the same element, or a record held to the schema the import names.
/// A bound is not compared: the build's bounds are a deployment's to move, and the table signs any
/// value of the shape (`message`, the reason it holds none).
fn same_type(
    signed: &ValueType,
    schema: &TypeOf<sce_build::forge::value_type::Bound>,
    of: &Declared,
    place: &str,
) {
    let ValueType::Required(shape) = signed else {
        panic!("{place}: the table signs it as optional, and the schema does not declare it so");
    };
    match (shape, schema) {
        (Shape::Integer(integer), TypeOf::Scalar(scalar)) => assert_eq!(
            Some(*integer),
            integer_of(scalar),
            "{place}: the integer it is"
        ),
        (Shape::Text { .. }, TypeOf::Text { .. }) => {}
        (Shape::List { elem, .. }, TypeOf::List { elem: declared, .. }) => {
            same_type(elem, declared, of, &format!("{place}[]"))
        }
        (Shape::Record(record), TypeOf::Record { alias }) => {
            let import = of
                .imports
                .iter()
                .find(|import| import.alias == *alias)
                .unwrap_or_else(|| panic!("{place}: the schema imports `{alias}`"));
            let name = import
                .src
                .rsplit('/')
                .next()
                .and_then(|file| file.strip_suffix(".scxml"))
                .unwrap_or_else(|| panic!("{place}: `{}` names a document", import.src));
            holds_to_its_schema(record, name);
        }
        (shape, declared) => {
            panic!("{place}: the table signs {shape:?} where the schema declares {declared:?}")
        }
    }
}

/// `record` is signed with every field `schema` declares, in its order, each read as its type is,
/// with the revision that added it and the default a value that leaves it out takes. Gives the
/// event the schema names.
fn holds_to_its_schema(record: &RecordType, schema: &str) -> String {
    let of = declared(schema);
    let signed: Vec<&str> = record.fields.iter().map(|field| field.id).collect();
    let declared: Vec<&str> = of.fields.iter().map(|field| field.id.as_str()).collect();
    assert_eq!(
        signed, declared,
        "the fields of {schema}, in the order it declares them"
    );
    for (field, schema_field) in record.fields.iter().zip(&of.fields) {
        let place = format!("{schema}.{}", field.id);
        same_type(&field.ty, &schema_field.sce_type, &of, &place);
        assert_eq!(
            field.since,
            schema_field.since.unwrap_or(1),
            "{place}: the revision that added it"
        );
        assert_eq!(
            field.default.is_some(),
            schema_field.default.is_some(),
            "{place}: whether a value that leaves it out is read as a default"
        );
    }
    of.event_name
}

#[test]
fn a_change_is_signed_with_every_field_record_change_and_record_op_declare_in_their_order() {
    let event = holds_to_its_schema(message::CHANGE.payload, "record_change");
    assert_eq!(message::CHANGE.name, event, "the event record_change names");
    assert!(
        message::CHANGE
            .payload
            .fields
            .iter()
            .any(|field| field.id == message::CHANGE.author),
        "the author is one of its fields"
    );
    holds_to_its_schema(&message::OP, "record_op");
}

/// What names a change, its subject, is its author and its stamp, the fields of `record_change`
/// that the list's dot is made of (docs/adr/0046, section 1), each of the type the schema gives it.
#[test]
fn a_change_is_named_by_the_fields_of_its_dot() {
    let names: Vec<&str> = message::CHANGE
        .names
        .fields
        .iter()
        .map(|field| field.id)
        .collect();
    assert_eq!(names, ["by", "wallTime", "counter"]);
    for name in message::CHANGE.names.fields.iter() {
        let payload = message::CHANGE
            .payload
            .fields
            .iter()
            .find(|field| field.id == name.id)
            .unwrap_or_else(|| panic!("{} is a field of the change", name.id));
        assert_eq!(name.ty, payload.ty, "{}: one type in both", name.id);
    }
}

/// The events the layer signs and raises are the standard list's, and only a change is signed:
/// the one-operation events of the parts a change was sent as before are not schemas of the list.
#[test]
fn every_event_the_layer_names_is_one_the_standard_list_declares() {
    let names: Vec<&str> = message::SIGNED.iter().map(|event| event.name).collect();
    assert_eq!(names, message::SIGNED_EVENTS);
    assert_eq!(names, [declared("record_change").event_name]);
    for retired in ["record_write", "record_supersede"] {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../stdlib/shared")
            .join(format!("{retired}.scxml"));
        assert!(!path.exists(), "{retired} is retired from the list");
    }

    let role = declared("record_role");
    assert_eq!(message::ROLE_EVENT, role.event_name);
    let written: Vec<&str> = role.fields.iter().map(|field| field.id.as_str()).collect();
    assert_eq!(
        written,
        ["node", "role"],
        "what a host's role event carries"
    );
}
