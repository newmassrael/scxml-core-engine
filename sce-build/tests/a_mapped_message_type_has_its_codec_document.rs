// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
//! A CBOR entry is a value of the language (docs/adr/0101, section 1), so the
//! type of an entry and the type of its field are one type, compared whole. A
//! record whose codec document lands with the composed codecs of ADR 0101
//! (its Landing order step 6) is named below with its first field that is a
//! composition, and the test holds it to have no codec document yet, so a
//! record cannot stay on the list once its document exists.

mod common;

use std::collections::BTreeSet;
use std::path::Path;

use common::lane_corpus::{codecs_dir, fields, mapped_types, read, CORPUS};
use sce_build::forge::model::{CodecEncoding, ContentLineEntry, EntryType, ForgeDocument, SceType};
use sce_build::forge::parser::parse_forge;
use sce_build::forge::value_type::{Bound, ValueType, WrittenType};
use sce_build::DocumentLabel;

/// The records a mapped port of the corpus carries whose codec documents land
/// with docs/adr/0101 Landing order step 6, each with its first field that is a
/// composition (a list, a record or a union): a CBOR entry holds it (section 1),
/// and no backend generates such an entry yet (section 5.4).
const NO_CODEC_FORM: &[(&str, &str)] = &[("job_status", "codes"), ("order_placed", "lines")];

/// The type a field written `ty` is, every bound inside it: the one type
/// grammar's reading (docs/adr/0086, section 1); `None` for a text the grammar
/// does not read whole.
fn field_type(ty: &str) -> Option<ValueType> {
    WrittenType::read(ty).ok()?.bounded().ok()
}

/// The value type of a content-line entry: its scalar, a text at its
/// `sce:max-size`, `optional<T>` for one that may be absent; `None` for a text
/// that states no bound.
fn content_line_type(entry: &ContentLineEntry) -> Option<ValueType> {
    let leaf = |scalar: &SceType| -> Option<ValueType> {
        Some(match scalar {
            SceType::String => ValueType::Text {
                max: Bound::of(entry.max_size?),
            },
            SceType::Bytes => ValueType::Bytes {
                max: Bound::of(entry.max_size?),
            },
            other => ValueType::Scalar(other.clone()),
        })
    };
    match &entry.sce_type {
        EntryType::Scalar(scalar) => leaf(scalar),
        EntryType::Optional { elem } => Some(ValueType::Optional {
            elem: Box::new(leaf(elem)?),
        }),
    }
}

/// Whether a format holds a value of `ty`: CBOR holds every type of the
/// language (docs/adr/0101, section 1), and content-line a leaf, a scalar or a
/// text, bare or optional, but no byte string (docs/adr/0101, section 1.4).
fn held_by(encoding: CodecEncoding, ty: &ValueType) -> bool {
    match encoding {
        CodecEncoding::Cbor => true,
        CodecEncoding::ContentLine => match ty.entry() {
            Ok((entry, _)) => !matches!(entry.present_type(), SceType::Bytes),
            Err(_) => false,
        },
        CodecEncoding::Positional => false,
    }
}

/// One entry of a codec document as the binding reads it: its id and its type.
type Entry = (String, ValueType);

/// The entries of the codec document at `path`, or why it is no codec document
/// of the corpus: one the parser refuses, a positional codec, or an entry of a
/// shape a field is not (an exact length, a parameter, a repeated property, a
/// text that states no bound).
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
            .map(|entry| match entry.length {
                None => Ok((entry.id.clone(), entry.sce_type.clone())),
                Some(_) => Err(format!(
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
                let shaped = entry.param.is_none() && !entry.repeated && entry.separator.is_none();
                match content_line_type(entry).filter(|_| shaped) {
                    Some(ty) => Ok((entry.id.clone(), ty)),
                    None => Err(format!(
                        "its entry {} is of no shape a field is: {entry:?}",
                        entry.id
                    )),
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
        let Some(want) = field_type(ty) else {
            wrong.push(format!(
                "the field {id}, of type {ty}, is no type of the grammar"
            ));
            continue;
        };
        if !held_by(encoding, &want) {
            wrong.push(format!(
                "the field {id}, of type {ty}, has no form in {encoding:?}"
            ));
        }
        match entries.iter().find(|(entry, _)| entry == id) {
            None => wrong.push(format!("no entry encodes the field {id}")),
            Some((_, got)) if *got != want => wrong.push(format!(
                "the entry {id} is `{got}`, and the field is `{want}`"
            )),
            Some(_) => {}
        }
    }
    let ids: BTreeSet<&str> = record.iter().map(|(id, _)| id.as_str()).collect();
    for (entry, _) in &entries {
        if !ids.contains(entry.as_str()) {
            wrong.push(format!("the entry {entry} encodes no field of the record"));
        }
    }
    wrong
}

/// docs/SCE_TTCN3_LANE.md §12.4: each record a mapped port of a suite of the
/// corpus carries has a codec document whose entries are its fields, or is a
/// record of NO_CODEC_FORM whose named field is a composition and which has no
/// codec document yet.
#[test]
fn every_mapped_message_type_has_its_codec_document() {
    let mut wrong = Vec::new();
    for suite in CORPUS {
        for (record, schema) in mapped_types(&suite.path()) {
            let codec = codecs_dir().join(format!("{record}_codec.scxml"));
            if let Some((_, field)) = NO_CODEC_FORM.iter().find(|(name, _)| *name == record) {
                let ty = fields(&schema)
                    .into_iter()
                    .find(|(id, _)| id == field)
                    .map(|(_, ty)| ty)
                    .unwrap_or_else(|| panic!("the record {record} has a field {field}"));
                if field_type(&ty).is_some_and(|ty| ty.entry().is_ok()) {
                    wrong.push(format!(
                        "{}: {record} is named as a record whose codec document waits for a \
                         composed entry, but its field {field}, of type {ty}, is no composition",
                        suite.name()
                    ));
                }
                if codec.exists() {
                    wrong.push(format!(
                        "{}: {record} is named as a record with no codec document, and it has \
                         {}",
                        suite.name(),
                        codec.display()
                    ));
                }
                continue;
            }
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
/// record whose entries are of another bound, of another width, of another
/// sign and of a field the record does not have is refused for each; a CBOR
/// entry holds a signed field (docs/adr/0101, section 1), and a content-line
/// codec holds no byte string.
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
    <data id="blob" sce:type="bytes[4]" sce:direction="in"/>
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
    <data id="label" sce:type="string[16]" sce:key="0" sce:direction="out"/>
    <data id="count" sce:type="uint32" sce:key="1" sce:direction="out"/>
    <data id="delta" sce:type="uint8" sce:key="2" sce:direction="out"/>
    <data id="blob" sce:type="bytes[4]" sce:key="3" sce:direction="out"/>
    <data id="spare" sce:type="bool" sce:key="4" sce:direction="out"/>
  </datamodel>
</scxml>
"#,
    )
    .expect("the codec is written");
    let lines = dir.join("probe_record_lines.scxml");
    std::fs::write(
        &lines,
        r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:encoding="content-line" sce:codec-format="2" sce:component="VEVENT" name="probe_record_lines" version="1.0">
  <datamodel>
    <data id="raw" sce:type="bytes" sce:direction="in"/>
    <data id="label" sce:type="string" sce:property="LABEL" sce:max-size="8"/>
    <data id="count" sce:type="uint16" sce:property="COUNT"/>
    <data id="delta" sce:type="int8" sce:property="DELTA"/>
  </datamodel>
</scxml>
"#,
    )
    .expect("the content-line codec is written");
    let wrong = mismatches(&codec, &schema);
    let wrong_lines = mismatches(&lines, &schema);
    std::fs::remove_dir_all(&dir).expect("the directory is removed");
    let starts = |wrong: &[String], prefix: &str| wrong.iter().any(|why| why.starts_with(prefix));
    assert!(starts(&wrong, "the entry label is"), "{wrong:?}");
    assert!(starts(&wrong, "the entry count is"), "{wrong:?}");
    assert!(starts(&wrong, "the entry delta is"), "{wrong:?}");
    assert!(
        starts(&wrong, "the entry spare encodes no field"),
        "{wrong:?}"
    );
    // A signed field and a byte string each have their CBOR form.
    assert!(
        !wrong.iter().any(|why| why.contains("has no form")),
        "{wrong:?}"
    );
    assert_eq!(wrong.len(), 4, "{wrong:?}");
    // The content-line codec encodes the record but for its byte string,
    // which no content line holds.
    assert_eq!(
        wrong_lines,
        [
            "the field blob, of type bytes[4], has no form in ContentLine".to_string(),
            "no entry encodes the field blob".to_string(),
        ],
        "{wrong_lines:?}"
    );
}
