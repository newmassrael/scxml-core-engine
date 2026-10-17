// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The rule a host puts in front of its machine and behind it.
//!
//! Who may write the list is what the administrators' certificates say, and nothing a peer says
//! about itself: a write that arrives is admitted when its author, the node it names as the one
//! that stamped it, was a member whose role let them write at the time they made it, and the
//! signature it carries is true under the key their certificate of that time gave. A write that
//! leaves carries its author's signature: made here when the author is this host's own person,
//! and otherwise the one it arrived with, which is how a hub hands on what a member wrote
//! without being able to make it.
//!
//! A note that a write was written over (`todo.supersede`) is signed by the person who wrote
//! over it, with the time they made it at (`at`), and the host keeps who wrote over what
//! (the note's provenance): the machine sends a tombstone again on a resync as the copy that
//! holds it, and the host puts back on it who really wrote over it and the signature they made.
//! The signature and `at` are the host's: they are written into the event as it leaves and taken
//! out as it arrives, and a machine sees neither.
//!
//! The time is the author's own claim, and the window it leaves is closed by the administrator:
//! an instant they attest as settled ([`Trust::attest_floor`]) is one every copy refuses writes
//! at or before, so that what a member taken off writes in a time before the certificate that took
//! them off, after it was made, is refused once the instant it was taken off from is settled.
//!
//! One [`Trust`] is both halves of one host: it is cloned, and a clone is put in front of the
//! machine as its [`Admission`] and another behind it as its [`Sealer`].

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard};

use sce_rust_mesh::endpoint::{Admission, Sealer};
use serde_json::{json, Value};

use crate::certificate::Certificate;
use crate::encoding;
use crate::identity::{Identity, PublicKey};
use crate::message::{self, SIGNATURE_FIELD};

/// The role from which a member may write.
const WRITER: u8 = 2;

/// The field under which a note carries the time its author made it at.
const AT_FIELD: &str = "at";

/// Why a certificate was not taken in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrustError {
    /// It is for another list.
    WrongCalendar,
    /// Its issuer is not an administrator this host was told to trust.
    UnknownIssuer,
    /// Its issuer did not sign it, or it was edited after.
    BadSignature,
    /// The host holds a certificate for the node with this serial already.
    Stale { held: u64 },
}

/// What one certificate says of a member from its time on.
struct Era {
    from: i64,
    key: PublicKey,
    role: u8,
}

/// Who wrote over a write, when, and the signature they made of it.
#[derive(Clone, Copy)]
struct Note {
    by: u64,
    at: i64,
    signature: [u8; 64],
}

type Clock = Box<dyn Fn() -> i64 + Send>;

struct State {
    calendar: String,
    administrators: Vec<PublicKey>,
    /// Each member's certificates, by serial.
    members: BTreeMap<u64, BTreeMap<u64, Era>>,
    own: Option<(u64, Identity)>,
    /// The host's clock, the one it gives its machine as `now`: what a note this host's person
    /// makes is made at.
    clock: Option<Clock>,
    /// What each write that was admitted was signed with, by the bytes it is signed as: what a
    /// host that relays a write sends it on with. The floors an administrator attested are here.
    signatures: BTreeMap<Vec<u8>, [u8; 64]>,
    /// Who wrote over each write, by the note's subject.
    notes: BTreeMap<Vec<u8>, Note>,
    /// The events that arrive with no signature and make a copy forget, which this host does
    /// not take ([`UNSIGNED_REFUSED`], and [`Trust::allow_unsigned`]).
    unsigned_refused: Vec<String>,
}

impl State {
    /// What the certificates say of `node` at the time `at`: the one with the highest serial
    /// among those from a time at or before it. `None` when none applies.
    fn era_at(&self, node: u64, at: i64) -> Option<&Era> {
        self.members
            .get(&node)?
            .values()
            .rev()
            .find(|era| era.from <= at)
    }

    fn keys(&self) -> impl Iterator<Item = &PublicKey> {
        self.members
            .values()
            .flat_map(|eras| eras.values().map(|era| &era.key))
            .chain(self.administrators.iter())
    }
}

/// The events a host that trusts nobody refuses unless it was told to take them: `todo.floor`
/// is the word that an instant is settled, on which a copy forgets the tombstones up to it,
/// and a copy that forgot a tombstone takes a write it named in again. It carries no
/// signature, so a host that relays it could settle an instant its copies have not, and bring
/// back with the old signed writes what was deleted. A host that trusts the one that settles
/// (its own hub) allows it.
pub const UNSIGNED_REFUSED: [&str; 1] = ["todo.floor"];

/// The trust of one host in one list. Clones share it.
#[derive(Clone)]
pub struct Trust {
    state: Arc<Mutex<State>>,
}

impl Trust {
    /// A host of the list `calendar` that trusts the certificates `administrators` sign, and
    /// knows no member yet.
    pub fn new(calendar: &str, administrators: &[PublicKey]) -> Self {
        Self {
            state: Arc::new(Mutex::new(State {
                calendar: calendar.to_string(),
                administrators: administrators.to_vec(),
                members: BTreeMap::new(),
                own: None,
                clock: None,
                signatures: BTreeMap::new(),
                notes: BTreeMap::new(),
                unsigned_refused: UNSIGNED_REFUSED.iter().map(|e| e.to_string()).collect(),
            })),
        }
    }

    fn state(&self) -> MutexGuard<'_, State> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// This host's person: the node their writes are stamped with, and the key that signs them.
    /// A host that has none (a hub) signs nothing of its own.
    pub fn set_identity(&self, node: u64, identity: Identity) {
        self.state().own = Some((node, identity));
    }

    /// The host's clock, the one it gives its machine as the `now` of an edit: the time a note
    /// is made at when this host's person writes over a value. A host with an identity and no
    /// clock signs no note.
    pub fn set_clock(&self, clock: impl Fn() -> i64 + Send + 'static) {
        self.state().clock = Some(Box::new(clock));
    }

    /// An administrator's attestation that the changes up to `before` are settled with the hash
    /// `hash`, made here by the host that holds the administrator's key and kept for the hub's
    /// floor to leave with. The administrator's host reads the hash from its own copy, which
    /// is a member's, so that what it attests is what its own copy held.
    pub fn attest_floor(
        &self,
        administrator: &Identity,
        before: i64,
        hash: u32,
    ) -> Result<(), TrustError> {
        let mut state = self.state();
        if !state.administrators.contains(&administrator.public()) {
            return Err(TrustError::UnknownIssuer);
        }
        let message = message::floor_message(&state.calendar, before, hash);
        let signature = administrator.sign(&message);
        state.signatures.insert(message, signature);
        Ok(())
    }

    /// Take an attestation another host made, for the floor of a hub that sends it on: held
    /// when an administrator's key verifies it, and refused otherwise.
    pub fn hold_floor_signature(
        &self,
        before: i64,
        hash: u32,
        signature: [u8; 64],
    ) -> Result<(), TrustError> {
        let mut state = self.state();
        let message = message::floor_message(&state.calendar, before, hash);
        if !state
            .administrators
            .iter()
            .any(|administrator| administrator.verifies(&message, &signature))
        {
            return Err(TrustError::BadSignature);
        }
        state.signatures.insert(message, signature);
        Ok(())
    }

    /// Take `events` from a peer although they carry no signature, because this host trusts
    /// the peer that settles ([`UNSIGNED_REFUSED`]).
    pub fn allow_unsigned(&self, events: &[&str]) {
        self.state()
            .unsigned_refused
            .retain(|refused| !events.contains(&refused.as_str()));
    }

    /// Take a certificate in, when its issuer is trusted, it is signed, it is for this list,
    /// and the host holds none with its serial for the node. An older certificate than one held
    /// is taken in too: certificates make a history, and the same ones give the same answer
    /// whatever order they arrived in.
    pub fn accept(&self, certificate: &Certificate) -> Result<(), TrustError> {
        let mut state = self.state();
        if certificate.calendar != state.calendar {
            return Err(TrustError::WrongCalendar);
        }
        if !state.administrators.contains(&certificate.issuer) {
            return Err(TrustError::UnknownIssuer);
        }
        if !certificate.is_signed_by_its_issuer() {
            return Err(TrustError::BadSignature);
        }
        let eras = state.members.entry(certificate.node).or_default();
        if eras.contains_key(&certificate.serial) {
            return Err(TrustError::Stale {
                held: certificate.serial,
            });
        }
        eras.insert(
            certificate.serial,
            Era {
                from: certificate.from,
                key: certificate.key,
                role: certificate.role,
            },
        );
        Ok(())
    }

    /// The role of `node` now: its latest certificate's, 0 when it is not a member.
    pub fn role(&self, node: u64) -> u8 {
        self.state()
            .members
            .get(&node)
            .and_then(|eras| eras.values().next_back())
            .map_or(0, |era| era.role)
    }

    /// What tells a person's machine the roster the certificates make, as `(event, data)` for
    /// the host to raise: a `todo.role` for each node the host holds a certificate for, a member
    /// that was taken off included, so that a role the machine held is overwritten.
    pub fn role_events(&self) -> Vec<(String, String)> {
        let state = self.state();
        state
            .members
            .iter()
            .filter_map(|(node, eras)| Some((node, eras.values().next_back()?.role)))
            .map(|(node, role)| role_event(*node, role))
            .collect()
    }

    /// What tells the hub's machine the roster: as [`Self::role_events`], except that a member
    /// who could write at some time is a writer there still. The host judges each write by the
    /// certificate of the time it was made at, so what a member wrote before they were taken
    /// off is admitted by it, and the hub's machine, which has no time to judge by, must not
    /// refuse what the host let through.
    pub fn hub_role_events(&self) -> Vec<(String, String)> {
        let state = self.state();
        state
            .members
            .iter()
            .filter_map(|(node, eras)| {
                let latest = eras.values().next_back()?.role;
                let ever = eras.values().map(|era| era.role).max()?;
                Some((
                    node,
                    if ever >= WRITER {
                        ever.max(latest)
                    } else {
                        latest
                    },
                ))
            })
            .map(|(node, role)| role_event(*node, role))
            .collect()
    }

    /// The signatures this host holds for the writes it relays, to keep across a restart.
    pub fn signatures(&self) -> Vec<(Vec<u8>, [u8; 64])> {
        self.state()
            .signatures
            .iter()
            .map(|(message, signature)| (message.clone(), *signature))
            .collect()
    }

    /// What this host holds, as one text for its store (the key-value feature): a JSON object
    /// with the signatures of the writes and floors it relays, as `[message, signature]`, and
    /// who wrote over what, as `[subject, by, at, signature]`, each bytes written as
    /// [`encoding`] writes them.
    pub fn signatures_to_text(&self) -> String {
        let state = self.state();
        json!({
            "signatures": state.signatures.iter()
                .map(|(message, signature)| json!([encoding::encode(message), encoding::encode(signature)]))
                .collect::<Vec<_>>(),
            "notes": state.notes.iter()
                .map(|(subject, note)| json!([
                    encoding::encode(subject),
                    note.by.to_string(),
                    note.at.to_string(),
                    encoding::encode(&note.signature),
                ]))
                .collect::<Vec<_>>(),
        })
        .to_string()
    }

    /// Take back what [`Self::signatures_to_text`] wrote: each signature and each note that a
    /// key of today verifies is kept, and the rest are dropped, since what a host reads back
    /// from its own storage is not taken for true. Returns how many were kept.
    pub fn restore_signatures_from_text(&self, text: &str) -> usize {
        let Ok(value) = serde_json::from_str::<Value>(text) else {
            return 0;
        };
        let rows = |name: &str| {
            value
                .get(name)
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
        };
        let signatures: Vec<(Vec<u8>, [u8; 64])> = rows("signatures")
            .iter()
            .filter_map(|row| {
                let message = encoding::decode(row.get(0)?.as_str()?)?;
                let signature = encoding::decode(row.get(1)?.as_str()?)?.try_into().ok()?;
                Some((message, signature))
            })
            .collect();
        let mut kept = self.restore_signatures(&signatures);
        let mut state = self.state();
        for row in rows("notes") {
            let parsed = (|| {
                let subject = encoding::decode(row.get(0)?.as_str()?)?;
                let by: u64 = row.get(1)?.as_str()?.parse().ok()?;
                let at: i64 = row.get(2)?.as_str()?.parse().ok()?;
                let signature: [u8; 64] =
                    encoding::decode(row.get(3)?.as_str()?)?.try_into().ok()?;
                Some((subject, by, at, signature))
            })();
            let Some((subject, by, at, signature)) = parsed else {
                continue;
            };
            let signed = message::note_message(&state.calendar, &subject, by, at);
            let true_then = state
                .era_at(by, at)
                .is_some_and(|era| era.role >= WRITER && era.key.verifies(&signed, &signature));
            if true_then {
                state.notes.insert(subject, Note { by, at, signature });
                kept += 1;
            }
        }
        kept
    }

    /// Take back signatures [`Self::signatures`] gave: each one that a key of today (a
    /// member's, or an administrator's for a floor) verifies is kept, and the rest are dropped,
    /// since what a host reads back from its own storage is not taken for true. Returns how
    /// many were kept.
    pub fn restore_signatures(&self, entries: &[(Vec<u8>, [u8; 64])]) -> usize {
        let mut state = self.state();
        let mut kept = 0;
        for (message, signature) in entries {
            let true_of_a_key = state.keys().any(|key| key.verifies(message, signature));
            if true_of_a_key {
                state.signatures.insert(message.clone(), *signature);
                kept += 1;
            }
        }
        kept
    }
}

fn role_event(node: u64, role: u8) -> (String, String) {
    (
        "todo.role".to_string(),
        json!({"node": node.to_string(), "role": role}).to_string(),
    )
}

fn signature_of(value: &mut Value, event: &str) -> Result<[u8; 64], String> {
    value
        .as_object_mut()
        .and_then(|object| object.remove(SIGNATURE_FIELD))
        .ok_or_else(|| format!("{event}: it carries no signature"))?
        .as_str()
        .and_then(encoding::decode)
        .and_then(|bytes| bytes.try_into().ok())
        .ok_or_else(|| format!("{event}: its signature is not a signature"))
}

impl Admission for Trust {
    fn admit(&mut self, _peer: &str, event: &str, data: &str) -> Result<Option<String>, String> {
        let mut state = self.state();
        if event == message::FLOOR_EVENT {
            return admit_floor(&state, data);
        }
        if state
            .unsigned_refused
            .iter()
            .any(|refused| refused == event)
        {
            return Err(format!(
                "{event}: it carries no signature and this host does not take it on a peer's word"
            ));
        }
        if !message::SIGNED_EVENTS.contains(&event) {
            return Ok(None);
        }
        let mut value: Value =
            serde_json::from_str(data).map_err(|_| format!("{event}: the data is not JSON"))?;
        let signature = signature_of(&mut value, event)?;
        let signing = match message::signing(&state.calendar, event, &value) {
            Some(Ok(signing)) => signing,
            Some(Err(error)) => return Err(format!("{event}: {error:?}")),
            None => return Ok(None),
        };
        let era = state.era_at(signing.author, signing.at).ok_or_else(|| {
            format!(
                "{event}: node {} was not a member at {}",
                signing.author, signing.at
            )
        })?;
        if era.role < WRITER {
            return Err(format!(
                "{event}: node {} may not write (role {} at {})",
                signing.author, era.role, signing.at
            ));
        }
        if !era.key.verifies(&signing.message, &signature) {
            return Err(format!(
                "{event}: the signature is not node {}'s",
                signing.author
            ));
        }
        if event == "todo.supersede" {
            state.notes.entry(signing.subject).or_insert(Note {
                by: signing.author,
                at: signing.at,
                signature,
            });
            value.as_object_mut().map(|object| object.remove(AT_FIELD));
        } else {
            state.signatures.insert(signing.message, signature);
        }
        Ok(Some(value.to_string()))
    }
}

/// A `todo.floor` is admitted when an administrator attested it: its `sig` is an administrator
/// key's signature of the instant and the hash (`message::floor_message`). One with no
/// signature is admitted only when the host was told to take it unsigned.
fn admit_floor(state: &State, data: &str) -> Result<Option<String>, String> {
    let event = message::FLOOR_EVENT;
    let mut value: Value =
        serde_json::from_str(data).map_err(|_| format!("{event}: the data is not JSON"))?;
    let has_signature = value
        .as_object()
        .is_some_and(|object| object.contains_key(SIGNATURE_FIELD));
    if !has_signature {
        return if state.unsigned_refused.iter().any(|e| e == event) {
            Err(format!(
                "{event}: it carries no administrator's signature and this host does not take it on a peer's word"
            ))
        } else {
            Ok(None)
        };
    }
    let signature = signature_of(&mut value, event)?;
    let (before, hash) =
        message::floor_fields(&value).map_err(|error| format!("{event}: {error:?}"))?;
    let signed = message::floor_message(&state.calendar, before, hash);
    if state
        .administrators
        .iter()
        .any(|administrator| administrator.verifies(&signed, &signature))
    {
        Ok(Some(value.to_string()))
    } else {
        Err(format!("{event}: no administrator signed this instant"))
    }
}

/// The floor leaves with the signature an administrator made of it, when the host holds one.
fn seal_floor(state: &State, data: &str) -> Option<String> {
    let mut value: Value = serde_json::from_str(data).ok()?;
    let (before, hash) = message::floor_fields(&value).ok()?;
    let signature = state
        .signatures
        .get(&message::floor_message(&state.calendar, before, hash))?;
    value.as_object_mut()?.insert(
        SIGNATURE_FIELD.to_string(),
        Value::String(encoding::encode(signature)),
    );
    Some(value.to_string())
}

/// A note leaves as who really wrote over the write and when, with their signature, when the
/// host holds that; and as the host's own person's, made now, when it does not and the note
/// names that person: a copy that holds a tombstone and sends it again vouches for it.
fn seal_note(state: &mut State, data: &str) -> Option<String> {
    let mut value: Value = serde_json::from_str(data).ok()?;
    let object = value.as_object_mut()?;
    // The subject is the same whoever wrote over and whenever, so a placeholder reads it.
    object.insert(AT_FIELD.to_string(), Value::String("0".to_string()));
    let subject = message::signing(&state.calendar, "todo.supersede", &value)?
        .ok()?
        .subject;
    let (by, at, signature) = match state.notes.get(&subject) {
        Some(note) => (note.by, note.at, note.signature),
        None => {
            let (node, identity) = state.own.as_ref()?;
            let named = message::signing(&state.calendar, "todo.supersede", &value)?
                .ok()?
                .author;
            if named != *node {
                return None;
            }
            let at = (state.clock.as_ref()?)();
            let object = value.as_object_mut()?;
            object.insert(AT_FIELD.to_string(), Value::String(at.to_string()));
            let signing = message::signing(&state.calendar, "todo.supersede", &value)?.ok()?;
            let signature = identity.sign(&signing.message);
            let note = Note {
                by: *node,
                at,
                signature,
            };
            state.notes.insert(subject, note);
            (note.by, note.at, signature)
        }
    };
    let object = value.as_object_mut()?;
    object.insert("by".to_string(), Value::String(by.to_string()));
    object.insert(AT_FIELD.to_string(), Value::String(at.to_string()));
    object.insert(
        SIGNATURE_FIELD.to_string(),
        Value::String(encoding::encode(&signature)),
    );
    Some(value.to_string())
}

impl Sealer for Trust {
    fn seal(&mut self, _target: &str, event: &str, data: &str) -> Option<String> {
        let mut state = self.state();
        if event == message::FLOOR_EVENT {
            return seal_floor(&state, data);
        }
        if event == "todo.supersede" {
            return seal_note(&mut state, data);
        }
        if !message::SIGNED_EVENTS.contains(&event) {
            return None;
        }
        let mut value: Value = serde_json::from_str(data).ok()?;
        let signing = message::signing(&state.calendar, event, &value)?.ok()?;
        let signature = match &state.own {
            Some((node, identity)) if *node == signing.author => identity.sign(&signing.message),
            _ => *state.signatures.get(&signing.message)?,
        };
        value.as_object_mut()?.insert(
            SIGNATURE_FIELD.to_string(),
            Value::String(encoding::encode(&signature)),
        );
        Some(value.to_string())
    }
}
