// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! Every message type a mapped port of the corpus carries has its codec
//! document (docs/SCE_TTCN3_LANE.md §12, §12.4; DD §8.4), in CBOR
//! (SCE_FORGE.md §4.6.1) or content-line (§4.6.4): in the lane the TTCN-3
//! side encodes each input of a trace and decodes each message the test
//! system sends to the system under test by a binding that reads the format's
//! parameters from that document. The document is
//! `test_profile/codecs/<record>_codec.scxml`; it is a codec the parser
//! accepts, and its entries are the record's fields, each of the type the field
//! has, so the binding reads a field by the entry of its id.
//!
//! A record a field of which no format holds, a list of integers, a record
//! within the record or a union, has no codec document; it is named below with
//! that field, and the test holds the field to have no form, so a record that
//! could have a codec cannot be left without one.

mod common;

use std::collections::BTreeSet;
use std::path::Path;

use common::lane_corpus::{codecs_dir, fields, mapped_types, read, CORPUS};
use sce_build::forge::model::{CborEntryType, CodecEncoding, EntryType, ForgeDocument, SceType};
use sce_build::forge::parser::parse_forge;
use sce_build::DocumentLabel;

/// The records a mapped port of the corpus carries that no codec format holds,
/// each with the first field that has no form in either: CBOR holds no list but
/// a list of records whose fields are scalars, texts and byte strings, and no
/// record within the record or union; content-line holds neither, nor a byte
/// string (docs/SCE_TTCN3_LANE.md §12.4).
const NO_CODEC_FORM: &[(&str, &str)] = &[("job_status", "codes"), ("order_placed", "lines")];

/// The entry a field of type `ty` is in a codec document, outside its
/// `optional<..>`: its type and the bound its `sce:max-size` states; `None` for
/// a type no entry of either format holds.
fn entry_form(ty: &str) -> Option<(SceType, Option<u32>)> {
    let bounded =
        |prefix: &str| -> Option<u32> { ty.strip_prefix(prefix)?.strip_suffix(']')?.parse().ok() };
    if let Some(bound) = bounded("string[") {
        return Some((SceType::String, Some(bound)));
    }
    if let Some(bound) = bounded("bytes[") {
        return Some((SceType::Bytes, Some(bound)));
    }
    let scalar = match ty {
        "uint8" => SceType::Uint8,
        "uint16" => SceType::Uint16,
        "uint32" => SceType::Uint32,
        "uint64" => SceType::Uint64,
        "int8" => SceType::Int8,
        "int16" => SceType::Int16,
        "int32" => SceType::Int32,
        "int64" => SceType::Int64,
        "bool" => SceType::Bool,
        _ => return None,
    };
    Some((scalar, None))
}

/// The entry type and bound of a field written `ty`: `optional<T>` is an entry
/// that may be absent, as an optional field is (docs/adr/0055, section 8).
fn expected_entry(ty: &str) -> Option<(EntryType, Option<u32>)> {
    match ty
        .strip_prefix("optional<")
        .and_then(|inner| inner.strip_suffix('>'))
    {
        Some(inner) => entry_form(inner).map(|(elem, bound)| (EntryType::Optional { elem }, bound)),
        None => entry_form(ty).map(|(scalar, bound)| (EntryType::Scalar(scalar), bound)),
    }
}

/// Whether a format holds a value of `ty`: CBOR holds an unsigned integer,
/// a truth value, a text and a byte string, and content-line a signed integer
/// beside them but no byte string.
fn held_by(encoding: CodecEncoding, ty: &SceType) -> bool {
    match encoding {
        CodecEncoding::Cbor => !matches!(
            ty,
            SceType::Int8 | SceType::Int16 | SceType::Int32 | SceType::Int64
        ),
        CodecEncoding::ContentLine => !matches!(ty, SceType::Bytes),
        CodecEncoding::Positional => false,
    }
}

/// One entry of a codec document as the binding reads it: its id, its type and
/// the bound its `sce:max-size` states.
type Entry = (String, EntryType, Option<u32>);

/// The entries of the codec document at `path`, or why it is no codec document
/// of the corpus: one the parser refuses, a positional codec, or an entry of a
/// shape a field is not (a list, an exact length, a parameter, a repeated
/// property).
fn codec_entries(path: &Path) -> Result<(CodecEncoding, Vec<Entry>), String> {
    let text = read(path);
    let stem = path
        .file_stem()
        .expect("a codec document is a file")
        .to_string_lossy()
        .into_owned();
    let label = DocumentLabel::symmetric(&stem);
    let model = match parse_forge(&text, label) {
        Ok(Some(ForgeDocument::Codec(model))) => model,
        other => return Err(format!("is no codec the parser accepts: {other:?}")),
    };
    match model.encoding {
        CodecEncoding::Cbor => model
            .cbor_entries
            .iter()
            .map(|entry| match (&entry.sce_type, entry.length) {
                (CborEntryType::Leaf(ty), None) => {
                    Ok((entry.id.clone(), ty.clone(), entry.max_size))
                }
                _ => Err(format!(
                    "its entry {} is of no shape a field is: {entry:?}",
                    entry.id
                )),
            })
            .collect::<Result<_, _>>()
            .map(|entries| (CodecEncoding::Cbor, entries)),
        CodecEncoding::ContentLine => model
            .content_line
            .as_ref()
            .expect("a content-line codec has its component")
            .entries
            .iter()
            .map(|entry| {
                if entry.param.is_some() || entry.repeated || entry.separator.is_some() {
                    Err(format!(
                        "its entry {} is of no shape a field is: {entry:?}",
                        entry.id
                    ))
                } else {
                    Ok((entry.id.clone(), entry.sce_type.clone(), entry.max_size))
                }
            })
            .collect::<Result<_, _>>()
            .map(|entries| (CodecEncoding::ContentLine, entries)),
        CodecEncoding::Positional => Err("is a positional codec, which no binding reads".into()),
    }
}

/// Why the codec document at `codec` does not encode the record of the event
/// schema at `schema`, each reason on a line; empty when it does.
fn mismatches(codec: &Path, schema: &Path) -> Vec<String> {
    let (encoding, entries) = match codec_entries(codec) {
        Ok(read) => read,
        Err(why) => return vec![why],
    };
    let mut wrong = Vec::new();
    let record = fields(schema);
    for (id, ty) in &record {
        let Some((want, bound)) = expected_entry(ty) else {
            wrong.push(format!("the field {id}, of type {ty}, has no codec form"));
            continue;
        };
        let scalar = match &want {
            EntryType::Scalar(scalar) | EntryType::Optional { elem: scalar } => scalar,
        };
        if !held_by(encoding, scalar) {
            wrong.push(format!(
                "the field {id}, of type {ty}, has no form in {encoding:?}"
            ));
        }
        match entries.iter().find(|(entry, _, _)| entry == id) {
            None => wrong.push(format!("no entry encodes the field {id}")),
            Some((_, got, got_bound)) if (got, *got_bound) != (&want, bound) => {
                wrong.push(format!(
                    "the entry {id} is {got:?} bounded by {got_bound:?}, and the field {ty} is \
                 {want:?} bounded by {bound:?}"
                ))
            }
            Some(_) => {}
        }
    }
    let ids: BTreeSet<&str> = record.iter().map(|(id, _)| id.as_str()).collect();
    for (entry, _, _) in &entries {
        if !ids.contains(entry.as_str()) {
            wrong.push(format!("the entry {entry} encodes no field of the record"));
        }
    }
    wrong
}

/// docs/SCE_TTCN3_LANE.md §12.4: each record a mapped port of a suite of the
/// corpus carries has a codec document whose entries are its fields, or is a
/// record of NO_CODEC_FORM whose named field no format holds.
#[test]
fn every_mapped_message_type_has_its_codec_document() {
    let mut wrong = Vec::new();
    for suite in CORPUS {
        for (record, schema) in mapped_types(&suite.path()) {
            if let Some((_, field)) = NO_CODEC_FORM.iter().find(|(name, _)| *name == record) {
                let ty = fields(&schema)
                    .into_iter()
                    .find(|(id, _)| id == field)
                    .map(|(_, ty)| ty)
                    .unwrap_or_else(|| panic!("the record {record} has a field {field}"));
                if expected_entry(&ty).is_some() {
                    wrong.push(format!(
                        "{}: {record} is named as a record with no codec form, but its field \
                         {field}, of type {ty}, has one",
                        suite.name()
                    ));
                }
                continue;
            }
            let codec = codecs_dir().join(format!("{record}_codec.scxml"));
            if !codec.exists() {
                wrong.push(format!(
                    "{}: the mapped type {record} has no codec document {}",
                    suite.name(),
                    codec.display()
                ));
                continue;
            }
            for why in mismatches(&codec, &schema) {
                wrong.push(format!("{}: {record}_codec {why}", suite.name()));
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// Every codec document of the corpus encodes a record a mapped port of one of
/// its suites carries, and every record NO_CODEC_FORM names is one, so neither
/// list holds a name nothing reads.
#[test]
fn every_codec_document_is_of_a_mapped_type() {
    let mapped: BTreeSet<String> = CORPUS
        .iter()
        .flat_map(|suite| mapped_types(&suite.path()).into_keys())
        .collect();
    let mut wrong = Vec::new();
    for entry in std::fs::read_dir(codecs_dir()).expect("the codecs directory is read") {
        let name = entry
            .expect("an entry of the codecs directory")
            .file_name()
            .to_string_lossy()
            .into_owned();
        match name.strip_suffix("_codec.scxml") {
            Some(record) if mapped.contains(record) => {}
            _ => wrong.push(format!("{name} is the codec document of no mapped type")),
        }
    }
    for (record, _) in NO_CODEC_FORM {
        if !mapped.contains(*record) {
            wrong.push(format!(
                "{record} is carried by no mapped port of the corpus"
            ));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// The comparison a codec document is held to is the one its reasons name: a
/// record of a text and an integer whose entries are of another bound, of
/// another type and of a field the record does not have is refused for each,
/// and the CBOR codec of a signed field for its format.
#[test]
fn a_codec_document_that_does_not_encode_its_record_is_named_why() {
    let dir = std::env::temp_dir().join(format!("sce-lane-codec-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("the directory is made");
    let schema = dir.join("probe_record.scxml");
    std::fs::write(
        &schema,
        r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="probe_record" sce:event-name="probe.record">
  <datamodel>
    <data id="label" sce:type="string[8]" sce:direction="in"/>
    <data id="count" sce:type="uint16" sce:direction="in"/>
    <data id="delta" sce:type="int8" sce:direction="in"/>
  </datamodel>
</scxml>
"#,
    )
    .expect("the schema is written");
    let codec = dir.join("probe_record_codec.scxml");
    std::fs::write(
        &codec,
        r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:encoding="cbor" sce:codec-format="2" name="probe_record_codec" version="1.0">
  <datamodel>
    <data id="raw" sce:type="bytes" sce:direction="in"/>
    <data id="label" sce:type="string" sce:key="0" sce:max-size="16" sce:direction="out"/>
    <data id="count" sce:type="uint32" sce:key="1" sce:direction="out"/>
    <data id="delta" sce:type="uint8" sce:key="2" sce:direction="out"/>
    <data id="spare" sce:type="bool" sce:key="3" sce:direction="out"/>
  </datamodel>
</scxml>
"#,
    )
    .expect("the codec is written");
    let wrong = mismatches(&codec, &schema);
    std::fs::remove_dir_all(&dir).expect("the directory is removed");
    let starts = |prefix: &str| wrong.iter().any(|why| why.starts_with(prefix));
    assert!(starts("the entry label is"), "{wrong:?}");
    assert!(starts("the entry count is"), "{wrong:?}");
    assert!(
        starts("the field delta, of type int8, has no form in Cbor"),
        "{wrong:?}"
    );
    assert!(starts("the entry delta is"), "{wrong:?}");
    assert!(starts("the entry spare encodes no field"), "{wrong:?}");
    assert_eq!(wrong.len(), 5, "{wrong:?}");
}
