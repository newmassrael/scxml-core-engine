// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The parts a change of the list was signed as before a change was one unit, read and never
//! written.
//!
//! Before docs/adr/0085 landing 4c each operation of a change was a signed event of its own: a
//! write (`record_list.write`, `record_write` revision 2) or a write-over (`record_list.supersede`,
//! `record_supersede` revision 2), under the second event domain (`sce-trust/event/2`), each field
//! written as a text after its length in the order its schema declares it, and made an operation
//! of its author's chain under the first operation domain (`sce-trust/operation/1`), whose link
//! holds no causal context. The content of a write the list kept from the hub was sealed under the
//! write's own stamp and the first content domain (`crate::secrecy::open_part`). A relay, and a
//! device's sealed state, written before that landing hold such parts.
//!
//! This module gives the bytes a part was signed as, for its signature to be checked
//! ([`signing`], [`operation_bytes`]), and what a part says ([`read`]). It signs nothing and
//! makes no part: no core of docs/adr/0085 signs under the second domain (section 8.1). What the
//! parts a host holds make together is folded into one state ([`fold`], docs/adr/0085, section
//! 8.3), which a host gives its copy as a stable state (`record_list.restore`), and the sends of a
//! part that a device saved unsigned are carried as changes ([`changes_of_sends`]).

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{json, Map, Value};

use crate::encoding;
use crate::message::{part, read_as, Kind, Link, MessageError, Signing, CONTEXT_FIELD};

/// A write to a field of a row, which its author stamped (`record_write`).
pub const WRITE_EVENT: &str = "record_list.write";

/// The note that a write was seen and written over, which the person who wrote over it made
/// (`by`, `record_supersede`).
pub const SUPERSEDE_EVENT: &str = "record_list.supersede";

/// The events a part was signed as.
pub const EVENTS: [&str; 2] = [WRITE_EVENT, SUPERSEDE_EVENT];

/// The second event domain, under which every part was signed.
const DOMAIN: &str = "sce-trust/event/2";
/// The first operation domain: an operation was this domain, the bytes of its event and its link
/// (its number, the one before it and the entry of the log), each written after its length.
const OPERATION_DOMAIN: &str = "sce-trust/operation/1";

/// What a field says of the part it is signed in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    /// It names a write: its row, its field and its stamp; in a write-over, the write written
    /// over. What a part is about ([`Signing::subject`]).
    Names,
    /// The content of a write, which is what was sealed when the list kept it from the hub.
    Content,
    /// It is of the change the part travelled in: who made it, how many parts it has and its
    /// stamp. Signed, and not what the part is about.
    Change,
}

/// One field a part is signed with, as its schema's revision 2 declares it: its name, how it is
/// read, what it says, and what a part that leaves it out is read as (a field the second revision
/// added; one of the first has none).
struct Field {
    name: &'static str,
    kind: Kind,
    role: Role,
    default: Option<&'static str>,
}

const fn first(name: &'static str, kind: Kind, role: Role) -> Field {
    Field {
        name,
        kind,
        role,
        default: None,
    }
}

const fn grown(name: &'static str, kind: Kind, default: &'static str) -> Field {
    Field {
        name,
        kind,
        role: Role::Change,
        default: Some(default),
    }
}

/// `record_write` revision 2, signed by `nodeId`.
const WRITE_FIELDS: [Field; 8] = [
    first("id", Kind::Unsigned, Role::Names),
    first("field", Kind::Unsigned, Role::Names),
    first("value", Kind::Signed, Role::Content),
    first("text", Kind::Text, Role::Content),
    first("wallTime", Kind::Signed, Role::Names),
    first("counter", Kind::Unsigned, Role::Names),
    first("nodeId", Kind::Unsigned, Role::Names),
    grown("parts", Kind::Unsigned, "1"),
];

/// `record_supersede` revision 2, signed by `by`.
const SUPERSEDE_FIELDS: [Field; 9] = [
    first("id", Kind::Unsigned, Role::Names),
    first("field", Kind::Unsigned, Role::Names),
    first("wallTime", Kind::Signed, Role::Names),
    first("counter", Kind::Unsigned, Role::Names),
    first("nodeId", Kind::Unsigned, Role::Names),
    first("by", Kind::Unsigned, Role::Change),
    grown("parts", Kind::Unsigned, "1"),
    grown("atWallTime", Kind::Signed, "0"),
    grown("atCounter", Kind::Unsigned, "0"),
];

/// The name of a part's event as a host keeps it, when `event` is one.
pub fn event(event: &str) -> Option<&'static str> {
    EVENTS.iter().copied().find(|name| *name == event)
}

/// The fields of the part `event` and the one that names its author.
fn fields_of(event: &str) -> Option<(&'static [Field], &'static str)> {
    match event {
        WRITE_EVENT => Some((&WRITE_FIELDS, "nodeId")),
        SUPERSEDE_EVENT => Some((&SUPERSEDE_FIELDS, "by")),
        _ => None,
    }
}

/// A field of a part as the text of its value, its default when the part leaves it out.
fn read_field(object: &Map<String, Value>, field: &Field) -> Result<String, MessageError> {
    match (object.get(field.name), field.default) {
        (None, Some(default)) => Ok(default.to_string()),
        _ => read_as(object, field.name, field.kind),
    }
}

/// The bytes the part `event` with `data` was signed as in the list `list`, and its author: the
/// second event domain, the list and the event's name, then each field the schema declares, in
/// its order, each written after its length. Its subject is the fields that name the write, and
/// not those of the change it travelled in. `None` when `event` is no part.
pub fn signing(list: &str, event: &str, data: &Value) -> Option<Result<Signing, MessageError>> {
    let (fields, author) = fields_of(event)?;
    Some(build(list, event, fields, author, data))
}

fn build(
    list: &str,
    event: &str,
    fields: &[Field],
    author: &'static str,
    data: &Value,
) -> Result<Signing, MessageError> {
    let object = data.as_object().ok_or(MessageError::NotAnObject)?;
    let mut message = Vec::new();
    part(&mut message, DOMAIN);
    part(&mut message, list);
    part(&mut message, event);
    let mut subject = Vec::new();
    let mut node = None;
    for field in fields {
        let text = read_field(object, field)?;
        if field.name == author {
            node = text.parse::<u64>().ok();
        }
        if field.role != Role::Change {
            part(&mut subject, &text);
        }
        part(&mut message, &text);
    }
    Ok(Signing {
        author: node.ok_or(MessageError::Field(author))?,
        subject,
        message,
    })
}

/// The bytes a device signed to make a part an operation of its chain: the first operation
/// domain, the part as it travels (`travelling`, the [`Signing::message`] of the data on the
/// wire) and its link's number, the one before it and the entry of the log, each after its
/// length. A link of the first domain holds no context, and none is read here.
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

/// Take a part's link out of its data, leaving the part as its author made it: its number, the
/// one before it and the entry of the log. A part that carries a context is refused: its author
/// signed none, and a context added on the way would be a second wire form of one operation.
pub fn take_link(data: &mut Value) -> Result<Link, String> {
    let link = Link::take_place(data)?;
    let carries_context = data
        .as_object()
        .is_some_and(|object| object.contains_key(CONTEXT_FIELD));
    if carries_context {
        return Err(format!(
            "it carries `{CONTEXT_FIELD}`, and a part of the second domain was signed with none"
        ));
    }
    Ok(link)
}

/// What a write was sealed under: the fields that name it (its row, its field, its time, its
/// counter and its author), each written after its length. Its content was sealed under this, and
/// two writes under one such stamp are the nonce used twice.
pub fn write_stamp(data: &Value) -> Result<Vec<u8>, MessageError> {
    let object = data.as_object().ok_or(MessageError::NotAnObject)?;
    let mut stamp = Vec::new();
    for field in WRITE_FIELDS
        .iter()
        .filter(|field| field.role == Role::Names)
    {
        part(&mut stamp, &read_field(object, field)?);
    }
    Ok(stamp)
}

/// The dot of a written value: the stamp its author wrote it at, and its author.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Dot {
    pub wall_time: i64,
    pub counter: u64,
    pub node: u64,
}

/// What a part says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Part {
    /// A write: the field `field` of the row `id` given `value` and `text`, at `dot`, which is
    /// also the stamp of the change it was a part of.
    Write {
        id: u64,
        field: u64,
        value: i64,
        text: String,
        dot: Dot,
    },
    /// A write-over by `by`: the value of the field `field` of the row `id` written at `dot`
    /// taken out. `change` is the stamp of the change it was a part of, which a part of a change
    /// of more than one carries and one of a change of one does not.
    WriteOver {
        id: u64,
        field: u64,
        dot: Dot,
        by: u64,
        change: Option<(i64, u64)>,
    },
}

impl Part {
    /// The author and the stamp of the change it was a part of, when the part says it.
    pub fn change(&self) -> Option<(u64, i64, u64)> {
        match self {
            Part::Write { dot, .. } => Some((dot.node, dot.wall_time, dot.counter)),
            Part::WriteOver { by, change, .. } => {
                change.map(|(wall, counter)| (*by, wall, counter))
            }
        }
    }

    /// The part as an operation of a change (`record_op`): a write carries no dot of its own, its
    /// change's stamp being its dot (docs/adr/0085, section 1.1), and a write-over names the dot
    /// it takes out.
    fn op(&self) -> Value {
        match self {
            Part::Write {
                id,
                field,
                value,
                text,
                ..
            } => json!({"kind": 0, "id": id, "field": field, "value": value.to_string(),
                        "text": text, "wallTime": "0", "counter": 0, "nodeId": "0"}),
            Part::WriteOver { id, field, dot, .. } => json!({"kind": 1, "id": id, "field": field,
                        "value": "0", "text": "", "wallTime": dot.wall_time.to_string(),
                        "counter": dot.counter, "nodeId": dot.node.to_string()}),
        }
    }
}

/// What the part `event` with `data` says, its content as it is in `data` (opened, or never
/// sealed). Refused when `event` is no part, or a field is missing or not of its kind.
pub fn read(event: &str, data: &Value) -> Result<Part, MessageError> {
    let object = data.as_object().ok_or(MessageError::NotAnObject)?;
    let (fields, _) = fields_of(event).ok_or(MessageError::NotAnObject)?;
    let text = |name: &'static str| -> Result<String, MessageError> {
        let field = fields
            .iter()
            .find(|field| field.name == name)
            .ok_or(MessageError::Field(name))?;
        read_field(object, field)
    };
    let number = |name: &'static str| -> Result<u64, MessageError> {
        text(name)?.parse().map_err(|_| MessageError::Field(name))
    };
    let signed = |name: &'static str| -> Result<i64, MessageError> {
        text(name)?.parse().map_err(|_| MessageError::Field(name))
    };
    let dot = Dot {
        wall_time: signed("wallTime")?,
        counter: number("counter")?,
        node: number("nodeId")?,
    };
    let (id, field) = (number("id")?, number("field")?);
    if event == WRITE_EVENT {
        return Ok(Part::Write {
            id,
            field,
            value: signed("value")?,
            text: text("text")?,
            dot,
        });
    }
    let at = (signed("atWallTime")?, number("atCounter")?);
    Ok(Part::WriteOver {
        id,
        field,
        dot,
        by: number("by")?,
        change: (at.0 > 0).then_some(at),
    })
}

/// A value the parts held live: a write no write-over names.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Live {
    pub id: u64,
    pub field: u64,
    pub dot: Dot,
    pub value: i64,
    pub text: String,
}

/// What the parts a host holds make together (docs/adr/0085, section 8.3): the state of the list
/// they wrote, as a stable state a copy is given (`record_list.restore`, section 4.4).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Fold {
    /// The clock of the state: the highest stamp of a change any part was of.
    pub wall_time: i64,
    pub counter: u64,
    /// The live values, each with its own dot, by register and then by dot.
    pub values: Vec<Live>,
    /// For each device, the highest number of its chain among the parts folded, lowest node
    /// first: the cut the state stands at.
    pub cut: Vec<(u64, u64)>,
}

impl Fold {
    /// Whether the parts held no value live.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// The state as `record_list.restore` payloads (`record_restore`), each of at most `parts`
    /// values (the build's `shared_parts`; 1 when 0 is given) and each with the state's clock: a
    /// state of more values than one restore carries is given in several, and a copy takes each
    /// value in once whatever restore carries it. Empty when no value is live, since a restore
    /// holds at least one.
    pub fn restores(&self, parts: usize) -> Vec<String> {
        self.values
            .chunks(parts.max(1))
            .map(|values| {
                let ops: Vec<Value> = values
                    .iter()
                    .map(|live| {
                        json!({"kind": 0, "id": live.id, "field": live.field,
                               "value": live.value.to_string(), "text": live.text,
                               "wallTime": live.dot.wall_time.to_string(),
                               "counter": live.dot.counter,
                               "nodeId": live.dot.node.to_string()})
                    })
                    .collect();
                json!({"by": "0", "wallTime": self.wall_time.to_string(),
                       "counter": self.counter, "ops": ops})
                .to_string()
            })
            .collect()
    }
}

/// Fold `held`, each part with its author's node and its number in the author's chain, into the
/// state they make: the live values are the writes no write-over names. The observed-remove rule
/// is a function of the set (docs/adr/0085, section 8.3): a write-over removes the value it names
/// whether it is read before or after the write, so the parts are read in no order, and two hosts
/// that hold the same parts fold the same state. A write that names one register and one dot
/// with two contents is one its chain holds as proof and no host folds; of two such the greater
/// is kept, so that even then the fold does not hang on the order.
pub fn fold(held: impl IntoIterator<Item = (u64, u64, Part)>) -> Fold {
    let mut written: BTreeMap<(u64, u64, Dot), (i64, String)> = BTreeMap::new();
    let mut over: BTreeSet<(u64, u64, Dot)> = BTreeSet::new();
    let mut clock = (0i64, 0u64);
    let mut cut: BTreeMap<u64, u64> = BTreeMap::new();
    for (node, seq, part) in held {
        let top = cut.entry(node).or_insert(0);
        *top = (*top).max(seq);
        if let Some((_, wall, counter)) = part.change() {
            clock = clock.max((wall, counter));
        }
        match part {
            Part::Write {
                id,
                field,
                value,
                text,
                dot,
            } => {
                let register = (id, field, dot);
                let content = (value, text);
                if written.get(&register).is_none_or(|held| content > *held) {
                    written.insert(register, content);
                }
            }
            Part::WriteOver { id, field, dot, .. } => {
                over.insert((id, field, dot));
            }
        }
    }
    Fold {
        wall_time: clock.0,
        counter: clock.1,
        values: written
            .into_iter()
            .filter(|(register, _)| !over.contains(register))
            .map(|((id, field, dot), (value, text))| Live {
                id,
                field,
                dot,
                value,
                text,
            })
            .collect(),
        cut: cut.into_iter().collect(),
    }
}

/// The sends of parts a device saved that it had not signed, carried as changes
/// ([`changes_of_sends`]).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Carried {
    /// The changes, each a `record_list.change` payload, in the order their parts were sent.
    pub changes: Vec<String>,
    /// The sends no change can carry, each `(event, data, why)`, in the order they were sent.
    pub withdrawn: Vec<(String, String, String)>,
}

/// Carry the sends `sends` (each `(event, data)`, in the order the machine sent them) of parts a
/// device's machine made and the device had not signed, as the changes they were parts of: the
/// parts of one change were sent one after the other, write-overs first, and are taken together,
/// in their order, as one `record_list.change` under the change's author and stamp. A write
/// carries its change's stamp, and a write-over of a change of more than one carries it too; a
/// write-over sent as a change of one carries none, and no change's identity can be given it. It
/// is withdrawn, and so is every send after it, since each was made on a state that held it
/// (docs/adr/0093, section 6.2); a send that is no part, or that is not read as one, likewise.
pub fn changes_of_sends(sends: &[(String, String)]) -> Carried {
    let mut carried = Carried::default();
    let mut current: Option<Gathered> = None;
    for (at, (name, data)) in sends.iter().enumerate() {
        let read_as_part = serde_json::from_str::<Value>(data)
            .ok()
            .and_then(|value| read(name, &value).ok());
        let Some((change, part)) =
            read_as_part.and_then(|part| part.change().map(|change| (change, part)))
        else {
            let why = format!(
                "{name}: a part of a change whose stamp it does not carry, which no change of the third domain can carry"
            );
            carried.withdrawn.push((name.clone(), data.clone(), why));
            for (later, later_data) in &sends[at + 1..] {
                carried.withdrawn.push((
                    later.clone(),
                    later_data.clone(),
                    format!(
                        "{later}: a part this device made after one that could not be carried, and a change made on a state no other copy will hold is withdrawn with it"
                    ),
                ));
            }
            break;
        };
        let same_change = current.as_ref().is_some_and(|(open, _)| *open == change);
        if same_change {
            if let Some((_, ops)) = current.as_mut() {
                ops.push(part.op());
            }
        } else {
            close(current.take(), &mut carried);
            current = Some((change, vec![part.op()]));
        }
    }
    close(current.take(), &mut carried);
    carried
}

/// The change being gathered from consecutive parts: its author and stamp, and its operations.
type Gathered = ((u64, i64, u64), Vec<Value>);

/// The change `gathered` holds, when it holds one, carried.
fn close(gathered: Option<Gathered>, into: &mut Carried) {
    if let Some(((by, wall, counter), ops)) = gathered {
        into.changes.push(
            json!({"by": by.to_string(), "wallTime": wall.to_string(), "counter": counter,
                   "ops": ops})
            .to_string(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(id: u64, wall: i64, node: u64, value: i64) -> Part {
        Part::Write {
            id,
            field: 3,
            value,
            text: String::new(),
            dot: Dot {
                wall_time: wall,
                counter: 0,
                node,
            },
        }
    }

    fn write_over(id: u64, wall: i64, node: u64, by: u64, at: i64) -> Part {
        Part::WriteOver {
            id,
            field: 3,
            dot: Dot {
                wall_time: wall,
                counter: 0,
                node,
            },
            by,
            change: Some((at, 0)),
        }
    }

    /// A write-over removes the value it names whichever is read first, so every order of the
    /// same parts folds the same state; a value no write-over names stays live, and so do two
    /// concurrent writes of one register. Red if the fold reads the parts in order and lets a
    /// write read after its write-over come back.
    #[test]
    fn every_order_of_the_same_parts_folds_one_state() {
        let parts = vec![
            (1, 1, write(1, 100, 1, 10)),
            (2, 1, write(1, 110, 2, 20)),
            (1, 2, write(2, 120, 1, 30)),
            (2, 2, write_over(1, 100, 1, 2, 130)),
            (1, 3, write_over(1, 100, 1, 1, 140)),
        ];
        let forward = fold(parts.clone());
        let backward = fold(parts.iter().rev().cloned());
        assert_eq!(forward, backward);
        let live: Vec<(u64, i64)> = forward.values.iter().map(|v| (v.id, v.value)).collect();
        assert_eq!(live, vec![(1, 20), (2, 30)]);
        assert_eq!((forward.wall_time, forward.counter), (140, 0));
        assert_eq!(forward.cut, vec![(1, 3), (2, 2)]);
    }

    /// The state is given as restores of at most the build's bound of values each, every one with
    /// the state's clock and each value with its own dot.
    #[test]
    fn a_fold_is_given_as_restores_within_the_bound() {
        let state = fold((1..=5).map(|id| (1, id, write(id, 100 + id as i64, 1, 7))));
        let restores = state.restores(2);
        assert_eq!(restores.len(), 3);
        let first: Value = serde_json::from_str(&restores[0]).unwrap();
        assert_eq!(first["by"], json!("0"));
        assert_eq!(first["wallTime"], json!("105"));
        assert_eq!(first["ops"][1]["wallTime"], json!("102"));
        assert_eq!(first["ops"][1]["nodeId"], json!("1"));
        assert!(fold(Vec::new()).restores(64).is_empty());
    }

    /// The parts of one change are carried as one change, write-overs first as they were sent,
    /// under the change's stamp; a write-over sent alone, whose change's stamp was never sent, is
    /// withdrawn with every send after it.
    #[test]
    fn unsigned_parts_are_carried_as_the_changes_they_were_parts_of() {
        let over = json!({"id":1,"field":3,"wallTime":"100","counter":0,"nodeId":"2","by":"1",
                          "parts":2,"atWallTime":"200","atCounter":0});
        let written = json!({"id":1,"field":3,"value":"5","text":"tea","wallTime":"200",
                             "counter":0,"nodeId":"1","parts":2});
        let alone = json!({"id":4,"field":3,"value":"6","text":"","wallTime":"300","counter":0,
                           "nodeId":"1"});
        let lone_over = json!({"id":2,"field":3,"wallTime":"100","counter":0,"nodeId":"2",
                               "by":"1"});
        let sends: Vec<(String, String)> = [
            (SUPERSEDE_EVENT, &over),
            (WRITE_EVENT, &written),
            (WRITE_EVENT, &alone),
            (SUPERSEDE_EVENT, &lone_over),
            (WRITE_EVENT, &alone),
        ]
        .iter()
        .map(|(event, data)| (event.to_string(), data.to_string()))
        .collect();
        let carried = changes_of_sends(&sends);
        assert_eq!(carried.changes.len(), 2);
        let first: Value = serde_json::from_str(&carried.changes[0]).unwrap();
        assert_eq!(first["by"], json!("1"));
        assert_eq!(first["wallTime"], json!("200"));
        assert_eq!(first["ops"][0]["kind"], json!(1));
        assert_eq!(first["ops"][0]["nodeId"], json!("2"));
        assert_eq!(first["ops"][1]["kind"], json!(0));
        assert_eq!(first["ops"][1]["text"], json!("tea"));
        let second: Value = serde_json::from_str(&carried.changes[1]).unwrap();
        assert_eq!(second["wallTime"], json!("300"));
        assert_eq!(
            carried.withdrawn.len(),
            2,
            "the lone write-over and what follows"
        );
        assert_eq!(carried.withdrawn[0].0, SUPERSEDE_EVENT);
    }
}
