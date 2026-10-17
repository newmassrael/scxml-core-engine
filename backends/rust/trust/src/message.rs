// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The bytes a write is signed as, and what makes it an operation of a device's chain.
//!
//! What an event says is written from the fields it carries and the name of the list, in one
//! fixed order and form, and not from the text of the event's data: a host that re-writes the data
//! (a different order of keys, a number written as text) changes no byte of what is signed, and a
//! signature does not move to another write, another kind of event, or another list. Each part is
//! written with its length, so that no two different writes are the same bytes.
//!
//! An event a device sends is an **operation** of that device's chain: the event as it travels
//! (its content sealed, when the list keeps it from the hub), the number it has among the
//! device's operations (`seq`), the id of the one before it (`prev`), and the entry of the
//! membership log it was made under (`mh`, and that entry's hash `mhh`). The device signs all of
//! it ([`operation_bytes`]), so that an operation neither moves to another chain position, nor
//! claims another entry of the log, nor leaves out what it follows.

use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

use crate::encoding;

/// The events a signature is carried by: a write to a field of a to-do, which its author
/// stamped, and the note that a write was seen and written over, which the person who wrote
/// over it made (`by`).
pub const SIGNED_EVENTS: [&str; 2] = ["todo.write", "todo.supersede"];

/// The key of the data under which the signature travels between hosts.
pub const SIGNATURE_FIELD: &str = "sig";

/// The keys of the data under which the operation's place in its chain travels.
pub const SEQ_FIELD: &str = "seq";
pub const PREV_FIELD: &str = "prev";
pub const LOG_FIELD: &str = "mh";
pub const LOG_HASH_FIELD: &str = "mhh";

const DOMAIN: &str = "sce-trust/event/1";
const OPERATION_DOMAIN: &str = "sce-trust/operation/1";

/// The id an operation has before any: what the first operation of a chain names as `prev`.
pub const NO_OPERATION: [u8; 32] = [0; 32];

/// Why an event's data could not be read as the event it names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageError {
    /// The data is not a JSON object.
    NotAnObject,
    /// A field the event always carries is missing or is not a number or text of the form the
    /// field has.
    Field(&'static str),
}

/// What an event is signed as, and who signed it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signing {
    /// The node of the person whose key signed it: `nodeId` of a write, `by` of a note.
    pub author: u64,
    /// What the note is about, written from the fields that name the write it wrote over and
    /// not from who wrote over it: two notes about one write have one subject.
    pub subject: Vec<u8>,
    pub message: Vec<u8>,
}

/// The bytes `event` with `data` is signed as in the list `calendar`, and its author. `None`
/// when `event` is not one a signature is carried by.
pub fn signing(calendar: &str, event: &str, data: &Value) -> Option<Result<Signing, MessageError>> {
    let (fields, author): (&[Field], &str) = match event {
        "todo.write" => (WRITE, "nodeId"),
        "todo.supersede" => (SUPERSEDE, "by"),
        _ => return None,
    };
    Some(build(calendar, event, data, fields, author))
}

/// The stamp of a write: its time and its counter, which are the author's clock when they wrote
/// it. Along one device's chain they do not go back.
pub fn stamp_of(data: &Value) -> Result<(i64, u64), MessageError> {
    let object = data.as_object().ok_or(MessageError::NotAnObject)?;
    let wall = read(object, "wallTime", Kind::Signed(None))?
        .parse::<i64>()
        .map_err(|_| MessageError::Field("wallTime"))?;
    let counter = read(object, "counter", Kind::Unsigned(None))?
        .parse::<u64>()
        .map_err(|_| MessageError::Field("counter"))?;
    Ok((wall, counter))
}

/// The number and the text a write carries, which is what is sealed when a host keeps a
/// write's content from the hub.
pub fn write_content(data: &Value) -> Result<(i64, String), MessageError> {
    let object = data.as_object().ok_or(MessageError::NotAnObject)?;
    let value = read(object, "value", Kind::Signed(Some(0)))?
        .parse::<i64>()
        .map_err(|_| MessageError::Field("value"))?;
    Ok((value, read(object, "text", Kind::Text)?))
}

/// What names a write, apart from its content and who it is for: the fields that make its
/// stamp, written as the parts of a message. A write is sealed under this (it is the nonce
/// and what the cipher binds the content to), so that sealed content moves to no other write.
pub fn write_stamp(data: &Value) -> Result<Vec<u8>, MessageError> {
    let object = data.as_object().ok_or(MessageError::NotAnObject)?;
    let mut stamp = Vec::new();
    for (name, kind) in WRITE {
        if ["value", "text"].contains(name) {
            continue;
        }
        part(&mut stamp, &read(object, name, *kind)?);
    }
    Ok(stamp)
}

/// Where an operation stands: its number among its device's operations, the id of the one before
/// it, and the entry of the membership log it was made under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Link {
    /// The number of the operation in its device's chain, from 1.
    pub seq: u64,
    /// The id of the operation before it ([`NO_OPERATION`] for the first).
    pub prev: [u8; 32],
    /// The position of the entry of the membership log it was made under, and that entry's hash.
    pub log: u64,
    pub log_hash: [u8; 32],
}

impl Link {
    /// Write the link into the data of an event, as the fields it travels in.
    pub fn write_into(&self, data: &mut Value) -> Option<()> {
        let object = data.as_object_mut()?;
        object.insert(SEQ_FIELD.to_string(), Value::String(self.seq.to_string()));
        object.insert(
            PREV_FIELD.to_string(),
            Value::String(encoding::encode(&self.prev)),
        );
        object.insert(LOG_FIELD.to_string(), Value::String(self.log.to_string()));
        object.insert(
            LOG_HASH_FIELD.to_string(),
            Value::String(encoding::encode(&self.log_hash)),
        );
        Some(())
    }

    /// Take the link out of the data of an event (leaving the event as its author made it), or
    /// say that a field of it is missing or not what it is written as.
    pub fn take_from(data: &mut Value) -> Result<Self, String> {
        let object = data
            .as_object_mut()
            .ok_or_else(|| "the data is not an object".to_string())?;
        let mut text = |name: &str| -> Result<String, String> {
            match object.remove(name) {
                Some(Value::String(text)) => Ok(text),
                _ => Err(format!("it carries no `{name}`")),
            }
        };
        let seq = text(SEQ_FIELD)?;
        let prev = text(PREV_FIELD)?;
        let log = text(LOG_FIELD)?;
        let log_hash = text(LOG_HASH_FIELD)?;
        let hash = |name: &str, value: &str| -> Result<[u8; 32], String> {
            encoding::decode(value)
                .and_then(|bytes| bytes.try_into().ok())
                .ok_or_else(|| format!("its `{name}` is not a hash"))
        };
        Ok(Self {
            seq: seq
                .parse()
                .map_err(|_| "its `seq` is not a number".to_string())?,
            prev: hash(PREV_FIELD, &prev)?,
            log: log
                .parse()
                .map_err(|_| "its `mh` is not a number".to_string())?,
            log_hash: hash(LOG_HASH_FIELD, &log_hash)?,
        })
    }
}

/// The bytes a device signs to make an event an operation of its chain: the event as it travels
/// (`travelling`, which is [`Signing::message`] of the data that goes on the wire) and the
/// link.
pub fn operation_bytes(travelling: &[u8], link: &Link) -> Vec<u8> {
    let mut bytes = Vec::new();
    part(&mut bytes, OPERATION_DOMAIN);
    bytes.extend_from_slice(&(travelling.len() as u64).to_be_bytes());
    bytes.extend_from_slice(travelling);
    part(&mut bytes, &link.seq.to_string());
    part(&mut bytes, &encoding::encode(&link.prev));
    part(&mut bytes, &link.log.to_string());
    part(&mut bytes, &encoding::encode(&link.log_hash));
    bytes
}

/// What names an operation, and what the next one of its chain names as `prev`.
pub fn operation_id(operation: &[u8]) -> [u8; 32] {
    Sha256::digest(operation).into()
}

/// The event by which the hub says an instant is settled, which an administrator attests.
pub const FLOOR_EVENT: &str = "todo.floor";

const FLOOR_DOMAIN: &str = "sce-trust/floor/1";

/// The bytes an administrator signs to attest that the changes up to `before` are settled in
/// `calendar` with the hash `hash` (todo_floor): not the event's own text, so that what is
/// attested is the instant and what every copy held of it, and nothing the hub could write
/// around it.
pub fn floor_message(calendar: &str, before: i64, hash: u32) -> Vec<u8> {
    let mut message = Vec::new();
    part(&mut message, FLOOR_DOMAIN);
    part(&mut message, calendar);
    part(&mut message, &before.to_string());
    part(&mut message, &hash.to_string());
    message
}

/// The instant and the hash a `todo.floor` carries.
pub fn floor_fields(data: &Value) -> Result<(i64, u32), MessageError> {
    let object = data.as_object().ok_or(MessageError::NotAnObject)?;
    let before = read(object, "before", Kind::Signed(None))?
        .parse::<i64>()
        .map_err(|_| MessageError::Field("before"))?;
    let hash = read(object, "hash", Kind::Unsigned(None))?
        .parse::<u32>()
        .map_err(|_| MessageError::Field("hash"))?;
    Ok((before, hash))
}

/// How a field is read: a whole number of the given sign, or text, with the value an event
/// that leaves it out is read as (a write of a number leaves its `text` out, and a write of a
/// text its `value`).
#[derive(Clone, Copy)]
enum Kind {
    Unsigned(Option<u64>),
    Signed(Option<i64>),
    Text,
}

type Field = (&'static str, Kind);

const WRITE: &[Field] = &[
    ("id", Kind::Unsigned(None)),
    ("field", Kind::Unsigned(None)),
    ("value", Kind::Signed(Some(0))),
    ("text", Kind::Text),
    ("wallTime", Kind::Signed(None)),
    ("counter", Kind::Unsigned(None)),
    ("nodeId", Kind::Unsigned(None)),
];

const SUPERSEDE: &[Field] = &[
    ("id", Kind::Unsigned(None)),
    ("field", Kind::Unsigned(None)),
    ("wallTime", Kind::Signed(None)),
    ("counter", Kind::Unsigned(None)),
    ("nodeId", Kind::Unsigned(None)),
    ("by", Kind::Unsigned(None)),
];

fn build(
    calendar: &str,
    event: &str,
    data: &Value,
    fields: &[Field],
    author: &'static str,
) -> Result<Signing, MessageError> {
    let object = data.as_object().ok_or(MessageError::NotAnObject)?;
    let mut message = Vec::new();
    part(&mut message, DOMAIN);
    part(&mut message, calendar);
    part(&mut message, event);
    let mut subject = Vec::new();
    let mut author_node = None;
    for (name, kind) in fields {
        let text = read(object, name, *kind)?;
        if *name == author {
            author_node = text.parse::<u64>().ok();
        }
        if *name != "by" {
            part(&mut subject, &text);
        }
        part(&mut message, &text);
    }
    Ok(Signing {
        author: author_node.ok_or(MessageError::Field(author))?,
        subject,
        message,
    })
}

fn part(message: &mut Vec<u8>, text: &str) {
    message.extend_from_slice(&(text.len() as u64).to_be_bytes());
    message.extend_from_slice(text.as_bytes());
}

/// The field as the text of its value: a number is written as its digits, whether the event
/// carries it as a number or, as every 64-bit number of a payload is carried, as text.
fn read(
    object: &Map<String, Value>,
    name: &'static str,
    kind: Kind,
) -> Result<String, MessageError> {
    let value = object.get(name);
    match (kind, value) {
        (Kind::Text, None) => Ok(String::new()),
        (Kind::Text, Some(Value::String(text))) => Ok(text.clone()),
        (Kind::Unsigned(default), None) => default
            .map(|n| n.to_string())
            .ok_or(MessageError::Field(name)),
        (Kind::Signed(default), None) => default
            .map(|n| n.to_string())
            .ok_or(MessageError::Field(name)),
        (Kind::Unsigned(_), Some(Value::String(text))) => text
            .parse::<u64>()
            .map(|n| n.to_string())
            .map_err(|_| MessageError::Field(name)),
        (Kind::Unsigned(_), Some(Value::Number(n))) => n
            .as_u64()
            .map(|n| n.to_string())
            .ok_or(MessageError::Field(name)),
        (Kind::Signed(_), Some(Value::String(text))) => text
            .parse::<i64>()
            .map(|n| n.to_string())
            .map_err(|_| MessageError::Field(name)),
        (Kind::Signed(_), Some(Value::Number(n))) => n
            .as_i64()
            .map(|n| n.to_string())
            .ok_or(MessageError::Field(name)),
        _ => Err(MessageError::Field(name)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn write() -> Value {
        json!({"id":"1","field":2,"value":"10","text":"","wallTime":"1000","counter":0,"nodeId":"3"})
    }

    #[test]
    fn a_write_is_signed_by_the_node_that_stamped_it() {
        let signing = signing("home", "todo.write", &write()).unwrap().unwrap();
        assert_eq!(signing.author, 3);
        assert_eq!(stamp_of(&write()), Ok((1000, 0)));
        let note = json!({"id":1,"field":2,"wallTime":5,"counter":0,"nodeId":3,"by":"4"});
        let signing = super::signing("home", "todo.supersede", &note)
            .unwrap()
            .unwrap();
        assert_eq!(signing.author, 4, "a note is signed by who wrote over");
        let other = json!({"id":1,"field":2,"wallTime":5,"counter":0,"nodeId":3,"by":"7"});
        let other = super::signing("home", "todo.supersede", &other)
            .unwrap()
            .unwrap();
        assert_eq!(
            other.subject, signing.subject,
            "two notes about one write have one subject"
        );
        assert_ne!(other.message, signing.message);
        assert!(super::signing("home", "todo.edit", &write()).is_none());
    }

    #[test]
    fn the_form_the_data_is_written_in_does_not_change_what_is_signed() {
        let as_text =
            json!({"id":"1","field":"2","value":"10","wallTime":"1000","counter":"0","nodeId":"3"});
        let as_numbers =
            json!({"nodeId":3,"counter":0,"wallTime":1000,"value":10,"field":2,"id":1,"text":""});
        let a = signing("home", "todo.write", &as_text).unwrap().unwrap();
        let b = signing("home", "todo.write", &as_numbers).unwrap().unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn a_signature_does_not_move_to_another_write_or_another_list() {
        let base = signing("home", "todo.write", &write()).unwrap().unwrap();
        let mut other = write();
        other["value"] = json!("11");
        assert_ne!(
            base,
            signing("home", "todo.write", &other).unwrap().unwrap()
        );
        assert_ne!(
            base,
            signing("work", "todo.write", &write()).unwrap().unwrap()
        );
        let mut text = write();
        text["text"] = json!("a");
        text["value"] = json!("0");
        let mut shifted = write();
        shifted["text"] = json!("");
        shifted["value"] = json!("0");
        assert_ne!(
            signing("home", "todo.write", &text).unwrap().unwrap(),
            signing("home", "todo.write", &shifted).unwrap().unwrap(),
        );
    }

    #[test]
    fn data_that_is_not_the_event_is_refused() {
        assert_eq!(
            signing("home", "todo.write", &json!([1])).unwrap(),
            Err(MessageError::NotAnObject)
        );
        let mut missing = write();
        missing.as_object_mut().unwrap().remove("nodeId");
        assert_eq!(
            signing("home", "todo.write", &missing).unwrap(),
            Err(MessageError::Field("nodeId"))
        );
        let mut negative = write();
        negative["counter"] = json!(-1);
        assert_eq!(
            signing("home", "todo.write", &negative).unwrap(),
            Err(MessageError::Field("counter"))
        );
    }

    #[test]
    fn a_link_travels_in_the_data_and_is_taken_out_whole() {
        let link = Link {
            seq: 7,
            prev: [3; 32],
            log: 4,
            log_hash: [5; 32],
        };
        let mut data = write();
        link.write_into(&mut data).unwrap();
        assert_eq!(Link::take_from(&mut data), Ok(link));
        assert_eq!(
            data,
            write(),
            "what is left is the event as its author made it"
        );
        assert!(Link::take_from(&mut data).is_err(), "it was taken out");
        let mut bad = write();
        link.write_into(&mut bad).unwrap();
        bad["seq"] = json!("seven");
        assert!(Link::take_from(&mut bad).is_err());
    }

    #[test]
    fn an_operation_is_signed_as_it_stands_in_its_chain() {
        let travelling = signing("home", "todo.write", &write())
            .unwrap()
            .unwrap()
            .message;
        let link = Link {
            seq: 1,
            prev: NO_OPERATION,
            log: 3,
            log_hash: [5; 32],
        };
        let base = operation_bytes(&travelling, &link);
        for moved in [
            Link { seq: 2, ..link },
            Link {
                prev: [1; 32],
                ..link
            },
            Link { log: 4, ..link },
            Link {
                log_hash: [6; 32],
                ..link
            },
        ] {
            assert_ne!(operation_bytes(&travelling, &moved), base, "{moved:?}");
        }
        assert_ne!(
            operation_id(&base),
            operation_id(&operation_bytes(b"x", &link))
        );
    }
}
