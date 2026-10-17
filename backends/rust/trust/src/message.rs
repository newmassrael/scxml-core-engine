// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The bytes a write is signed as.
//!
//! They are written from the fields the event carries and the name of the list, in one fixed
//! order and form, and not from the text of the event's data: a host that re-writes the data
//! (a different order of keys, a number written as text) changes no byte of what is signed,
//! and a signature does not move to another write, another kind of event, or another list.
//! Each part is written with its length, so that no two different writes are the same bytes.

use serde_json::{Map, Value};

/// The events a signature is carried by: a write to a field of a to-do, which its author
/// stamped, and the note that a write was seen and written over, which the person who wrote
/// over it made (`by`).
pub const SIGNED_EVENTS: [&str; 2] = ["todo.write", "todo.supersede"];

/// The key of the data under which the signature travels between hosts.
pub const SIGNATURE_FIELD: &str = "sig";

const DOMAIN: &str = "sce-trust/event/1";

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
    let mut author_node = None;
    for (name, kind) in fields {
        let text = read(object, name, *kind)?;
        if *name == author {
            author_node = text.parse::<u64>().ok();
        }
        part(&mut message, &text);
    }
    Ok(Signing {
        author: author_node.ok_or(MessageError::Field(author))?,
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
        let note = json!({"id":1,"field":2,"wallTime":5,"counter":0,"nodeId":3,"by":"4"});
        let signing = super::signing("home", "todo.supersede", &note)
            .unwrap()
            .unwrap();
        assert_eq!(signing.author, 4, "a note is signed by who wrote over");
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
}
