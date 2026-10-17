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
use crate::secrecy;

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

/// Why a grant of a key was not taken in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GrantError {
    /// It is not what [`Trust::grant_key`] writes.
    Malformed,
    /// It is for another list.
    WrongCalendar,
    /// It is for another person, or this host has no person.
    NotForThisHost,
    /// It is from a key that is no member's who could write, and no administrator's.
    UnknownGiver,
    /// The secret the giver's key and this person's agree on does not open it.
    CannotOpen,
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

/// A write as it travels, and the signature its author made of it.
#[derive(Clone)]
struct Wire {
    data: String,
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
    /// The floors an administrator attested, by the bytes they are signed as: what the hub's
    /// floor leaves with.
    signatures: BTreeMap<Vec<u8>, [u8; 64]>,
    /// Each write that was admitted or made here, by the bytes it is signed as when the machine
    /// holds it: what the host that relays it sends it on as. It is the write as it travels
    /// (sealed, when the host keeps content from the hub) and its author's signature.
    wires: BTreeMap<Vec<u8>, Wire>,
    /// The keys of the epochs this host holds. A host that holds any keeps the content of every
    /// write from the hub, and takes no write that is not sealed.
    epoch_keys: BTreeMap<u32, secrecy::EpochKey>,
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
                wires: BTreeMap::new(),
                epoch_keys: BTreeMap::new(),
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

    /// This host holds the key of `epoch`: from now on it keeps the content of every write from
    /// the hub, seals what its person writes under the latest epoch it holds, and takes only
    /// writes that are sealed under an epoch it holds. For a host that makes a list's key
    /// (its owner's; the key is drawn from the host's source of randomness) or reads one it kept.
    pub fn set_epoch_key(&self, epoch: u32, key: secrecy::EpochKey) {
        self.state().epoch_keys.insert(epoch, key);
    }

    /// The epochs this host holds a key of.
    pub fn epochs(&self) -> Vec<u32> {
        self.state().epoch_keys.keys().copied().collect()
    }

    /// The key of `epoch` as a grant to `to`: sealed so that only `to`'s key opens it, as text
    /// for any channel to carry (the hub's included). `None` when this host has no identity or
    /// no key of the epoch, or `to` is not a point of the curve.
    pub fn grant_key(&self, epoch: u32, to: &PublicKey) -> Option<String> {
        let state = self.state();
        let (_, identity) = state.own.as_ref()?;
        let key = state.epoch_keys.get(&epoch)?;
        let from = identity.public();
        let shared = identity.agree(to)?;
        Some(
            json!({
                "calendar": state.calendar,
                "epoch": epoch,
                "from": from.to_text(),
                "to": to.to_text(),
                "wrapped": secrecy::wrap(&state.calendar, &shared, &from, to, epoch, key),
            })
            .to_string(),
        )
    }

    /// Take a key a member or an administrator gave this host's person: held when it is for
    /// this list and this person, from a key that belongs to the list's members or
    /// administrators, and opens. Gives its epoch.
    pub fn receive_grant(&self, text: &str) -> Result<u32, GrantError> {
        let mut state = self.state();
        let grant: Value = serde_json::from_str(text).map_err(|_| GrantError::Malformed)?;
        let text_of = |name: &str| grant.get(name).and_then(Value::as_str);
        if text_of("calendar") != Some(state.calendar.as_str()) {
            return Err(GrantError::WrongCalendar);
        }
        let epoch = u32::try_from(
            grant
                .get("epoch")
                .and_then(Value::as_u64)
                .ok_or(GrantError::Malformed)?,
        )
        .map_err(|_| GrantError::Malformed)?;
        let from = text_of("from")
            .and_then(PublicKey::from_text)
            .ok_or(GrantError::Malformed)?;
        let to = text_of("to")
            .and_then(PublicKey::from_text)
            .ok_or(GrantError::Malformed)?;
        let wrapped = text_of("wrapped").ok_or(GrantError::Malformed)?;
        let (_, identity) = state.own.as_ref().ok_or(GrantError::NotForThisHost)?;
        if identity.public() != to {
            return Err(GrantError::NotForThisHost);
        }
        let known = state.administrators.contains(&from)
            || state.members.values().any(|eras| {
                eras.values()
                    .any(|era| era.key == from && era.role >= WRITER)
            });
        if !known {
            return Err(GrantError::UnknownGiver);
        }
        let shared = identity.agree(&from).ok_or(GrantError::CannotOpen)?;
        let key = secrecy::unwrap(&state.calendar, &shared, &from, &to, epoch, wrapped)
            .ok_or(GrantError::CannotOpen)?;
        state.epoch_keys.insert(epoch, key);
        Ok(epoch)
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

    /// What this host holds, as one text for its store (the key-value feature): a JSON object
    /// with the floors an administrator attested, as `[message, signature]`, the writes it
    /// relays, as `[the write as it travels, signature]`, and who wrote over what, as
    /// `[subject, by, at, signature]`, each bytes written as [`encoding`] writes them. It holds
    /// no key: a host keeps those where it keeps its person's secret.
    pub fn signatures_to_text(&self) -> String {
        let state = self.state();
        json!({
            "signatures": state.signatures.iter()
                .map(|(message, signature)| json!([encoding::encode(message), encoding::encode(signature)]))
                .collect::<Vec<_>>(),
            "wires": state.wires.values()
                .map(|wire| json!([wire.data, encoding::encode(&wire.signature)]))
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
        for row in rows("wires") {
            let parsed = (|| {
                let data = row.get(0)?.as_str()?.to_string();
                let signature: [u8; 64] =
                    encoding::decode(row.get(1)?.as_str()?)?.try_into().ok()?;
                Some((data, signature))
            })();
            let Some((data, signature)) = parsed else {
                continue;
            };
            let Ok(wire) = serde_json::from_str::<Value>(&data) else {
                continue;
            };
            if let Ok((visible, _)) = verified_write(&state, &wire, &signature) {
                state.wires.insert(visible, Wire { data, signature });
                kept += 1;
            }
        }
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

    /// Take back the floors [`Self::signatures_to_text`] wrote: each one that an administrator's
    /// key verifies is kept, and the rest are dropped, since what a host reads back from its
    /// own storage is not taken for true. Returns how many were kept.
    fn restore_signatures(&self, entries: &[(Vec<u8>, [u8; 64])]) -> usize {
        let mut state = self.state();
        let mut kept = 0;
        for (message, signature) in entries {
            let true_of_a_key = state
                .administrators
                .iter()
                .any(|key| key.verifies(message, signature));
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
        if event == "todo.write" {
            let (visible_message, visible) = verified_write(&state, &value, &signature)?;
            state.wires.insert(
                visible_message,
                Wire {
                    data: value.to_string(),
                    signature,
                },
            );
            return Ok(Some(visible.to_string()));
        }
        let signing = match message::signing(&state.calendar, event, &value) {
            Some(Ok(signing)) => signing,
            Some(Err(error)) => return Err(format!("{event}: {error:?}")),
            None => return Ok(None),
        };
        check_author(&state, event, &signing, &signature)?;
        state.notes.entry(signing.subject).or_insert(Note {
            by: signing.author,
            at: signing.at,
            signature,
        });
        value.as_object_mut().map(|object| object.remove(AT_FIELD));
        Ok(Some(value.to_string()))
    }
}

/// Whether the author of what was signed was a member who could write at the time they say they
/// made it, and the signature is true under the key their certificate of that time gave.
fn check_author(
    state: &State,
    event: &str,
    signing: &message::Signing,
    signature: &[u8; 64],
) -> Result<(), String> {
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
    if !era.key.verifies(&signing.message, signature) {
        return Err(format!(
            "{event}: the signature is not node {}'s",
            signing.author
        ));
    }
    Ok(())
}

/// A write as it traveled (`wire`, without its signature) and the signature: judged as a write
/// is, and opened when this host keeps content from the hub. Gives the bytes the write is signed
/// as when the machine holds it, and the write as the machine is told it.
fn verified_write(
    state: &State,
    wire: &Value,
    signature: &[u8; 64],
) -> Result<(Vec<u8>, Value), String> {
    let event = "todo.write";
    let signing = match message::signing(&state.calendar, event, wire) {
        Some(Ok(signing)) => signing,
        Some(Err(error)) => return Err(format!("{event}: {error:?}")),
        None => return Err(format!("{event}: it is not a write")),
    };
    check_author(state, event, &signing, signature)?;
    let visible = open_write(state, wire)?;
    let held = message::signing(&state.calendar, event, &visible)
        .and_then(Result::ok)
        .ok_or_else(|| format!("{event}: what it holds is not a write"))?;
    Ok((held.message, visible))
}

/// The write as the machine is told it: the content opened, when this host keeps content from
/// the hub, and the write as it is otherwise.
fn open_write(state: &State, wire: &Value) -> Result<Value, String> {
    if state.epoch_keys.is_empty() {
        return Ok(wire.clone());
    }
    let sealed = wire.get("text").and_then(Value::as_str).unwrap_or("");
    let stamp = message::write_stamp(wire).map_err(|error| format!("todo.write: {error:?}"))?;
    let (value, text) =
        secrecy::open(&state.calendar, &state.epoch_keys, &stamp, sealed).map_err(|error| {
            match error {
            secrecy::OpenError::NotSealed => {
                "todo.write: its content is not sealed, and this host keeps content from the hub"
                    .to_string()
            }
            secrecy::OpenError::NoKey { epoch } => {
                format!("todo.write: this host holds no key of epoch {epoch}")
            }
            secrecy::OpenError::Corrupt => "todo.write: its content does not open".to_string(),
        }
        })?;
    let mut visible = wire.clone();
    let object = visible
        .as_object_mut()
        .ok_or("todo.write: the data is not an object")?;
    object.insert("value".to_string(), Value::String(value.to_string()));
    object.insert("text".to_string(), Value::String(text));
    Ok(visible)
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
        seal_write(&mut state, data)
    }
}

/// A write leaves as it traveled to this host, with its author's signature, when it is another
/// person's; and as this host's person's own, sealed when the host keeps content from the hub,
/// and signed here, when it is theirs.
fn seal_write(state: &mut State, data: &str) -> Option<String> {
    let visible: Value = serde_json::from_str(data).ok()?;
    let signing = message::signing(&state.calendar, "todo.write", &visible)?.ok()?;
    let held = signing.message;
    let own = matches!(&state.own, Some((node, _)) if *node == signing.author);
    // What was made or admitted before leaves as it did: one write is one set of bytes.
    if let Some(wire) = state.wires.get(&held).cloned() {
        let mut value: Value = serde_json::from_str(&wire.data).ok()?;
        value.as_object_mut()?.insert(
            SIGNATURE_FIELD.to_string(),
            Value::String(encoding::encode(&wire.signature)),
        );
        return Some(value.to_string());
    }
    if !own {
        return None;
    }
    let mut wire = visible.clone();
    if let Some(epoch) = state.epoch_keys.keys().next_back().copied() {
        let key = state.epoch_keys.get(&epoch)?;
        let (number, text) = message::write_content(&visible).ok()?;
        let stamp = message::write_stamp(&visible).ok()?;
        let object = wire.as_object_mut()?;
        object.insert("value".to_string(), Value::String("0".to_string()));
        object.insert(
            "text".to_string(),
            Value::String(secrecy::seal(
                &state.calendar,
                epoch,
                key,
                &stamp,
                number,
                &text,
            )),
        );
    }
    let travelling = message::signing(&state.calendar, "todo.write", &wire)?.ok()?;
    let (_, identity) = state.own.as_ref()?;
    let signature = identity.sign(&travelling.message);
    state.wires.insert(
        held,
        Wire {
            data: wire.to_string(),
            signature,
        },
    );
    wire.as_object_mut()?.insert(
        SIGNATURE_FIELD.to_string(),
        Value::String(encoding::encode(&signature)),
    );
    Some(wire.to_string())
}
