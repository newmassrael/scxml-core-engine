// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The trust layer (sce-rust-trust) secures the standard shared record list
// (sce:std/shared/record_list.scxml, docs/adr/0042) and no other document: the events a signature
// is carried by, and the fields each is signed with, in order, are the ones the list's event
// schemas declare. The layer states them in a table of its own (`message::SIGNED`), since a host
// reads no document when it signs; this test holds that table to the schemas, so that a field the
// part adds, moves or renames is a failure here and not a signature that leaves it out.
//
// It lives in this crate because this crate reads both: the schemas through `sce-build`, which
// parses them as every generator does, and the table through `sce-rust-trust`.

use std::path::Path;

use sce_build::forge::model::{EntryField, EntryType, ForgeDocument, SceType};
use sce_build::forge::parser::parse_forge_with_imports;
use sce_build::DocumentLabel;
use sce_rust_trust::message::{self, Kind, SignedEvent};

/// The event name and the fields a standard schema of the shared list declares, each one
/// entry: the trust layer signs a field one value at a time.
fn declared(name: &str) -> (String, Vec<EntryField>) {
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
    let schema = schema
        .entry_schema()
        .unwrap_or_else(|composed| panic!("{name}: {composed:?}"));
    (schema.event_name, schema.fields)
}

/// How the trust layer reads a field of the type the schema gives it.
fn kind_of(field_type: &EntryType) -> Option<Kind> {
    match field_type.scalar()? {
        SceType::Uint8 | SceType::Uint16 | SceType::Uint32 | SceType::Uint64 => {
            Some(Kind::Unsigned)
        }
        SceType::Int8 | SceType::Int16 | SceType::Int32 | SceType::Int64 => Some(Kind::Signed),
        SceType::String => Some(Kind::Text),
        _ => None,
    }
}

/// `event` is signed with every field `schema` declares, in its order, each read as its type
/// is, with the revision that added it and the default an event that leaves it out takes.
fn holds_to_its_schema(event: &SignedEvent, schema: &str) {
    let (event_name, fields) = declared(schema);
    assert_eq!(event.name, event_name, "the event {schema} names");
    let signed: Vec<&str> = event.fields.iter().map(|field| field.name).collect();
    let declared: Vec<&str> = fields.iter().map(|field| field.id.as_str()).collect();
    assert_eq!(
        signed, declared,
        "the fields of {schema}, in the order it declares them"
    );
    for (field, schema_field) in event.fields.iter().zip(&fields) {
        assert_eq!(
            Some(field.kind),
            kind_of(&schema_field.sce_type),
            "{schema}.{}: how it is read",
            field.name
        );
        assert_eq!(
            field.since,
            schema_field.since.unwrap_or(1),
            "{schema}.{}: the revision that added it",
            field.name
        );
        assert_eq!(
            field.default,
            schema_field.default.as_deref(),
            "{schema}.{}: what an event that leaves it out is read as",
            field.name
        );
    }
    assert!(
        signed.contains(&event.author),
        "{schema}: the author is one of its fields"
    );
}

#[test]
fn a_write_is_signed_with_every_field_record_write_declares_in_its_order() {
    holds_to_its_schema(&message::WRITE, "record_write");
}

#[test]
fn a_note_is_signed_with_every_field_record_supersede_declares_in_its_order() {
    holds_to_its_schema(&message::SUPERSEDE, "record_supersede");
}

/// The events the layer signs, refuses unsigned and raises are the standard list's, and only
/// they are signed.
#[test]
fn every_event_the_layer_names_is_one_the_standard_list_declares() {
    let names: Vec<&str> = message::SIGNED.iter().map(|event| event.name).collect();
    assert_eq!(names, message::SIGNED_EVENTS);
    assert_eq!(
        names,
        [declared("record_write").0, declared("record_supersede").0]
    );

    let (floor, fields) = declared("record_floor");
    assert_eq!(message::FLOOR_EVENT, floor);
    let read: Vec<&str> = fields.iter().map(|field| field.id.as_str()).collect();
    assert_eq!(
        read,
        ["before", "hash"],
        "what `message::floor_fields` reads"
    );

    let (role, fields) = declared("record_role");
    assert_eq!(message::ROLE_EVENT, role);
    let written: Vec<&str> = fields.iter().map(|field| field.id.as_str()).collect();
    assert_eq!(
        written,
        ["node", "role"],
        "what a host's role event carries"
    );
}
