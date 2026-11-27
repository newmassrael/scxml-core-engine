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
}
