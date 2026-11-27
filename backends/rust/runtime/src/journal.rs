// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The entries of the event journal a session's macrosteps make (docs/adr/0114, D1).
//!
//! A document of a keyed pool says a fact with `<send type="sce:journal" event="E">`,
//! the event of a schema it imports, its `<param>`s the fields of the schema's record.
//! The one send does two things. It makes an entry: the record's deterministic CBOR
//! (docs/adr/0101, section 2) with its schema's id and revision, and the valid time
//! the schema marks, which the engine keeps in the order made
//! ([`EngineState::record_journal_entry`](crate::EngineState::record_journal_entry)).
//! And it puts the event, with the entry's data, on the session's internal queue, as
//! `<raise>` does, so the transitions that apply the fact run in the same macrostep: a
//! document cannot record one thing and apply another.
//!
//! An entry is not written where it is said. The host takes the entries a run made
//! ([`EngineState::take_journal`](crate::EngineState::take_journal)) when it writes
//! the checkpoint that holds the state that made them, as the facts of that very write
//! (`sce_rust_session_store::Write::journal`), so an entry exists exactly when the
//! state that made it does. Until then it has no position. Nothing here is part of a
//! saved state: the state is the checkpoint, and a restore replays nothing.

/// One entry a macrostep made, as the write that keeps its checkpoint appends it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JournalEntry {
    /// The id of the schema its data is a record of: the name of the event the
    /// schema declares, which a follower is delivered the entry as (docs/adr/0114,
    /// D4).
    pub schema: &'static str,
    /// The schema's revision that wrote the data, by which an entry of an earlier
    /// revision is read (docs/adr/0044).
    pub revision: u32,
    /// The record's deterministic CBOR.
    pub data: Vec<u8>,
    /// When the fact holds from in the world, in milliseconds since the Unix epoch:
    /// the field the schema marks `sce:valid-time="from"`; `None` for a schema
    /// that marks none, whose fact holds from its recorded moment (D5).
    pub valid_ms: Option<i64>,
}

/// The entry of a fact said of the schema `schema` at `revision`, whose record is
/// `record_json`, the event's data as the send wrote it, under `description`, the
/// description the build wrote of the record's type
/// (`sce_build::forge::payload_record::describe`); its valid time read from the field
/// `valid_time` names, when the schema marks one.
///
/// # Errors
///
/// What the record codec refuses of the record (a list below its lower count, a
/// value past a bound, a field of the first revision missing), and a valid time
/// that is no integer of 64 bits: the fact has no entry, and the send that said it
/// is the argument error, so nothing is said and nothing is taken.
#[cfg(feature = "record-cbor")]
pub fn entry(
    schema: &'static str,
    revision: u32,
    record_json: &str,
    description: &str,
    valid_time: Option<&str>,
) -> Result<JournalEntry, crate::record_cbor::RecordCborRefusal> {
    let data = crate::record_cbor::encode(record_json, description)?;
    let valid_ms = valid_time
        .map(|field| valid_ms_of(record_json, field))
        .transpose()?;
    Ok(JournalEntry {
        schema,
        revision,
        data,
        valid_ms,
    })
}

/// The milliseconds the field `field` of the record `record_json` holds: a number,
/// or the text of its digits, which a 64-bit integer is written as.
#[cfg(feature = "record-cbor")]
fn valid_ms_of(
    record_json: &str,
    field: &str,
) -> Result<i64, crate::record_cbor::RecordCborRefusal> {
    use crate::json::{self, Value};
    let refused = |reason: &str| crate::record_cbor::RecordCborRefusal {
        pointer: format!("/{field}"),
        reason: reason.to_string(),
    };
    let record = json::parse(record_json.trim_matches(json::is_json_whitespace))
        .map_err(|_| refused("is in a record that is not JSON"))?;
    match record.member(field) {
        Some(Value::Number(text) | Value::Text(text)) => text
            .parse::<i64>()
            .map_err(|_| refused("is no whole number of milliseconds of 64 bits")),
        Some(_) => Err(refused("is no whole number of milliseconds of 64 bits")),
        None => Err(refused(
            "is missing, and it is the valid time the schema marks",
        )),
    }
}

/// One entry of a page of the journal as `sce-host-journal-read` answers it
/// (docs/adr/0114, D5; docs/SCE_HOST_FEATURES.md, "Event journal"): its position,
/// its recorded moment, when its fact holds from, the id another system gave it
/// (empty for a session's), its schema and revision, and the fact's
/// deterministic CBOR.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageEntry<'a> {
    pub position: u64,
    pub recorded_ms: u64,
    pub holds_ms: i64,
    pub origin: &'a str,
    pub schema: &'a str,
    pub revision: u32,
    pub data: &'a [u8],
}

/// The description, for the record codec, of a page of at most `entries`
/// entries in at most `bytes` bytes: a list of records, each field keyed by its
/// name, in the order their keys encode (`crate::record_cbor`). The page's
/// deterministic CBOR is what the result's `entries` holds; the host writes it
/// ([`write_page`]) and the machine opens it ([`open_page`]) by this one
/// description.
#[cfg(feature = "record-cbor")]
pub fn page_description(entries: u32, bytes: u64) -> String {
    format!(
        r#"{{"list":{{"min":0,"max":{entries},"of":{{"record":[["data",false,{{"bytes":{bytes}}},null],["origin",false,{{"text":{bytes}}},null],["schema",false,{{"text":{bytes}}},null],["holds_ms",false,{{"int":64}},null],["position",false,{{"uint":64}},null],["revision",false,{{"uint":32}},null],["recorded_ms",false,{{"uint":64}},null]]}}}}}}"#
    )
}

/// The page `entries` as the deterministic CBOR the result's `entries` holds,
/// at most `max_entries` entries in at most `max_bytes` bytes.
///
/// # Errors
///
/// A page of more entries, or one whose bytes pass `max_bytes`: the host
/// answers the read with the failure `response_too_large`, and the reader asks
/// again for fewer.
#[cfg(feature = "record-cbor")]
pub fn write_page(
    entries: &[PageEntry<'_>],
    max_entries: u32,
    max_bytes: u64,
) -> Result<Vec<u8>, crate::record_cbor::RecordCborRefusal> {
    use crate::json::Value;
    let items: Vec<Value> = entries
        .iter()
        .map(|entry| {
            Value::Object(vec![
                (
                    "data".to_string(),
                    Value::Text(crate::event_payload::bytes_as_payload_text(entry.data)),
                ),
                ("origin".to_string(), Value::Text(entry.origin.to_string())),
                ("schema".to_string(), Value::Text(entry.schema.to_string())),
                (
                    "holds_ms".to_string(),
                    Value::Text(entry.holds_ms.to_string()),
                ),
                (
                    "position".to_string(),
                    Value::Text(entry.position.to_string()),
                ),
                (
                    "revision".to_string(),
                    Value::Number(entry.revision.to_string()),
                ),
                (
                    "recorded_ms".to_string(),
                    Value::Text(entry.recorded_ms.to_string()),
                ),
            ])
        })
        .collect();
    let bytes = crate::record_cbor::encode(
        &crate::json::write(&Value::Array(items)),
        &page_description(max_entries, max_bytes),
    )?;
    if bytes.len() as u64 > max_bytes {
        return Err(crate::record_cbor::RecordCborRefusal {
            pointer: String::new(),
            reason: format!(
                "is {} bytes, past the {max_bytes} a page holds",
                bytes.len()
            ),
        });
    }
    Ok(bytes)
}

/// The completion `data` of a read of the journal with its page opened: what the
/// machine's lift of the completion reads (docs/adr/0114, D5).
///
/// `field` is the result's page (`entries`), `max_entries` and `max_bytes` its
/// bounds as the machine's build resolved them, `cases` the case of the read's
/// union each schema's fact is, as JSON `[[event, tag, variant, description],
/// ...]`, and `failure` the completion the read's failure `not_a_record` is,
/// which the build writes.
///
/// - A completion whose `error` is not empty carries no page: `field` is an
///   empty list, the rest as it came.
/// - One that read a page: `field` is its entries, each its `position`,
///   `recorded_ms`, `holds_ms`, `origin` and `fact`, the fact's record with the
///   union's tag naming its case.
/// - One whose page is no value of the page, or holds a fact of a schema no
///   case holds, or whose bytes are no value of its case's record: `failure`,
///   whole.
///
/// A completion that is no JSON object, or that carries no text under `field`,
/// is handed on as it came, for the lift to refuse as it refuses any data its
/// record does not fit.
#[cfg(feature = "record-cbor")]
pub fn open_page(
    data: &str,
    field: &str,
    max_entries: u32,
    max_bytes: u64,
    cases: &str,
    failure: &str,
) -> String {
    use crate::json::{self, Value};
    let Ok(Value::Object(mut members)) = json::parse(data.trim_matches(json::is_json_whitespace))
    else {
        return data.to_string();
    };
    let failed = members
        .iter()
        .find(|(key, _)| key == "error")
        .is_some_and(|(_, error)| !matches!(error, Value::Text(code) if code.is_empty()));
    let Some(slot) = members.iter().position(|(key, _)| key == field) else {
        return data.to_string();
    };
    if failed {
        members[slot].1 = Value::Array(Vec::new());
        return json::write(&Value::Object(members));
    }
    let Value::Text(text) = &members[slot].1 else {
        return data.to_string();
    };
    match read_page(text, max_entries, max_bytes, cases) {
        Some(entries) => {
            members[slot].1 = Value::Array(entries);
            json::write(&Value::Object(members))
        }
        None => failure.to_string(),
    }
}

/// The entries the page `text` (its bytes' Latin-1 text) holds, each opened by
/// its case; `None` for a page that is no value of the page or of its cases.
#[cfg(feature = "record-cbor")]
fn read_page(
    text: &str,
    max_entries: u32,
    max_bytes: u64,
    cases: &str,
) -> Option<Vec<crate::json::Value>> {
    use crate::json::{self, Value};
    let bytes = crate::record_cbor::latin1_bytes(text)?;
    if bytes.len() as u64 > max_bytes {
        return None;
    }
    let page =
        crate::record_cbor::decode(&bytes, &page_description(max_entries, max_bytes)).ok()?;
    let Value::Array(entries) = json::parse(&page).ok()? else {
        return None;
    };
    let Value::Array(cases) = json::parse(cases).ok()? else {
        return None;
    };
    entries
        .into_iter()
        .map(|entry| open_entry(entry, &cases))
        .collect()
}

/// The entry `entry` of a page with its fact read by the case of `cases` its
/// schema names, as the derived record's entry; `None` for one no case holds or
/// whose bytes are no value of its case's record.
#[cfg(feature = "record-cbor")]
fn open_entry(
    entry: crate::json::Value,
    cases: &[crate::json::Value],
) -> Option<crate::json::Value> {
    use crate::json::{self, Value};
    let Value::Object(members) = entry else {
        return None;
    };
    let get = |name: &str| {
        members
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.clone())
    };
    let Some(Value::Text(schema)) = get("schema") else {
        return None;
    };
    let Some(Value::Text(data)) = get("data") else {
        return None;
    };
    let (tag, variant, description) = cases.iter().find_map(|case| match case {
        Value::Array(parts) => match parts.as_slice() {
            [Value::Text(event), Value::Text(tag), Value::Text(variant), description]
                if *event == schema =>
            {
                Some((tag.clone(), variant.clone(), json::write(description)))
            }
            _ => None,
        },
        _ => None,
    })?;
    let record =
        crate::record_cbor::decode(&crate::record_cbor::latin1_bytes(&data)?, &description).ok()?;
    let Value::Object(mut fact) = json::parse(&record).ok()? else {
        return None;
    };
    fact.push((tag, Value::Text(variant)));
    Some(Value::Object(vec![
        ("position".to_string(), get("position")?),
        ("recorded_ms".to_string(), get("recorded_ms")?),
        ("holds_ms".to_string(), get("holds_ms")?),
        ("origin".to_string(), get("origin")?),
        ("fact".to_string(), Value::Object(fact)),
    ]))
}

#[cfg(all(test, feature = "record-cbor"))]
mod tests {
    use super::*;

    /// A record of two fields, the second the valid time, each of the first revision.
    const DESCRIPTION: &str =
        r#"{"record":[["at",false,{"int":64},null],["room",false,{"text":16},null]]}"#;

    #[test]
    fn an_entry_is_the_records_cbor_and_its_valid_time() {
        let said = entry(
            "room.booked",
            2,
            r#"{"at":"1700000000000","room":"a"}"#,
            DESCRIPTION,
            Some("at"),
        )
        .expect("a record of its type");
        assert_eq!(said.schema, "room.booked");
        assert_eq!(said.revision, 2);
        assert_eq!(said.valid_ms, Some(1_700_000_000_000));
        assert_eq!(
            said.data,
            crate::record_cbor::encode(r#"{"at":"1700000000000","room":"a"}"#, DESCRIPTION)
                .expect("the same record")
        );
        let unmarked = entry(
            "room.booked",
            1,
            r#"{"at":1,"room":"a"}"#,
            DESCRIPTION,
            None,
        )
        .expect("a record of its type");
        assert_eq!(unmarked.valid_ms, None);
    }

    #[test]
    fn a_record_that_has_no_encoding_makes_no_entry() {
        let refused = entry("room.booked", 1, r#"{"at":1}"#, DESCRIPTION, None)
            .expect_err("a field of the first revision is missing");
        assert_eq!(refused.pointer, "/room");
        let past = entry(
            "room.booked",
            1,
            r#"{"at":1,"room":"a room past sixteen bytes"}"#,
            DESCRIPTION,
            None,
        )
        .expect_err("a text past its bound");
        assert_eq!(past.pointer, "/room");
    }

    /// The cases of a union of one schema, `room.booked` under the tag `kind`.
    fn cases() -> String {
        format!(r#"[["room.booked","kind","booked",{DESCRIPTION}]]"#)
    }

    fn page(schema: &str, data: &[u8]) -> Vec<u8> {
        write_page(
            &[PageEntry {
                position: 7,
                recorded_ms: 1_000,
                holds_ms: -5,
                origin: "",
                schema,
                revision: 1,
                data,
            }],
            4,
            4096,
        )
        .expect("a page within its bounds")
    }

    fn completion(page: &[u8]) -> String {
        format!(
            r#"{{"error":"","horizon":"9","as_of_position":"9","entries":{}}}"#,
            crate::json::quote(&crate::event_payload::bytes_as_payload_text(page))
        )
    }

    const FAILURE: &str =
        r#"{"error":"not_a_record","horizon":"0","as_of_position":"0","entries":[]}"#;

    /// docs/adr/0114, D5: a page the host writes is opened into entries whose fact
    /// is a value of the read's union, its tag naming the case its schema is. Red
    /// if the page's description differs between the writer and the opener, or
    /// the tag is not written.
    #[test]
    fn a_page_is_opened_into_entries_of_the_union() {
        let fact = crate::record_cbor::encode(r#"{"at":"1","room":"a"}"#, DESCRIPTION)
            .expect("a record of its type");
        let opened = open_page(
            &completion(&page("room.booked", &fact)),
            "entries",
            4,
            4096,
            &cases(),
            FAILURE,
        );
        let value = crate::json::parse(&opened).expect("JSON");
        let entries = value.member("entries").expect("the page");
        let crate::json::Value::Array(entries) = entries else {
            panic!("a list: {opened}");
        };
        assert_eq!(entries.len(), 1, "{opened}");
        let entry = &entries[0];
        assert_eq!(
            entry.member("position"),
            Some(&crate::json::Value::Text("7".to_string()))
        );
        assert_eq!(
            entry.member("holds_ms"),
            Some(&crate::json::Value::Text("-5".to_string()))
        );
        let fact = entry.member("fact").expect("the fact");
        assert_eq!(
            fact.member("kind"),
            Some(&crate::json::Value::Text("booked".to_string()))
        );
        assert_eq!(
            fact.member("room"),
            Some(&crate::json::Value::Text("a".to_string()))
        );
    }

    /// A fact of a schema no case of the union holds, or bytes that are no value
    /// of its record, make the completion the failure `not_a_record`; a failed
    /// read carries an empty page.
    #[test]
    fn a_page_the_union_does_not_hold_is_the_failure() {
        let fact = crate::record_cbor::encode(r#"{"at":"1","room":"a"}"#, DESCRIPTION)
            .expect("a record of its type");
        for written in [page("room.cancelled", &fact), page("room.booked", &[0xff])] {
            assert_eq!(
                open_page(&completion(&written), "entries", 4, 4096, &cases(), FAILURE),
                FAILURE
            );
        }
        let failed = open_page(
            r#"{"error":"unavailable","horizon":"0","as_of_position":"0","entries":""}"#,
            "entries",
            4,
            4096,
            &cases(),
            FAILURE,
        );
        assert!(failed.contains(r#""entries":[]"#), "{failed}");
    }

    /// A page of more entries than its bound, or of more bytes, is not written.
    #[test]
    fn a_page_past_its_bounds_is_not_written() {
        let entry = PageEntry {
            position: 1,
            recorded_ms: 1,
            holds_ms: 1,
            origin: "",
            schema: "room.booked",
            revision: 1,
            data: &[0u8; 64],
        };
        assert!(write_page(&[entry, entry], 1, 4096).is_err());
        assert!(write_page(&[entry], 1, 32).is_err());
        assert!(write_page(&[entry], 1, 4096).is_ok());
    }
}
