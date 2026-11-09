// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The bytes a change is signed as, and what makes it an operation of a device's chain.
//!
//! What a change says is written from the fields it carries and the name of the list, as the
//! deterministic CBOR of their types (RFC 8949 section 4.2.1, [`crate::cbor`]; docs/adr/0086,
//! section 9), and not from the text of the event's data: a host that re-writes the data (a
//! different order of keys, a number written as text) changes no byte of what is signed, and a
//! signature does not move to another change, another kind of event, or another list. Every part
//! is a value of its type with its own length, so that no two different changes are the same
//! bytes.
//!
//! A change of the list is one unit and one message (`record_list.change`, docs/adr/0085, section
//! 1): its author, its stamp, and its operations in their order, a list of records. It is signed
//! once, whole, as one operation of its author's chain, so a change is atomic in the signature as
//! it is on the wire: no relay can hold, drop or reorder a part of one.
//!
//! An event a device sends is an **operation** of that device's chain: the event as it travels
//! (the content of its writes sealed, when the list keeps it from the hub), the number it has among
//! the device's operations (`seq`), the id of the one before it (`prev`), the entry of the
//! membership log it was made under (`mh`, and that entry's hash `mhh`), and its causal context, the
//! highest operation of each other device its machine had been told (`cx`). The device signs all of
//! it ([`operation_bytes`]), so that an operation neither moves to another chain position, nor
//! claims another entry of the log, nor leaves out what it follows, in its chain or in another's: a
//! relay cannot change the order a host tells its machine operations in.
//!
//! The event is the standard shared record list's ([`LIST_DOCUMENT`], docs/adr/0042), and no other:
//! a list an application builds on that part is secured without this crate learning the
//! application. The fields a change is signed with, and their types, are the ones the part's event
//! schemas declare (`record_change`, and `record_op` for each operation); [`SIGNED`] states them
//! here, and a test of the world crate holds it to the schemas, so that the two cannot drift.

use std::collections::BTreeMap;
use std::convert::Infallible;

use sce_forge_runtime::cbor::{write_array_head, write_bytes, write_text};
use sce_forge_runtime::codec::VecSink;
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

use crate::cbor::{self, Field, Integer, RecordType, Shape, Step, ValueType};
use crate::encoding;

/// The document whose events this crate secures: the standard shared record list.
pub const LIST_DOCUMENT: &str = "sce:std/shared/record_list.scxml";

/// A change to the list: its author, its stamp and its operations, one message
/// (`record_change`).
pub const CHANGE_EVENT: &str = "record_list.change";

/// What a host tells its machine of a member's role (`record_role`).
pub const ROLE_EVENT: &str = "record_list.role";

/// The events a signature is carried by.
pub const SIGNED_EVENTS: [&str; 1] = [CHANGE.name];

/// The kind of an operation that writes a register (`record_op.kind`).
pub const WRITE_KIND: u64 = 0;

/// The kind of an operation that writes a value over (`record_op.kind`).
pub const WRITE_OVER_KIND: u64 = 1;

/// No bound of the build is held here: a deployment moves `shared_parts` and `merge_text`
/// (`--limit`), and the text of a write the list keeps from the hub travels sealed, longer than the
/// text it seals. The build and the list refuse a change past its bounds; what is signed is any
/// value of the change's shape.
const UNBOUNDED: u32 = u32::MAX;

const fn field(id: &'static str, ty: ValueType) -> Field {
    Field {
        id,
        ty,
        since: 1,
        default: None,
    }
}

const fn integer(integer: Integer) -> ValueType {
    ValueType::Required(Shape::Integer(integer))
}

const TEXT: ValueType = ValueType::Required(Shape::Text { max: UNBOUNDED });

/// `sce:std/shared/record_op.scxml`, revision 1: one operation of a change. A write (`kind` 0)
/// gives the register `id`, `field` the number `value` and the text `text`; a write-over (`kind` 1)
/// takes out of the register the value of the write whose dot is `wallTime`, `counter`, `nodeId`. A
/// write's own dot is its change's stamp, and the dot it carries is 0.
pub const OP: RecordType = RecordType {
    fields: &[
        field("kind", integer(Integer::Uint8)),
        field("id", integer(Integer::Uint32)),
        field("field", integer(Integer::Uint16)),
        field("value", integer(Integer::Int64)),
        field("text", TEXT),
        field("wallTime", integer(Integer::Int64)),
        field("counter", integer(Integer::Uint32)),
        field("nodeId", integer(Integer::Uint64)),
    ],
};

/// `sce:std/shared/record_change.scxml`, revision 1: a change, its author `by`, its stamp
/// (`wallTime`, `counter`) and its operations in their order.
pub const CHANGE_TYPE: RecordType = RecordType {
    fields: &[
        field("by", integer(Integer::Uint64)),
        field("wallTime", integer(Integer::Int64)),
        field("counter", integer(Integer::Uint32)),
        field(
            "ops",
            ValueType::Required(Shape::List {
                elem: &ValueType::Required(Shape::Record(&OP)),
                max: UNBOUNDED,
            }),
        ),
    ],
};

/// What names a change: its author and its stamp, which is the dot of every write in it
/// (docs/adr/0046, section 1). Its subject ([`Signing::subject`]).
const DOT: RecordType = RecordType {
    fields: &[
        field("by", integer(Integer::Uint64)),
        field("wallTime", integer(Integer::Int64)),
        field("counter", integer(Integer::Uint32)),
    ],
};

/// An event a signature is carried by: its name, the field that names the node whose key signs
/// it, its payload's type (every field its schema declares, in the order it declares them), and
/// what names it apart from what it does.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SignedEvent {
    pub name: &'static str,
    pub author: &'static str,
    pub payload: &'static RecordType,
    /// The fields that name the event, a record of its own: what it is about, whatever it does.
    pub names: &'static RecordType,
}

/// `record_list.change`: signed by its author, `by`, and named by its dot.
pub const CHANGE: SignedEvent = SignedEvent {
    name: CHANGE_EVENT,
    author: "by",
    payload: &CHANGE_TYPE,
    names: &DOT,
};

/// Every event a signature is carried by.
pub static SIGNED: [SignedEvent; 1] = [CHANGE];

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

/// The third: the one signed event is a change of the list (docs/adr/0085, section 1), whose
/// operations are a list of records, and an event is written as the deterministic CBOR of its
/// fields by their types (docs/adr/0086, section 9). The second signed each part of a change as an
/// event of its own (`record_list.write`, `record_list.supersede`), as length-prefixed texts, and a
/// signature must not verify under two meanings: a part signed under the second is no change here.
const DOMAIN: &str = "sce-trust/event/3";
/// The second: an operation is the CBOR array of this domain, the bytes of its event (which open
/// with the event's own domain, above) as a byte string, and its link as a map, the link holding
/// the operation's causal context (docs/adr/0085, section 3.1), which the first did not. A
/// signature of an operation of the first form verifies under none of the second, so no relay can
/// strip a context, or give an operation signed without one as if it named nothing it follows.
const OPERATION_DOMAIN: &str = "sce-trust/operation/2";

/// The id an operation has before any: what the first operation of a chain names as `prev`.
pub const NO_OPERATION: [u8; 32] = [0; 32];

/// Why an event's data could not be read as the event it names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageError {
    /// The data is not a JSON object.
    NotAnObject,
    /// A field is missing, or is not a value of the type its schema declares; in an operation of a
    /// change, the operation's field.
    Field(&'static str),
    /// The change holds no operation: a change that names nothing is no change, and its schema
    /// holds at least one.
    Empty,
    /// The change writes one register twice. Its writes share a dot, so the two would be one write
    /// with two contents, and one sealed under one nonce twice.
    WrittenTwice,
}

/// What an event is signed as, and who signed it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signing {
    /// The node of the person whose key signed it: `by` of a change.
    pub author: u64,
    /// What the event is about: the encoding of the fields that name it (its author and its stamp,
    /// [`SignedEvent::names`]), whatever it does. One change has one subject, and a second change
    /// under it is another content under one dot.
    pub subject: Vec<u8>,
    pub message: Vec<u8>,
}

/// The bytes `event` with `data` is signed as in the list `list`, and its author. `None` when
/// `event` is not one a signature is carried by.
pub fn signing(list: &str, event: &str, data: &Value) -> Option<Result<Signing, MessageError>> {
    let signed = signed_event(event)?;
    Some(build(list, signed, data))
}

/// The stamp of a change: its time and its counter, which are the author's clock when they made
/// it. Along one device's chain they do not go back.
pub fn stamp_of(data: &Value) -> Result<(i64, u64), MessageError> {
    let object = data.as_object().ok_or(MessageError::NotAnObject)?;
    let wall = read_as(object, "wallTime", Kind::Signed)?
        .parse::<i64>()
        .map_err(|_| MessageError::Field("wallTime"))?;
    let counter = read_as(object, "counter", Kind::Unsigned)?
        .parse::<u64>()
        .map_err(|_| MessageError::Field("counter"))?;
    Ok((wall, counter))
}

/// The operations of a change, each as the object it travels as, in their order. A change holds
/// at least one.
pub fn ops_of(data: &Value) -> Result<&[Value], MessageError> {
    let object = data.as_object().ok_or(MessageError::NotAnObject)?;
    let ops = object
        .get("ops")
        .and_then(Value::as_array)
        .ok_or(MessageError::Field("ops"))?;
    if ops.is_empty() {
        return Err(MessageError::Empty);
    }
    if !ops.iter().all(Value::is_object) {
        return Err(MessageError::Field("ops"));
    }
    Ok(ops.as_slice())
}

/// Whether the operation `op` of a change writes a register (`kind` 0), and not a value over
/// (`kind` 1). An operation of any other kind is no operation of a change.
pub fn is_write(op: &Value) -> Result<bool, MessageError> {
    let object = op.as_object().ok_or(MessageError::Field("ops"))?;
    match read_as(object, "kind", Kind::Unsigned)?.parse::<u64>() {
        Ok(WRITE_KIND) => Ok(true),
        Ok(WRITE_OVER_KIND) => Ok(false),
        _ => Err(MessageError::Field("kind")),
    }
}

/// The number and the text a write carries, which is what is sealed when a host keeps a write's
/// content from the hub.
pub fn write_content(op: &Value) -> Result<(i64, String), MessageError> {
    let object = op.as_object().ok_or(MessageError::Field("ops"))?;
    let value = read_as(object, "value", Kind::Signed)?
        .parse::<i64>()
        .map_err(|_| MessageError::Field("value"))?;
    Ok((value, read_as(object, "text", Kind::Text)?))
}

/// What names the write `op` of the change `change`: its register (`id`, `field`) and its dot,
/// which is the change's stamp and author, written as the parts of a message. A write is sealed
/// under this (it is the nonce and what the cipher binds the content to), so that sealed content
/// moves to no other write, and two writes of one change are sealed under two.
pub fn write_stamp(change: &Value, op: &Value) -> Result<Vec<u8>, MessageError> {
    let change = change.as_object().ok_or(MessageError::NotAnObject)?;
    let op = op.as_object().ok_or(MessageError::Field("ops"))?;
    let mut stamp = Vec::new();
    part(&mut stamp, &read_as(op, "id", Kind::Unsigned)?);
    part(&mut stamp, &read_as(op, "field", Kind::Unsigned)?);
    part(&mut stamp, &read_as(change, "wallTime", Kind::Signed)?);
    part(&mut stamp, &read_as(change, "counter", Kind::Unsigned)?);
    part(&mut stamp, &read_as(change, "by", Kind::Unsigned)?);
    Ok(stamp)
}

/// The writes of a change, each as its place in `ops` and what it is sealed under
/// ([`write_stamp`]), in their order; the write-overs are not among them. Refused when an
/// operation is of no kind a change holds, or when two writes name one register.
pub fn writes_of(change: &Value) -> Result<Vec<(usize, Vec<u8>)>, MessageError> {
    let mut writes: Vec<(usize, Vec<u8>)> = Vec::new();
    for (at, op) in ops_of(change)?.iter().enumerate() {
        if !is_write(op)? {
            continue;
        }
        let stamp = write_stamp(change, op)?;
        if writes.iter().any(|(_, held)| *held == stamp) {
            return Err(MessageError::WrittenTwice);
        }
        writes.push((at, stamp));
    }
    Ok(writes)
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

/// One entry of a causal context: a device's node and the highest number of its chain told.
const CONTEXT_ENTRY: RecordType = RecordType {
    fields: &[
        field("node", integer(Integer::Uint64)),
        field("seq", integer(Integer::Uint64)),
    ],
};

/// A link as it is signed: a map of its fields, keyed as they travel.
const LINK: RecordType = RecordType {
    fields: &[
        field(SEQ_FIELD, integer(Integer::Uint64)),
        field(PREV_FIELD, ValueType::Required(Shape::Bytes { max: 32 })),
        field(LOG_FIELD, integer(Integer::Uint64)),
        field(
            LOG_HASH_FIELD,
            ValueType::Required(Shape::Bytes { max: 32 }),
        ),
        field(
            CONTEXT_FIELD,
            ValueType::Required(Shape::List {
                elem: &ValueType::Required(Shape::Record(&CONTEXT_ENTRY)),
                max: UNBOUNDED,
            }),
        ),
    ],
};

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

    /// The link as the typed value it is signed as.
    fn to_value(&self) -> cbor::Value {
        let number = |n: u64| cbor::Value::Integer(i128::from(n));
        let entries = self
            .context
            .iter()
            .map(|(node, seq)| {
                cbor::Value::Record(BTreeMap::from([
                    ("node".to_string(), number(*node)),
                    ("seq".to_string(), number(*seq)),
                ]))
            })
            .collect();
        cbor::Value::Record(BTreeMap::from([
            (SEQ_FIELD.to_string(), number(self.seq)),
            (
                PREV_FIELD.to_string(),
                cbor::Value::Bytes(self.prev.to_vec()),
            ),
            (LOG_FIELD.to_string(), number(self.log)),
            (
                LOG_HASH_FIELD.to_string(),
                cbor::Value::Bytes(self.log_hash.to_vec()),
            ),
            (CONTEXT_FIELD.to_string(), cbor::Value::List(entries)),
        ]))
    }
}

/// What a write into a growing vector returns: it cannot fail.
fn put(written: Result<(), Infallible>) {
    written.unwrap_or_else(|never| match never {});
}

/// The bytes a device signs to make an event an operation of its chain: the CBOR array of the
/// operation domain, the event as it travels (`travelling`, which is [`Signing::message`] of the
/// data that goes on the wire) as a byte string, and the link as a map of its fields, its context
/// a list of records of a node and a number.
pub fn operation_bytes(travelling: &[u8], link: &Link) -> Vec<u8> {
    let link = cbor::encode(&link.to_value(), &ValueType::Required(Shape::Record(&LINK))).expect(
        "every link is a value of the link's type: its numbers are 64-bit, its hashes 32 bytes",
    );
    let mut bytes = Vec::new();
    {
        let mut sink = VecSink::new(&mut bytes);
        put(write_array_head(&mut sink, 3));
        put(write_text(&mut sink, OPERATION_DOMAIN));
        put(write_bytes(&mut sink, travelling));
    }
    bytes.extend_from_slice(&link);
    bytes
}

/// What names an operation, and what the next one of its chain names as `prev`.
pub fn operation_id(operation: &[u8]) -> [u8; 32] {
    Sha256::digest(operation).into()
}

/// The event by which the hub says an instant is settled, which the members' settlement proves
/// (`crate::checkpoint::Settlement`, carried in the data under [`SETTLEMENT_FIELD`]): the core's
/// word of stability, which docs/adr/0085 landing 5 replaces with the stable cut.
pub const FLOOR_EVENT: &str = "record_list.floor";

/// The key of the floor's data under which the settlement travels between hosts.
pub const SETTLEMENT_FIELD: &str = "cert";

/// The instant and the hash a `record_list.floor` carries.
pub fn floor_fields(data: &Value) -> Result<(i64, u32), MessageError> {
    let object = data.as_object().ok_or(MessageError::NotAnObject)?;
    let before = read_as(object, "before", Kind::Signed)?
        .parse::<i64>()
        .map_err(|_| MessageError::Field("before"))?;
    let hash = read_as(object, "hash", Kind::Unsigned)?
        .parse::<u32>()
        .map_err(|_| MessageError::Field("hash"))?;
    Ok((before, hash))
}

fn build(list: &str, event: &SignedEvent, data: &Value) -> Result<Signing, MessageError> {
    let object = data.as_object().ok_or(MessageError::NotAnObject)?;
    // An empty change is refused by name before its type is read: the type bounds the list's
    // length, not its least, which is the schema's `[1..]`.
    if let Some(Value::Array(ops)) = object.get("ops") {
        if ops.is_empty() {
            return Err(MessageError::Empty);
        }
    }
    let payload = encoded(data, event.payload)?;
    let subject = encoded(data, event.names)?;
    let author = read_as(object, event.author, Kind::Unsigned)?
        .parse::<u64>()
        .map_err(|_| MessageError::Field(event.author))?;
    let mut message = Vec::new();
    {
        let mut sink = VecSink::new(&mut message);
        put(write_array_head(&mut sink, 4));
        put(write_text(&mut sink, DOMAIN));
        put(write_text(&mut sink, list));
        put(write_text(&mut sink, event.name));
    }
    message.extend_from_slice(&payload);
    Ok(Signing {
        author,
        subject,
        message,
    })
}

/// The deterministic CBOR of `data` as a value of `record`: each field the record declares, read
/// by its type, and nothing else of `data`.
fn encoded(data: &Value, record: &'static RecordType) -> Result<Vec<u8>, MessageError> {
    let ty = ValueType::Required(Shape::Record(record));
    let value = typed(data, &ty, None)?;
    cbor::encode(&value, &ty).map_err(|refused| MessageError::Field(innermost(&refused.at)))
}

/// The id of the innermost field of `path`, or the change's operations when the path names none.
fn innermost(path: &[Step]) -> &'static str {
    path.iter()
        .rev()
        .find_map(|step| match step {
            Step::Field(id) => Some(*id),
            Step::Element(_) => None,
        })
        .unwrap_or("ops")
}

/// `json` read as a value of `ty`: a number written as its digits or as a number, a text, a list
/// or a record. `name` is the field it is the value of, for the refusal.
fn typed(
    json: &Value,
    ty: &ValueType,
    name: Option<&'static str>,
) -> Result<cbor::Value, MessageError> {
    let refused = || match name {
        Some(name) => MessageError::Field(name),
        None => MessageError::NotAnObject,
    };
    match ty.shape() {
        Shape::Integer(_) | Shape::Enum(_) => match json {
            Value::Number(n) => n
                .as_i64()
                .map(i128::from)
                .or_else(|| n.as_u64().map(i128::from))
                .map(cbor::Value::Integer)
                .ok_or_else(refused),
            Value::String(text) => text
                .parse::<i128>()
                .map(cbor::Value::Integer)
                .map_err(|_| refused()),
            _ => Err(refused()),
        },
        Shape::Text { .. } => match json {
            Value::String(text) => Ok(cbor::Value::Text(text.clone())),
            _ => Err(refused()),
        },
        Shape::List { elem, .. } => match json {
            Value::Array(items) => items
                .iter()
                .map(|item| typed(item, elem, name))
                .collect::<Result<_, _>>()
                .map(cbor::Value::List),
            _ => Err(refused()),
        },
        Shape::Record(record) => {
            let object = json.as_object().ok_or_else(refused)?;
            let mut fields = BTreeMap::new();
            for field in record.fields {
                if let Some(value) = object.get(field.id) {
                    fields.insert(
                        field.id.to_string(),
                        typed(value, &field.ty, Some(field.id))?,
                    );
                }
            }
            Ok(cbor::Value::Record(fields))
        }
        Shape::Float32 | Shape::Float64 | Shape::Bool | Shape::Bytes { .. } => Err(refused()),
    }
}

pub(crate) fn part(message: &mut Vec<u8>, text: &str) {
    message.extend_from_slice(&(text.len() as u64).to_be_bytes());
    message.extend_from_slice(text.as_bytes());
}

/// How a field is read where its text is taken: a whole number of the given sign, or text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Unsigned,
    Signed,
    Text,
}

/// The field `name` as the text of its value: a number is written as its digits, whether the
/// event carries it as a number or, as every 64-bit number of a payload is carried, as text. A
/// field the event leaves out is refused.
fn read_as(
    object: &Map<String, Value>,
    name: &'static str,
    kind: Kind,
) -> Result<String, MessageError> {
    match (kind, object.get(name)) {
        (_, None) => Err(MessageError::Field(name)),
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

    fn write(id: u64, field: u64, text: &str) -> Value {
        json!({"kind":0,"id":id,"field":field,"value":"10","text":text,"wallTime":"0","counter":0,"nodeId":"0"})
    }

    fn write_over(id: u64, field: u64, by: u64) -> Value {
        json!({"kind":1,"id":id,"field":field,"value":"0","text":"","wallTime":"900","counter":0,"nodeId":by.to_string()})
    }

    fn change() -> Value {
        json!({"by":"3","wallTime":"1000","counter":0,"ops":[write_over(1, 2, 4), write(1, 2, "milk")]})
    }

    fn signed(event: &str, data: &Value) -> Signing {
        signing("home", event, data).unwrap().unwrap()
    }

    #[test]
    fn a_change_is_signed_by_its_author_and_named_by_its_dot() {
        let signing = signed(CHANGE_EVENT, &change());
        assert_eq!(signing.author, 3);
        assert_eq!(stamp_of(&change()), Ok((1000, 0)));
        let mut other = change();
        other["ops"] = json!([write(9, 1, "tea")]);
        let other = signed(CHANGE_EVENT, &other);
        assert_eq!(
            other.subject, signing.subject,
            "one dot names one change, whatever it does"
        );
        assert_ne!(other.message, signing.message);
        let mut later = change();
        later["counter"] = json!(1);
        assert_ne!(signed(CHANGE_EVENT, &later).subject, signing.subject);
        assert!(super::signing("home", "record_list.edit", &change()).is_none());
    }

    /// The one-operation events of the list's parts are no longer signed, admitted or relayed:
    /// there is no second shape of a change a signature could be made under (docs/adr/0085,
    /// section 1.4).
    #[test]
    fn only_a_change_of_the_standard_record_list_carries_a_signature() {
        assert_eq!(SIGNED_EVENTS, ["record_list.change"]);
        let part = json!({"id":1,"field":2,"value":"10","text":"","wallTime":"1000","counter":0,"nodeId":"3","by":"3"});
        for event in [
            "record_list.write",
            "record_list.supersede",
            "todo.write",
            "todo.supersede",
        ] {
            assert!(super::signing("home", event, &part).is_none(), "{event}");
        }
        assert_eq!(FLOOR_EVENT, "record_list.floor");
        assert_eq!(ROLE_EVENT, "record_list.role");
    }

    #[test]
    fn the_form_the_data_is_written_in_does_not_change_what_is_signed() {
        let as_text = json!({"by":"3","wallTime":"1000","counter":"0","ops":[
            {"kind":"1","id":"1","field":"2","value":"0","text":"","wallTime":"900","counter":"0","nodeId":"4"},
            {"kind":"0","id":"1","field":"2","value":"10","text":"milk","wallTime":"0","counter":"0","nodeId":"0"}]});
        let as_numbers = json!({"ops":[
            {"nodeId":4,"counter":0,"wallTime":900,"text":"","value":0,"field":2,"id":1,"kind":1},
            {"nodeId":0,"counter":0,"wallTime":0,"text":"milk","value":10,"field":2,"id":1,"kind":0}],
            "counter":0,"wallTime":1000,"by":3});
        assert_eq!(
            signed(CHANGE_EVENT, &as_text),
            signed(CHANGE_EVENT, &as_numbers)
        );
        let mut extra = change();
        extra["hub"] = json!(true);
        extra["ops"][1]["note"] = json!("not declared");
        assert_eq!(
            signed(CHANGE_EVENT, &extra),
            signed(CHANGE_EVENT, &change()),
            "a key the schemas do not declare is not signed"
        );
    }

    /// Every operation of a change is signed, in its order: a change cannot lose, gain or
    /// reorder one under its author's signature.
    #[test]
    fn every_operation_of_a_change_is_signed_in_its_order() {
        let base = signed(CHANGE_EVENT, &change()).message;
        let mut fewer = change();
        fewer["ops"] = json!([write(1, 2, "milk")]);
        let mut more = change();
        more["ops"]
            .as_array_mut()
            .unwrap()
            .push(write(1, 3, "today"));
        let mut swapped = change();
        swapped["ops"] = json!([write(1, 2, "milk"), write_over(1, 2, 4)]);
        let mut renamed = change();
        renamed["ops"][0]["nodeId"] = json!("5");
        let mut rewritten = change();
        rewritten["ops"][1]["text"] = json!("oat milk");
        for (moved, why) in [
            (fewer, "an operation dropped"),
            (more, "an operation added"),
            (swapped, "two operations swapped"),
            (renamed, "the dot a write-over names"),
            (rewritten, "the text of a write"),
        ] {
            assert_ne!(signed(CHANGE_EVENT, &moved).message, base, "{why}");
        }
    }

    #[test]
    fn a_signature_does_not_move_to_another_change_or_another_list() {
        let base = signed(CHANGE_EVENT, &change());
        assert_ne!(
            base,
            signing("work", CHANGE_EVENT, &change()).unwrap().unwrap()
        );
        let mut text = change();
        text["ops"][1]["text"] = json!("a");
        text["ops"][1]["value"] = json!("0");
        let mut shifted = change();
        shifted["ops"][1]["text"] = json!("");
        shifted["ops"][1]["value"] = json!("0");
        assert_ne!(signed(CHANGE_EVENT, &text), signed(CHANGE_EVENT, &shifted));
    }

    #[test]
    fn data_that_is_not_a_change_is_refused() {
        assert_eq!(
            signing("home", CHANGE_EVENT, &json!([1])).unwrap(),
            Err(MessageError::NotAnObject)
        );
        let mut empty = change();
        empty["ops"] = json!([]);
        assert_eq!(
            signing("home", CHANGE_EVENT, &empty).unwrap(),
            Err(MessageError::Empty),
            "a change that names nothing is no change"
        );
        // Every field is one every change carries: a text left out is not read as an empty one.
        for name in ["by", "wallTime", "ops"] {
            let mut missing = change();
            missing.as_object_mut().unwrap().remove(name);
            assert_eq!(
                signing("home", CHANGE_EVENT, &missing).unwrap(),
                Err(MessageError::Field(name)),
                "{name}"
            );
        }
        for name in ["text", "nodeId", "kind"] {
            let mut missing = change();
            missing["ops"][1].as_object_mut().unwrap().remove(name);
            assert_eq!(
                signing("home", CHANGE_EVENT, &missing).unwrap(),
                Err(MessageError::Field(name)),
                "an operation's {name}"
            );
        }
        let mut negative = change();
        negative["counter"] = json!(-1);
        assert_eq!(
            signing("home", CHANGE_EVENT, &negative).unwrap(),
            Err(MessageError::Field("counter"))
        );
        let mut wide = change();
        wide["ops"][0]["field"] = json!(65_536);
        assert_eq!(
            signing("home", CHANGE_EVENT, &wide).unwrap(),
            Err(MessageError::Field("field")),
            "a field is a uint16"
        );
        let mut not_a_list = change();
        not_a_list["ops"] = json!({"kind": 0});
        assert_eq!(
            signing("home", CHANGE_EVENT, &not_a_list).unwrap(),
            Err(MessageError::Field("ops"))
        );
    }

    /// A write of a change is sealed under its register and the change's dot, and only a write is:
    /// two writes of one change are sealed under two stamps, and a change that writes one register
    /// twice is refused, since its two writes would be one write sealed under one nonce twice.
    #[test]
    fn the_writes_of_a_change_are_named_by_their_register_and_its_dot() {
        let data = change();
        let writes = writes_of(&data).unwrap();
        assert_eq!(writes.len(), 1, "the write-over carries no content");
        assert_eq!(writes[0].0, 1);
        let mut expected = Vec::new();
        for text in ["1", "2", "1000", "0", "3"] {
            part(&mut expected, text);
        }
        assert_eq!(writes[0].1, expected, "id, field, wallTime, counter, by");
        assert_eq!(write_content(&data["ops"][1]), Ok((10, "milk".to_string())));

        let mut two = change();
        two["ops"] = json!([write(1, 2, "milk"), write(1, 3, "today")]);
        let stamps = writes_of(&two).unwrap();
        assert_ne!(stamps[0].1, stamps[1].1);

        let mut twice = change();
        twice["ops"] = json!([write(1, 2, "milk"), write(1, 2, "tea")]);
        assert_eq!(writes_of(&twice), Err(MessageError::WrittenTwice));

        let mut strange = change();
        strange["ops"][0]["kind"] = json!(2);
        assert_eq!(writes_of(&strange), Err(MessageError::Field("kind")));
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
        let mut data = change();
        link.write_into(&mut data).unwrap();
        assert_eq!(data["cx"], json!([["2", "5"], ["9", "1"]]));
        assert_eq!(Link::take_from(&mut data), Ok(link.clone()));
        assert_eq!(
            data,
            change(),
            "what is left is the event as its author made it"
        );
        assert!(Link::take_from(&mut data).is_err(), "it was taken out");
        let mut bad = change();
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
            let mut data = change();
            link.write_into(&mut data).unwrap();
            data["cx"] = context;
            assert!(Link::take_from(&mut data).is_err(), "{why}");
        }
        let mut none = change();
        link.write_into(&mut none).unwrap();
        none.as_object_mut().unwrap().remove("cx");
        assert!(
            Link::take_from(&mut none).is_err(),
            "an operation carries its context, an empty one included"
        );
        let mut empty = change();
        link.write_into(&mut empty).unwrap();
        assert_eq!(Link::take_from(&mut empty), Ok(link));
    }

    #[test]
    fn an_operation_is_signed_as_it_stands_in_its_chain() {
        let travelling = signed(CHANGE_EVENT, &change()).message;
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
