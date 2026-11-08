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
//! membership log it was made under (`mh`, and that entry's hash `mhh`), and its causal context,
//! the highest operation of each other device its machine had been told (`cx`). The device signs
//! all of it ([`operation_bytes`]), so that an operation neither moves to another chain position,
//! nor claims another entry of the log, nor leaves out what it follows, in its chain or in
//! another's: a relay cannot change the order a host tells its machine operations in.
//!
//! The events are those of the standard shared record list ([`LIST_DOCUMENT`], docs/adr/0042),
//! and no others: a list an application builds on that part is secured without this crate
//! learning the application. The fields each is signed as, and their order, are the ones the
//! part's event schemas declare (`record_write`, `record_supersede`); [`SIGNED`] states them
//! here, and a test of the world crate holds it to the schemas, so that the two cannot drift.

use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

use crate::encoding;

/// The document whose events this crate secures: the standard shared record list.
pub const LIST_DOCUMENT: &str = "sce:std/shared/record_list.scxml";

/// A write to a field of a row, which its author stamped (`record_write`).
pub const WRITE_EVENT: &str = "record_list.write";

/// The note that a write was seen and written over, which the person who wrote over it made
/// (`by`, `record_supersede`).
pub const SUPERSEDE_EVENT: &str = "record_list.supersede";

/// What a host tells its machine of a member's role (`record_role`).
pub const ROLE_EVENT: &str = "record_list.role";

/// The events a signature is carried by.
pub const SIGNED_EVENTS: [&str; 2] = [WRITE.name, SUPERSEDE.name];

/// How a field is read: a whole number of the given sign, or text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Unsigned,
    Signed,
    Text,
}

/// What a field says of the operation it is signed in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// It names a write: its row, its field and its stamp. In a note, the write written over.
    Names,
    /// The content of a write, which is what is sealed when a host keeps it from the hub.
    Content,
    /// It is of the change the operation travels in: who made it (`by`), how many operations it
    /// has (`parts`) and its stamp (`atWallTime`, `atCounter`). ADR 0046 makes them signed, since
    /// a relay that changed them could hold a change back from being whole or make one part look
    /// like a change of its own; they are not what the operation is about ([`Signing::subject`]).
    Change,
}

/// One field an event is signed with, as its event schema declares it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SignedField {
    pub name: &'static str,
    pub kind: Kind,
    pub role: Role,
    /// The revision of the schema that added the field (`sce:since`), 1 for the first.
    pub since: u32,
    /// What an event that leaves the field out is read as (`sce:default`). A field of the first
    /// revision has none, since every event carries it; one added later has the default its schema
    /// gives, so that an event written before the field existed and one that carries the default
    /// are signed as the same bytes, as they mean the same.
    pub default: Option<&'static str>,
}

/// An event a signature is carried by: its name, the field that names the node whose key signs
/// it, and the fields it is signed with, in the order its schema declares them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SignedEvent {
    pub name: &'static str,
    pub author: &'static str,
    pub fields: &'static [SignedField],
}

const fn first(name: &'static str, kind: Kind, role: Role) -> SignedField {
    SignedField {
        name,
        kind,
        role,
        since: 1,
        default: None,
    }
}

const fn grown(
    name: &'static str,
    kind: Kind,
    role: Role,
    since: u32,
    default: &'static str,
) -> SignedField {
    SignedField {
        name,
        kind,
        role,
        since,
        default: Some(default),
    }
}

/// `sce:std/shared/record_write.scxml`, revision 2.
pub const WRITE: SignedEvent = SignedEvent {
    name: WRITE_EVENT,
    author: "nodeId",
    fields: &[
        first("id", Kind::Unsigned, Role::Names),
        first("field", Kind::Unsigned, Role::Names),
        first("value", Kind::Signed, Role::Content),
        first("text", Kind::Text, Role::Content),
        first("wallTime", Kind::Signed, Role::Names),
        first("counter", Kind::Unsigned, Role::Names),
        first("nodeId", Kind::Unsigned, Role::Names),
        grown("parts", Kind::Unsigned, Role::Change, 2, "1"),
    ],
};

/// `sce:std/shared/record_supersede.scxml`, revision 2.
pub const SUPERSEDE: SignedEvent = SignedEvent {
    name: SUPERSEDE_EVENT,
    author: "by",
    fields: &[
        first("id", Kind::Unsigned, Role::Names),
        first("field", Kind::Unsigned, Role::Names),
        first("wallTime", Kind::Signed, Role::Names),
        first("counter", Kind::Unsigned, Role::Names),
        first("nodeId", Kind::Unsigned, Role::Names),
        first("by", Kind::Unsigned, Role::Change),
        grown("parts", Kind::Unsigned, Role::Change, 2, "1"),
        grown("atWallTime", Kind::Signed, Role::Change, 2, "0"),
        grown("atCounter", Kind::Unsigned, Role::Change, 2, "0"),
    ],
};

/// Every event a signature is carried by.
pub static SIGNED: [SignedEvent; 2] = [WRITE, SUPERSEDE];

/// The event `event` names, when a signature is carried by it.
pub fn signed_event(event: &str) -> Option<&'static SignedEvent> {
    SIGNED.iter().find(|signed| signed.name == event)
}

/// The key of the data under which the signature travels between hosts.
pub const SIGNATURE_FIELD: &str = "sig";

/// The keys of the data under which the operation's place in its chain travels.
pub const SEQ_FIELD: &str = "seq";
pub const PREV_FIELD: &str = "prev";
pub const LOG_FIELD: &str = "mh";
pub const LOG_HASH_FIELD: &str = "mhh";
/// The key under which the operation's causal context travels ([`Link::context`]).
pub const CONTEXT_FIELD: &str = "cx";

/// The second: the events are the standard record list's, with the fields its second revision
/// added. The first named the events of a test fixture by other names and fields, and a
/// signature must not verify under two meanings.
const DOMAIN: &str = "sce-trust/event/2";
/// The second: an operation is this domain, the bytes of its event (which open with the event's
/// own domain, above) and its link, and the link now holds the operation's causal context
/// (docs/adr/0085, section 3.1), which the first did not. A signature of an operation of the first
/// form verifies under none of the second, so no relay can strip a context, or give an operation
/// signed without one as if it named nothing it follows.
const OPERATION_DOMAIN: &str = "sce-trust/operation/2";

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
    /// What the operation is about, written from the fields that name a write and its content
    /// and not from the change it travels in ([`Role::Change`]): two notes about one write have
    /// one subject, whoever wrote over it, and a write has one whether it was sent alone or as a
    /// part of a change (a copy shares it alone on a resync).
    pub subject: Vec<u8>,
    pub message: Vec<u8>,
}

/// The bytes `event` with `data` is signed as in the list `list`, and its author. `None` when
/// `event` is not one a signature is carried by.
pub fn signing(list: &str, event: &str, data: &Value) -> Option<Result<Signing, MessageError>> {
    let signed = signed_event(event)?;
    Some(build(list, signed, data))
}

/// The stamp of a write: its time and its counter, which are the author's clock when they wrote
/// it. Along one device's chain they do not go back.
pub fn stamp_of(data: &Value) -> Result<(i64, u64), MessageError> {
    let object = data.as_object().ok_or(MessageError::NotAnObject)?;
    let wall = read_as(object, "wallTime", Kind::Signed, None)?
        .parse::<i64>()
        .map_err(|_| MessageError::Field("wallTime"))?;
    let counter = read_as(object, "counter", Kind::Unsigned, None)?
        .parse::<u64>()
        .map_err(|_| MessageError::Field("counter"))?;
    Ok((wall, counter))
}

/// The number and the text a write carries, which is what is sealed when a host keeps a
/// write's content from the hub.
pub fn write_content(data: &Value) -> Result<(i64, String), MessageError> {
    let object = data.as_object().ok_or(MessageError::NotAnObject)?;
    let value = read_as(object, "value", Kind::Signed, None)?
        .parse::<i64>()
        .map_err(|_| MessageError::Field("value"))?;
    Ok((value, read_as(object, "text", Kind::Text, None)?))
}

/// What names a write, apart from its content, who it is for and the change it travels in: the
/// fields that make its stamp ([`Role::Names`]), written as the parts of a message. A write is
/// sealed under this (it is the nonce and what the cipher binds the content to), so that sealed
/// content moves to no other write.
pub fn write_stamp(data: &Value) -> Result<Vec<u8>, MessageError> {
    let object = data.as_object().ok_or(MessageError::NotAnObject)?;
    let mut stamp = Vec::new();
    for field in WRITE
        .fields
        .iter()
        .filter(|field| field.role == Role::Names)
    {
        part(&mut stamp, &read(object, field)?);
    }
    Ok(stamp)
}

/// Where an operation stands: its number among its device's operations, the id of the one before
/// it, the entry of the membership log it was made under, and what it follows of other devices.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    /// The number of the operation in its device's chain, from 1.
    pub seq: u64,
    /// The id of the operation before it ([`NO_OPERATION`] for the first).
    pub prev: [u8; 32],
    /// The position of the entry of the membership log it was made under, and that entry's hash.
    pub log: u64,
    pub log_hash: [u8; 32],
    /// The operation's causal context (docs/adr/0085, section 3.1): for each other device whose
    /// operations the author's machine had been told when the author signed it, that device's node
    /// and the highest number told, lowest node first, each number at least 1. A host tells its
    /// machine the operation only once it has told it every operation the context names. The
    /// author's own column is not in it: the operation's own number says it follows the ones
    /// before it in its chain.
    pub context: Vec<(u64, u64)>,
}

/// Whether `context` is written as a context is: nodes rising, each named once, and every number
/// at least 1.
fn context_is_well_formed(context: &[(u64, u64)]) -> bool {
    context.iter().all(|(_, seq)| *seq >= 1) && context.windows(2).all(|two| two[0].0 < two[1].0)
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
        object.insert(
            CONTEXT_FIELD.to_string(),
            Value::Array(
                self.context
                    .iter()
                    .map(|(node, seq)| {
                        Value::Array(vec![
                            Value::String(node.to_string()),
                            Value::String(seq.to_string()),
                        ])
                    })
                    .collect(),
            ),
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
        let context = match object.remove(CONTEXT_FIELD) {
            Some(Value::Array(entries)) => entries
                .iter()
                .map(|entry| {
                    let pair = entry.as_array().filter(|pair| pair.len() == 2)?;
                    let number = |at: usize| pair[at].as_str()?.parse::<u64>().ok();
                    Some((number(0)?, number(1)?))
                })
                .collect::<Option<Vec<(u64, u64)>>>()
                .filter(|context| context_is_well_formed(context))
                .ok_or_else(|| {
                    format!(
                        "its `{CONTEXT_FIELD}` is not a context (pairs of a node and a number, nodes rising, numbers from 1)"
                    )
                })?,
            _ => return Err(format!("it carries no `{CONTEXT_FIELD}`")),
        };
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
            context,
        })
    }
}

/// The bytes a device signs to make an event an operation of its chain: the event as it travels
/// (`travelling`, which is [`Signing::message`] of the data that goes on the wire) and the
/// link, its context last: how many entries it has, then each entry's node and number.
pub fn operation_bytes(travelling: &[u8], link: &Link) -> Vec<u8> {
    let mut bytes = Vec::new();
    part(&mut bytes, OPERATION_DOMAIN);
    bytes.extend_from_slice(&(travelling.len() as u64).to_be_bytes());
    bytes.extend_from_slice(travelling);
    part(&mut bytes, &link.seq.to_string());
    part(&mut bytes, &encoding::encode(&link.prev));
    part(&mut bytes, &link.log.to_string());
    part(&mut bytes, &encoding::encode(&link.log_hash));
    part(&mut bytes, &link.context.len().to_string());
    for (node, seq) in &link.context {
        part(&mut bytes, &node.to_string());
        part(&mut bytes, &seq.to_string());
    }
    bytes
}

/// What names an operation, and what the next one of its chain names as `prev`.
pub fn operation_id(operation: &[u8]) -> [u8; 32] {
    Sha256::digest(operation).into()
}

/// The event by which the hub says an instant is settled, which the members' settlement proves
/// (`crate::checkpoint::Settlement`, carried in the data under [`SETTLEMENT_FIELD`]):
/// `record_floor`.
pub const FLOOR_EVENT: &str = "record_list.floor";

/// The key of the floor's data under which the settlement travels between hosts.
pub const SETTLEMENT_FIELD: &str = "cert";

/// The instant and the hash a `record_list.floor` carries.
pub fn floor_fields(data: &Value) -> Result<(i64, u32), MessageError> {
    let object = data.as_object().ok_or(MessageError::NotAnObject)?;
    let before = read_as(object, "before", Kind::Signed, None)?
        .parse::<i64>()
        .map_err(|_| MessageError::Field("before"))?;
    let hash = read_as(object, "hash", Kind::Unsigned, None)?
        .parse::<u32>()
        .map_err(|_| MessageError::Field("hash"))?;
    Ok((before, hash))
}

fn build(list: &str, event: &SignedEvent, data: &Value) -> Result<Signing, MessageError> {
    let object = data.as_object().ok_or(MessageError::NotAnObject)?;
    let mut message = Vec::new();
    part(&mut message, DOMAIN);
    part(&mut message, list);
    part(&mut message, event.name);
    let mut subject = Vec::new();
    let mut author_node = None;
    for field in event.fields {
        let text = read(object, field)?;
        if field.name == event.author {
            author_node = text.parse::<u64>().ok();
        }
        if field.role != Role::Change {
            part(&mut subject, &text);
        }
        part(&mut message, &text);
    }
    Ok(Signing {
        author: author_node.ok_or(MessageError::Field(event.author))?,
        subject,
        message,
    })
}

pub(crate) fn part(message: &mut Vec<u8>, text: &str) {
    message.extend_from_slice(&(text.len() as u64).to_be_bytes());
    message.extend_from_slice(text.as_bytes());
}

/// A field of a signed event as the text of its value.
fn read(object: &Map<String, Value>, field: &SignedField) -> Result<String, MessageError> {
    read_as(object, field.name, field.kind, field.default)
}

/// The field `name` as the text of its value: a number is written as its digits, whether the
/// event carries it as a number or, as every 64-bit number of a payload is carried, as text. A
/// field the event leaves out is `default`, and refused when it has none.
fn read_as(
    object: &Map<String, Value>,
    name: &'static str,
    kind: Kind,
    default: Option<&'static str>,
) -> Result<String, MessageError> {
    match (kind, object.get(name)) {
        (_, None) => default.map(str::to_string).ok_or(MessageError::Field(name)),
        (Kind::Text, Some(Value::String(text))) => Ok(text.clone()),
        (Kind::Unsigned, Some(Value::String(text))) => text
            .parse::<u64>()
            .map(|n| n.to_string())
            .map_err(|_| MessageError::Field(name)),
        (Kind::Unsigned, Some(Value::Number(n))) => n
            .as_u64()
            .map(|n| n.to_string())
            .ok_or(MessageError::Field(name)),
        (Kind::Signed, Some(Value::String(text))) => text
            .parse::<i64>()
            .map(|n| n.to_string())
            .map_err(|_| MessageError::Field(name)),
        (Kind::Signed, Some(Value::Number(n))) => n
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

    fn note() -> Value {
        json!({"id":1,"field":2,"wallTime":5,"counter":0,"nodeId":3,"by":"4"})
    }

    fn signed(event: &str, data: &Value) -> Signing {
        signing("home", event, data).unwrap().unwrap()
    }

    #[test]
    fn a_write_is_signed_by_the_node_that_stamped_it() {
        let signing = signed(WRITE_EVENT, &write());
        assert_eq!(signing.author, 3);
        assert_eq!(stamp_of(&write()), Ok((1000, 0)));
        let signing = signed(SUPERSEDE_EVENT, &note());
        assert_eq!(signing.author, 4, "a note is signed by who wrote over");
        let mut other = note();
        other["by"] = json!("7");
        let other = signed(SUPERSEDE_EVENT, &other);
        assert_eq!(
            other.subject, signing.subject,
            "two notes about one write have one subject"
        );
        assert_ne!(other.message, signing.message);
        assert!(super::signing("home", "record_list.edit", &write()).is_none());
    }

    /// The events of the fixture the layer secured before the standard part are no longer signed,
    /// admitted or relayed: there is no second meaning a signature could be made under.
    #[test]
    fn only_the_standard_record_lists_events_carry_a_signature() {
        assert_eq!(
            SIGNED_EVENTS,
            ["record_list.write", "record_list.supersede"]
        );
        for event in ["todo.write", "todo.supersede"] {
            assert!(super::signing("home", event, &write()).is_none(), "{event}");
            assert!(super::signing("home", event, &note()).is_none(), "{event}");
        }
        assert_eq!(FLOOR_EVENT, "record_list.floor");
        assert_eq!(ROLE_EVENT, "record_list.role");
    }

    #[test]
    fn the_form_the_data_is_written_in_does_not_change_what_is_signed() {
        let as_text = json!({"id":"1","field":"2","value":"10","text":"","wallTime":"1000","counter":"0","nodeId":"3"});
        let as_numbers =
            json!({"nodeId":3,"counter":0,"wallTime":1000,"value":10,"field":2,"id":1,"text":""});
        assert_eq!(
            signed(WRITE_EVENT, &as_text),
            signed(WRITE_EVENT, &as_numbers)
        );
    }

    /// A field the second revision added and an event leaves out is signed as its schema's
    /// default, so an event of the first revision and one that carries the default are one set
    /// of bytes; any other value is another.
    #[test]
    fn a_field_the_second_revision_added_is_signed_and_reads_as_its_default_when_left_out() {
        let mut with_default = write();
        with_default["parts"] = json!(1);
        assert_eq!(
            signed(WRITE_EVENT, &write()),
            signed(WRITE_EVENT, &with_default)
        );
        let mut part = write();
        part["parts"] = json!(2);
        let alone = signed(WRITE_EVENT, &write());
        let one_of_two = signed(WRITE_EVENT, &part);
        assert_ne!(alone.message, one_of_two.message, "`parts` is signed");
        assert_eq!(
            alone.subject, one_of_two.subject,
            "a write is about one register and one value, sent alone or as a part"
        );
        assert_eq!(write_stamp(&write()), write_stamp(&part));

        let mut defaults = note();
        defaults["parts"] = json!("1");
        defaults["atWallTime"] = json!("0");
        defaults["atCounter"] = json!(0);
        let base = signed(SUPERSEDE_EVENT, &note());
        assert_eq!(base, signed(SUPERSEDE_EVENT, &defaults));
        for (name, value) in [
            ("parts", json!(2)),
            ("atWallTime", json!("6")),
            ("atCounter", json!(1)),
        ] {
            let mut moved = note();
            moved[name] = value;
            let moved = signed(SUPERSEDE_EVENT, &moved);
            assert_ne!(moved.message, base.message, "`{name}` is signed");
            assert_eq!(moved.subject, base.subject, "`{name}` is of the change");
        }
    }

    #[test]
    fn a_signature_does_not_move_to_another_write_or_another_list() {
        let base = signed(WRITE_EVENT, &write());
        let mut other = write();
        other["value"] = json!("11");
        assert_ne!(base, signed(WRITE_EVENT, &other));
        assert_ne!(
            base,
            signing("work", WRITE_EVENT, &write()).unwrap().unwrap()
        );
        let mut text = write();
        text["text"] = json!("a");
        text["value"] = json!("0");
        let mut shifted = write();
        shifted["text"] = json!("");
        shifted["value"] = json!("0");
        assert_ne!(signed(WRITE_EVENT, &text), signed(WRITE_EVENT, &shifted));
    }

    #[test]
    fn data_that_is_not_the_event_is_refused() {
        assert_eq!(
            signing("home", WRITE_EVENT, &json!([1])).unwrap(),
            Err(MessageError::NotAnObject)
        );
        // Every field of the first revision is one every event carries: a text left out is not
        // read as an empty one.
        for name in ["nodeId", "text", "value"] {
            let mut missing = write();
            missing.as_object_mut().unwrap().remove(name);
            assert_eq!(
                signing("home", WRITE_EVENT, &missing).unwrap(),
                Err(MessageError::Field(name))
            );
        }
        let mut negative = write();
        negative["counter"] = json!(-1);
        assert_eq!(
            signing("home", WRITE_EVENT, &negative).unwrap(),
            Err(MessageError::Field("counter"))
        );
        let mut parts = note();
        parts["parts"] = json!("many");
        assert_eq!(
            signing("home", SUPERSEDE_EVENT, &parts).unwrap(),
            Err(MessageError::Field("parts"))
        );
    }

    #[test]
    fn a_link_travels_in_the_data_and_is_taken_out_whole() {
        let link = Link {
            seq: 7,
            prev: [3; 32],
            log: 4,
            log_hash: [5; 32],
            context: vec![(2, 5), (9, 1)],
        };
        let mut data = write();
        link.write_into(&mut data).unwrap();
        assert_eq!(data["cx"], json!([["2", "5"], ["9", "1"]]));
        assert_eq!(Link::take_from(&mut data), Ok(link.clone()));
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

    /// A context is read only as it is written: nodes rising, each once, numbers from 1. Any other
    /// spelling would let two texts name one context, or name a column twice with two numbers.
    #[test]
    fn a_context_that_is_not_written_as_one_is_refused() {
        let link = Link {
            seq: 2,
            prev: [3; 32],
            log: 4,
            log_hash: [5; 32],
            context: Vec::new(),
        };
        for (context, why) in [
            (json!([["3", "1"], ["2", "1"]]), "nodes falling"),
            (json!([["2", "1"], ["2", "4"]]), "a node twice"),
            (json!([["2", "0"]]), "a number of 0"),
            (json!([["2"]]), "an entry without its number"),
            (json!([[2, 1]]), "numbers not written as text"),
            (json!("2:1"), "not a list"),
        ] {
            let mut data = write();
            link.write_into(&mut data).unwrap();
            data["cx"] = context;
            assert!(Link::take_from(&mut data).is_err(), "{why}");
        }
        let mut none = write();
        link.write_into(&mut none).unwrap();
        none.as_object_mut().unwrap().remove("cx");
        assert!(
            Link::take_from(&mut none).is_err(),
            "an operation carries its context, an empty one included"
        );
        let mut empty = write();
        link.write_into(&mut empty).unwrap();
        assert_eq!(Link::take_from(&mut empty), Ok(link));
    }

    #[test]
    fn an_operation_is_signed_as_it_stands_in_its_chain() {
        let travelling = signed(WRITE_EVENT, &write()).message;
        let link = Link {
            seq: 1,
            prev: NO_OPERATION,
            log: 3,
            log_hash: [5; 32],
            context: vec![(2, 4)],
        };
        let base = operation_bytes(&travelling, &link);
        for moved in [
            Link {
                seq: 2,
                ..link.clone()
            },
            Link {
                prev: [1; 32],
                ..link.clone()
            },
            Link {
                log: 4,
                ..link.clone()
            },
            Link {
                log_hash: [6; 32],
                ..link.clone()
            },
            Link {
                context: Vec::new(),
                ..link.clone()
            },
            Link {
                context: vec![(2, 5)],
                ..link.clone()
            },
            Link {
                context: vec![(3, 4)],
                ..link.clone()
            },
            Link {
                context: vec![(2, 4), (3, 1)],
                ..link.clone()
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
