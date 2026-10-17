// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The rule a host puts in front of its machine and behind it.
//!
//! Who may write the list is what the membership log says (`crate::membership`), and nothing a
//! peer says about itself: a write that arrives is admitted when its author, the node it names as
//! the one that stamped it, was a member whose role let them write at the time they made it, and
//! the signature it carries is true under the key the log gave them for that time. A write that
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
//! at or before, so that what a member taken off writes in a time before the entry that took them
//! off, after it was made, is refused once the instant it was taken off from is settled.
//!
//! One [`Trust`] is both halves of one host: it is cloned, and a clone is put in front of the
//! machine as its [`Admission`] and another behind it as its [`Sealer`].

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard};

use sce_rust_mesh::endpoint::{Admission, Sealer};
use serde_json::{json, Value};

use crate::encoding;
use crate::identity::{Identity, PublicKey};
use crate::membership::{Body, Draft, Entry, Hash, Log, LogError, Profile, WRITER};
use crate::message::{self, Link, SIGNATURE_FIELD};
use crate::secrecy;

/// Why an attestation was not taken in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrustError {
    /// The key is no administrator of the list's membership log.
    UnknownIssuer,
    /// No administrator signed it.
    BadSignature,
}

/// A new epoch begun by an administrator: the entry of the membership log that begins it, and the
/// grant of its key for each recipient.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rotation {
    pub epoch: u32,
    /// The entry that was appended to the administrator's log, for every other host to append.
    pub entry: Entry,
    /// Each recipient's key and the grant (text) for that member's host, to carry by any channel.
    pub grants: Vec<(PublicKey, String)>,
}

/// Why an epoch could not be begun.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RotateError {
    /// The key that asked is no administrator this host was told to trust.
    NotAnAdministrator,
    /// The operating system gave no randomness, and no weaker source is used in its place.
    NoRandomness,
    /// Every epoch number is used.
    NoMoreEpochs,
    /// A recipient is not a point of the curve a key can be agreed on.
    BadRecipient(PublicKey),
    /// The host holds a key of the next epoch already: the log it holds is behind the keys it
    /// has, and it has not been given the entry that begins it.
    EpochHeld,
    /// The log did not take the entry that begins the epoch.
    LogRefused,
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
    /// The host holds another key of that epoch already.
    EpochTaken,
}

/// An operation of a device as it travels: the event as its author made it (sealed, when the list
/// keeps content from the hub), where it stands in the device's chain, and the signature the
/// device made of all of it.
#[derive(Clone)]
struct Wire {
    data: String,
    link: Link,
    signature: [u8; 64],
}

impl Wire {
    /// The data of the event as it goes on the wire: the event, its link and its signature.
    fn to_json(&self) -> Option<String> {
        let mut value: Value = serde_json::from_str(&self.data).ok()?;
        self.link.write_into(&mut value)?;
        value.as_object_mut()?.insert(
            SIGNATURE_FIELD.to_string(),
            Value::String(encoding::encode(&self.signature)),
        );
        Some(value.to_string())
    }
}

/// What a host remembers of an operation of a device that it admitted: its id, the one it
/// names before it, and, for a write, its stamp, so that a chain can be checked whatever order its
/// operations arrive in.
#[derive(Clone, Copy)]
struct Held {
    id: [u8; 32],
    prev: [u8; 32],
    stamp: Option<(i64, u64)>,
}

struct State {
    calendar: String,
    /// Who the administrators and members are, from when, and the epoch: the chain of entries the
    /// host holds and has verified.
    log: Log,
    /// Where the log forked, when the host was shown two histories: nothing is taken in until an
    /// administrator resolves it.
    forked: Option<u64>,
    own: Option<(u64, Identity)>,
    /// The floors an administrator attested, by the bytes they are signed as: what the hub's
    /// floor leaves with.
    signatures: BTreeMap<Vec<u8>, [u8; 64]>,
    /// Each write that was admitted or made here, by the bytes it is signed as when the machine
    /// holds it: what the host that relays it sends it on as. It is the operation as it travels.
    wires: BTreeMap<Vec<u8>, Wire>,
    /// Who wrote over each write, by the note's subject: the operation of the person who did, as
    /// it travels, so that a tombstone the machine sends again is sent as theirs.
    notes: BTreeMap<Vec<u8>, Wire>,
    /// The operations admitted from each device, by their number: what the chains are checked
    /// against, in whatever order the operations came.
    ops: BTreeMap<u64, BTreeMap<u64, Held>>,
    /// The devices that signed two different operations at one number, and the number from which
    /// this host takes nothing of theirs until an administrator removes them.
    frozen: BTreeMap<u64, u64>,
    /// The keys of the epochs this host holds. A host that holds any keeps the content of every
    /// write from the hub, and takes no write that is not sealed.
    epoch_keys: BTreeMap<u32, secrecy::EpochKey>,
    /// The events that arrive with no signature and make a copy forget, which this host does
    /// not take ([`UNSIGNED_REFUSED`], and [`Trust::allow_unsigned`]).
    unsigned_refused: Vec<String>,
}

impl State {
    /// Whether the list keeps the content of writes from the hub: from the entry of the log that
    /// begins its first epoch. What a host holds of keys decides what it can open, and the log
    /// alone decides that the content is sealed.
    fn keeps_content(&self) -> bool {
        self.log.epoch() > 0
    }

    /// Whether this host is a hub that holds no key: it checks that content is sealed, under an
    /// epoch the log has begun, and hands it on as it is. A person's host that has no key yet is
    /// not one: it opens what it takes in, or says it holds no key.
    fn relays_sealed(&self) -> bool {
        self.own.is_none() && self.epoch_keys.is_empty()
    }

    /// Whether `event` is refused when it arrives with no signature: by this host's own setting,
    /// and never when the log says the hub is trusted.
    fn refuses_unsigned(&self, event: &str) -> bool {
        self.log.profile() == Profile::Untrusted
            && self.unsigned_refused.iter().any(|refused| refused == event)
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
    /// The host of whoever makes the list `calendar`: it holds an empty log, takes the genesis it
    /// is first given (the one its owner made), and every entry after it from there.
    pub fn open(calendar: &str) -> Self {
        Self::with_log(calendar, Log::open(calendar))
    }

    /// The host of a person invited to the list `calendar` with the hash of its genesis entry,
    /// which they were told in a channel they trust: it takes that genesis and no other, and the
    /// rest of the log is checked from it.
    pub fn invited(calendar: &str, genesis: Hash) -> Self {
        Self::with_log(calendar, Log::invited(calendar, genesis))
    }

    fn with_log(calendar: &str, log: Log) -> Self {
        Self {
            state: Arc::new(Mutex::new(State {
                calendar: calendar.to_string(),
                log,
                forked: None,
                own: None,
                signatures: BTreeMap::new(),
                wires: BTreeMap::new(),
                notes: BTreeMap::new(),
                ops: BTreeMap::new(),
                frozen: BTreeMap::new(),
                epoch_keys: BTreeMap::new(),
                unsigned_refused: UNSIGNED_REFUSED.iter().map(|e| e.to_string()).collect(),
            })),
        }
    }

    fn state(&self) -> MutexGuard<'_, State> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Take an entry of the membership log in: the genesis, or the one that follows the head and is
    /// signed by the administrators it needs. `Ok(false)` when it is one already held. Two
    /// different entries at one position are a fork, which freezes the host: it takes no write,
    /// note or settled instant until an administrator resolves it ([`Self::forked`]).
    pub fn append(&self, entry: &Entry) -> Result<bool, LogError> {
        let mut state = self.state();
        let appended = state.log.append(entry);
        if let Err(LogError::Fork { index }) = &appended {
            state.forked.get_or_insert(*index);
        }
        appended
    }

    /// The position where the log forked, when this host was shown two histories of it.
    pub fn forked(&self) -> Option<u64> {
        self.state().forked
    }

    /// The draft of the entry that follows the head of this host's log, for administrators to sign
    /// and for the host to [`Self::append`]. A list's first entry is made with
    /// [`Log::genesis_draft`].
    pub fn draft(&self, body: Body) -> Draft {
        self.state().log.draft(body)
    }

    /// The position the next entry takes and the hash it names: what two hosts compare to know
    /// whether they hold the same log.
    pub fn head(&self) -> (u64, Hash) {
        self.state().log.head()
    }

    /// The entries this host holds, in order, to give a host that is behind.
    pub fn entries(&self) -> Vec<Entry> {
        self.state().log.entries().to_vec()
    }

    /// Whether the hub of the list is trusted, as the log says.
    pub fn profile(&self) -> Profile {
        self.state().log.profile()
    }

    /// The latest epoch the log begins, 0 when none.
    pub fn epoch(&self) -> u32 {
        self.state().log.epoch()
    }

    /// The administrators the log has now.
    pub fn administrators(&self) -> Vec<PublicKey> {
        self.state().log.administrators().to_vec()
    }

    /// This host's person: the node their writes are stamped with, and the key that signs them.
    /// A host that has none (a hub) signs nothing of its own.
    pub fn set_identity(&self, node: u64, identity: Identity) {
        self.state().own = Some((node, identity));
    }

    /// The number of the last operation this host has admitted from the device of `node`, 0 when
    /// it has none. What an administrator reads to say where a removed device's chain ends
    /// (`Body::Grant::final_seq`): from this host, and from the checkpoints of the others.
    pub fn last_seq(&self, node: u64) -> u64 {
        self.state()
            .ops
            .get(&node)
            .and_then(|ops| ops.keys().next_back().copied())
            .unwrap_or(0)
    }

    /// The devices this host has caught signing two different operations at one number, and the
    /// number: proof, held by any host that saw both, that the device's chain forked.
    pub fn equivocating(&self) -> Vec<(u64, u64)> {
        self.state()
            .frozen
            .iter()
            .map(|(node, seq)| (*node, *seq))
            .collect()
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
        if !state.log.administrators().contains(&administrator.public()) {
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
            .log
            .administrators()
            .iter()
            .any(|administrator| administrator.verifies(&message, &signature))
        {
            return Err(TrustError::BadSignature);
        }
        state.signatures.insert(message, signature);
        Ok(())
    }

    /// This host holds the key of `epoch`, which it kept from an earlier run or was given outside a
    /// grant. Whether the list keeps its content from the hub, and which epoch a write is sealed
    /// under, is the membership log's: a key opens what is sealed under its epoch, and a host
    /// holding none that it needs says so and takes or sends nothing in the clear.
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

    /// The members the log makes of the list now, as the node and the key of each: every node whose
    /// latest grant gives a role of 1 or more. What a rotation is given to when a member was taken
    /// off: the ones left.
    pub fn current_members(&self) -> Vec<(u64, PublicKey)> {
        self.state()
            .log
            .latest()
            .filter_map(|(node, era)| (era.role >= 1).then_some((node, era.key)))
            .collect()
    }

    /// Begin a new epoch of the list, made by an administrator's host: an entry of the membership
    /// log that begins it (appended here and returned for every other host to append), a fresh
    /// key (drawn here, from the operating system), and a grant for each of `recipients`, which
    /// is the keys of the members that are left when one was taken off. A member who is not among
    /// them is given nothing, derives nothing from what they were given before (the key is not
    /// made from the old one), and so opens none of what is sealed from here on. What was sealed
    /// before stays what it was, to those who hold those keys.
    ///
    /// Nothing here makes the removed member's older keys stop opening older content, which no
    /// scheme can; it makes sure that nothing written afterwards opens for them, and that a host
    /// that has not been given the new key writes nothing until it is, and not under the old one.
    pub fn rotate_epoch(
        &self,
        administrator: &Identity,
        recipients: &[PublicKey],
    ) -> Result<Rotation, RotateError> {
        let mut state = self.state();
        if !state.log.administrators().contains(&administrator.public()) {
            return Err(RotateError::NotAnAdministrator);
        }
        let epoch = state
            .log
            .epoch()
            .checked_add(1)
            .ok_or(RotateError::NoMoreEpochs)?;
        if state.epoch_keys.contains_key(&epoch) {
            return Err(RotateError::EpochHeld);
        }
        let key = secrecy::generate_key().map_err(|_| RotateError::NoRandomness)?;
        let entry = state
            .log
            .draft(Body::Epoch { epoch })
            .sign(administrator)
            .finish();
        let from = administrator.public();
        let mut grants = Vec::with_capacity(recipients.len());
        for to in recipients {
            let shared = administrator
                .agree(to)
                .ok_or(RotateError::BadRecipient(*to))?;
            grants.push((
                *to,
                json!({
                    "calendar": state.calendar,
                    "epoch": epoch,
                    "from": from.to_text(),
                    "to": to.to_text(),
                    "wrapped": secrecy::wrap(&state.calendar, &shared, &from, to, epoch, &key),
                })
                .to_string(),
            ));
        }
        state
            .log
            .append(&entry)
            .map_err(|_| RotateError::LogRefused)?;
        state.epoch_keys.insert(epoch, key);
        Ok(Rotation {
            epoch,
            entry,
            grants,
        })
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
        let known = state.log.administrators().contains(&from) || state.log.was_a_writer(&from);
        if !known {
            return Err(GrantError::UnknownGiver);
        }
        let shared = identity.agree(&from).ok_or(GrantError::CannotOpen)?;
        let key = secrecy::unwrap(&state.calendar, &shared, &from, &to, epoch, wrapped)
            .ok_or(GrantError::CannotOpen)?;
        // A key of an epoch the host holds is not replaced by another: two keys for one epoch is a
        // host that was given a number someone else already used, and what it sealed under the
        // first would not open for those who hold the second.
        match state.epoch_keys.get(&epoch) {
            Some(held) if *held != key => return Err(GrantError::EpochTaken),
            _ => {}
        }
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

    /// The role of `node` now: its latest grant's, 0 when it is not a member.
    pub fn role(&self, node: u64) -> u8 {
        self.state()
            .log
            .latest()
            .find(|(held, _)| *held == node)
            .map_or(0, |(_, era)| era.role)
    }

    /// What tells a person's machine the roster the log makes, as `(event, data)` for the host to
    /// raise: a `todo.role` for each node the log has a grant for, a member that was taken off
    /// included, so that a role the machine held is overwritten.
    pub fn role_events(&self) -> Vec<(String, String)> {
        let state = self.state();
        state
            .log
            .latest()
            .map(|(node, era)| role_event(node, era.role))
            .collect()
    }

    /// What tells the hub's machine the roster: as [`Self::role_events`], except that a member
    /// who could write at some time is a writer there still. The host judges each write by the
    /// grant of the time it was made at, so what a member wrote before they were taken off is
    /// admitted by it, and the hub's machine, which has no time to judge by, must not refuse what
    /// the host let through.
    pub fn hub_role_events(&self) -> Vec<(String, String)> {
        let state = self.state();
        state
            .log
            .eras()
            .filter_map(|(node, eras)| {
                let latest = eras.last()?.role;
                let ever = eras.iter().map(|era| era.role).max()?;
                Some(role_event(
                    node,
                    if ever >= WRITER {
                        ever.max(latest)
                    } else {
                        latest
                    },
                ))
            })
            .collect()
    }

    /// What this host holds, as one text for its store (the key-value feature): a JSON object
    /// with the floors an administrator attested, as `[message, signature]`, and the operations it
    /// holds (the writes it relays and the notes of who wrote over what), each as the data it goes
    /// on the wire with. It holds no key: a host keeps those where it keeps its person's secret.
    pub fn signatures_to_text(&self) -> String {
        let state = self.state();
        json!({
            "signatures": state.signatures.iter()
                .map(|(message, signature)| json!([encoding::encode(message), encoding::encode(signature)]))
                .collect::<Vec<_>>(),
            "wires": state.wires.values().filter_map(Wire::to_json).collect::<Vec<_>>(),
            "notes": state.notes.values().filter_map(Wire::to_json).collect::<Vec<_>>(),
        })
        .to_string()
    }

    /// Take back what [`Self::signatures_to_text`] wrote: each floor and each operation that
    /// today's log verifies is kept, and the rest are dropped, since what a host reads back from
    /// its own storage is not taken for true. The chain of this host's own person is what was
    /// kept of it, so that what it writes next is the next one. Returns how many were kept.
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
        for (name, event) in [("wires", "todo.write"), ("notes", "todo.supersede")] {
            for row in rows(name) {
                let Some(text) = row.as_str() else { continue };
                let Ok(mut value) = serde_json::from_str::<Value>(text) else {
                    continue;
                };
                let Ok(signature) = signature_of(&mut value, event) else {
                    continue;
                };
                if let Ok(verified) = verify_operation(&state, event, value, &signature) {
                    record(&mut state, event, verified, signature);
                    kept += 1;
                }
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
                .log
                .administrators()
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
        if let Some(index) = state.forked {
            if event == message::FLOOR_EVENT || message::SIGNED_EVENTS.contains(&event) {
                return Err(format!(
                    "{event}: the membership log forked at {index}, and nothing is taken in until an administrator resolves it"
                ));
            }
        }
        if event == message::FLOOR_EVENT {
            return admit_floor(&state, data);
        }
        if state.refuses_unsigned(event) {
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
        match verify_operation(&state, event, value, &signature) {
            Ok(verified) => {
                let visible = verified.visible.to_string();
                record(&mut state, event, verified, signature);
                Ok(Some(visible))
            }
            Err(Refusal::Refused(reason)) => Err(reason),
            Err(Refusal::Equivocation { node, seq, reason }) => {
                let from = state.frozen.get(&node).map_or(seq, |held| (*held).min(seq));
                state.frozen.insert(node, from);
                Err(reason)
            }
        }
    }
}

/// Why an operation was not taken in.
enum Refusal {
    /// It does not hold: its reason.
    Refused(String),
    /// The device signed two different operations at one number: proof that its chain forked, which
    /// freezes the device (`State::frozen`) from `seq`.
    Equivocation { node: u64, seq: u64, reason: String },
}

impl From<String> for Refusal {
    fn from(reason: String) -> Self {
        Refusal::Refused(reason)
    }
}

/// An operation that holds: who made it, where it stands, and the event as its author made it
/// (`travelling`, sealed when the list keeps content) and as the machine is told it (`visible`).
struct Verified {
    author: u64,
    link: Link,
    id: [u8; 32],
    stamp: Option<(i64, u64)>,
    travelling: Value,
    visible: Value,
    /// What the host keeps it under: the bytes of the write the machine holds, or the subject of a
    /// note.
    key: Vec<u8>,
}

/// Whether an operation holds, as `event` with its link in the data and without its signature:
/// the entry of the log it names is the one this host holds, its author was a writer under that
/// entry, the administrators accepted the device's operations up to this number, the signature is
/// the key's of that entry, it stands consistently among the device's operations this host holds
/// (whatever order they came in), and its content opens. Nothing here reads a clock: the entry it
/// names and the number it has are what are signed, and what a device claims of time decides
/// nothing.
fn verify_operation(
    state: &State,
    event: &str,
    mut value: Value,
    signature: &[u8; 64],
) -> Result<Verified, Refusal> {
    let link = Link::take_from(&mut value).map_err(|why| format!("{event}: {why}"))?;
    let signing = match message::signing(&state.calendar, event, &value) {
        Some(Ok(signing)) => signing,
        Some(Err(error)) => return Err(format!("{event}: {error:?}").into()),
        None => return Err(format!("{event}: it is not an event that is signed").into()),
    };
    let author = signing.author;
    let bytes = message::operation_bytes(&signing.message, &link);
    let id = message::operation_id(&bytes);

    if state.log.hash_at(link.log) != Some(link.log_hash) {
        return Err(format!(
            "{event}: it names entry {} of the membership log, which this host does not hold as it is",
            link.log
        )
        .into());
    }
    let era = state.log.era_for(author, link.log).ok_or_else(|| {
        format!(
            "{event}: node {author} was not a member at entry {}",
            link.log
        )
    })?;
    if era.role < WRITER {
        return Err(format!(
            "{event}: node {author} may not write (role {} at entry {})",
            era.role, link.log
        )
        .into());
    }
    if let Some(cutoff) = state.log.cutoff_after(author, era) {
        if link.seq > cutoff {
            return Err(format!(
                "{event}: operation {} of node {author} is past {cutoff}, the last the administrators accepted of it",
                link.seq
            )
            .into());
        }
    }
    if !era.key.verifies(&bytes, signature) {
        return Err(format!("{event}: the signature is not node {author}'s").into());
    }
    if link.seq == 0 {
        return Err(format!("{event}: a chain begins at 1").into());
    }
    let floor = state.log.floor_before(author, era);
    if link.seq <= floor {
        return Err(format!(
            "{event}: number {} of node {author} belongs to the chain an earlier grant ended at {floor}",
            link.seq
        )
        .into());
    }
    // A chain begins at 1, and a new one (another key, or the device made a writer again) at the
    // number after the end an earlier grant set: its first operation names no operation before
    // it, and the checks of what follows and precedes stop at the edge.
    let cutoffs = state.log.cutoffs(author);
    let segment_start = cutoffs
        .iter()
        .rev()
        .find(|cutoff| **cutoff < link.seq)
        .map_or(1, |cutoff| cutoff + 1);
    let segment_end = cutoffs
        .iter()
        .find(|cutoff| **cutoff >= link.seq)
        .copied()
        .unwrap_or(u64::MAX);
    let begins = link.seq == segment_start;
    if begins && link.prev != message::NO_OPERATION {
        return Err(format!("{event}: it begins a chain and names an operation before it").into());
    }
    if let Some(from) = state.frozen.get(&author) {
        if link.seq >= *from {
            return Err(format!(
                "{event}: node {author} signed two operations at {from}, and this host takes nothing of theirs from there"
            )
            .into());
        }
    }

    let held = state.ops.get(&author);
    let equivocation = |seq: u64, what: &str| Refusal::Equivocation {
        node: author,
        seq,
        reason: format!("{event}: node {author} signed two different operations at {seq} ({what})"),
    };
    if let Some(held) = held {
        if let Some(same) = held.get(&link.seq) {
            if same.id != id {
                return Err(equivocation(link.seq, "this and the one held"));
            }
        }
        if !begins {
            if let Some(before) = held.get(&(link.seq - 1)) {
                if before.id != link.prev {
                    return Err(equivocation(
                        link.seq - 1,
                        "the one held and the one this names before it",
                    ));
                }
            }
        }
        if link.seq < segment_end {
            if let Some(after) = held.get(&(link.seq + 1)) {
                if after.prev != id {
                    return Err(equivocation(
                        link.seq,
                        "this and the one the next names before it",
                    ));
                }
            }
        }
    }

    // A write's stamp is its author's clock when it wrote it, and along one chain it does not go
    // back: a device cannot give a later write an earlier time than one it had already made.
    let stamp = if event == "todo.write" {
        Some(message::stamp_of(&value).map_err(|error| format!("{event}: {error:?}"))?)
    } else {
        None
    };
    if let (Some(stamp), Some(held)) = (stamp, held) {
        let before = held
            .range(segment_start..link.seq)
            .rev()
            .find_map(|(_, other)| other.stamp);
        let after = if link.seq < segment_end {
            held.range(link.seq + 1..=segment_end)
                .find_map(|(_, other)| other.stamp)
        } else {
            None
        };
        if before.is_some_and(|earlier| stamp < earlier) || after.is_some_and(|later| stamp > later)
        {
            return Err(format!(
                "{event}: its stamp goes back along node {author}'s chain at {}",
                link.seq
            )
            .into());
        }
    }

    let visible = if event == "todo.write" {
        open_write(state, &value)?
    } else {
        value.clone()
    };
    let key = if event == "todo.write" {
        message::signing(&state.calendar, event, &visible)
            .and_then(Result::ok)
            .ok_or_else(|| format!("{event}: what it holds is not a write"))?
            .message
    } else {
        signing.subject
    };
    Ok(Verified {
        author,
        link,
        id,
        stamp,
        travelling: value,
        visible,
        key,
    })
}

/// Keep an operation that holds: among its device's, and under the key the host finds it by.
fn record(state: &mut State, event: &str, verified: Verified, signature: [u8; 64]) {
    state.ops.entry(verified.author).or_default().insert(
        verified.link.seq,
        Held {
            id: verified.id,
            prev: verified.link.prev,
            stamp: verified.stamp,
        },
    );
    let wire = Wire {
        data: verified.travelling.to_string(),
        link: verified.link,
        signature,
    };
    let table = if event == "todo.write" {
        &mut state.wires
    } else {
        &mut state.notes
    };
    table.entry(verified.key).or_insert(wire);
}

/// The write as the machine is told it: the content opened, when this host keeps content from
/// the hub, and the write as it is otherwise.
fn open_write(state: &State, wire: &Value) -> Result<Value, String> {
    if !state.keeps_content() {
        return Ok(wire.clone());
    }
    let sealed = wire.get("text").and_then(Value::as_str).unwrap_or("");
    let Some(epoch) = secrecy::epoch_of(sealed) else {
        return Err(
            "todo.write: its content is not sealed, and the list keeps content from the hub"
                .to_string(),
        );
    };
    if epoch == 0 || epoch > state.log.epoch() {
        return Err(format!(
            "todo.write: it is sealed under epoch {epoch}, which the membership log has not begun"
        ));
    }
    if state.relays_sealed() {
        return Ok(wire.clone());
    }
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
        return if state.refuses_unsigned(event) {
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
        .log
        .administrators()
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

/// Make the next operation of this host's person's chain out of `travelling`, the event as it
/// goes on the wire: its number and the one it follows, the entry of the log it is made under, and
/// the person's signature of all of it. Kept among the operations the host holds, so that the one
/// after it follows it.
fn make_operation(
    state: &mut State,
    event: &str,
    travelling: Value,
    author: u64,
) -> Result<Wire, String> {
    let (node, identity) = state
        .own
        .as_ref()
        .ok_or_else(|| format!("{event}: this host has no person"))?;
    if *node != author {
        return Err(format!(
            "{event}: this host's person is node {node}, and it signs none in node {author}'s name"
        ));
    }
    let (log, log_hash) = state
        .log
        .tip()
        .ok_or_else(|| format!("{event}: the membership log has not begun"))?;
    // The next number is past what this device has made and past every chain an earlier grant
    // ended; when it is past the end of one it begins a new chain and names nothing before it.
    let floor = state.log.cutoffs(author).last().copied().unwrap_or(0);
    let (seq, prev) = match state
        .ops
        .get(&author)
        .and_then(|ops| ops.iter().next_back())
    {
        Some((last, held)) if *last >= floor => (last + 1, held.id),
        _ => (floor + 1, message::NO_OPERATION),
    };
    let link = Link {
        seq,
        prev,
        log,
        log_hash,
    };
    let signing = message::signing(&state.calendar, event, &travelling)
        .and_then(Result::ok)
        .ok_or_else(|| format!("{event}: the data is not what it names"))?;
    let bytes = message::operation_bytes(&signing.message, &link);
    let signature = identity.sign(&bytes);
    let stamp = if event == "todo.write" {
        Some(message::stamp_of(&travelling).map_err(|error| format!("{event}: {error:?}"))?)
    } else {
        None
    };
    state.ops.entry(author).or_default().insert(
        seq,
        Held {
            id: message::operation_id(&bytes),
            prev,
            stamp,
        },
    );
    Ok(Wire {
        data: travelling.to_string(),
        link,
        signature,
    })
}

/// A note leaves as who really wrote over the write, with their operation as it was made, when
/// the host holds that; and as the host's own person's next operation, when it does not and the
/// note names that person: a copy that holds a tombstone and sends it again vouches for it by
/// sending what the person signed, and a host that holds no author for it sends nothing.
fn seal_note(state: &mut State, data: &str) -> Result<Option<String>, String> {
    let event = "todo.supersede";
    let malformed = || format!("{event}: the data is not a note");
    let value: Value = serde_json::from_str(data).map_err(|_| malformed())?;
    let signing = message::signing(&state.calendar, event, &value)
        .and_then(Result::ok)
        .ok_or_else(malformed)?;
    if let Some(wire) = state.notes.get(&signing.subject) {
        return wire.to_json().map(Some).ok_or_else(malformed);
    }
    if !matches!(&state.own, Some((node, _)) if *node == signing.author) {
        return Err(format!(
            "{event}: this host holds no author for this note, which names node {}",
            signing.author
        ));
    }
    let wire = make_operation(state, event, value, signing.author)?;
    let sent = wire.to_json().ok_or_else(malformed)?;
    state.notes.insert(signing.subject, wire);
    Ok(Some(sent))
}

impl Sealer for Trust {
    fn seal(&mut self, _target: &str, event: &str, data: &str) -> Result<Option<String>, String> {
        let mut state = self.state();
        if let Some(index) = state.forked {
            if event == message::FLOOR_EVENT || message::SIGNED_EVENTS.contains(&event) {
                return Err(format!(
                    "{event}: the membership log forked at {index}, and nothing is sent until an administrator resolves it"
                ));
            }
        }
        if event == message::FLOOR_EVENT {
            return Ok(seal_floor(&state, data));
        }
        if event == "todo.supersede" {
            return seal_note(&mut state, data);
        }
        if !message::SIGNED_EVENTS.contains(&event) {
            return Ok(None);
        }
        seal_write(&mut state, data).map(Some)
    }
}

/// A write leaves as it traveled to this host, with its author's signature, when it is another
/// person's; and as this host's person's own, sealed when the host keeps content from the hub,
/// and signed here, when it is theirs. A write this host cannot make leave as it must is not sent
/// as something else: a host that keeps content and holds no key of the epoch the log is in sends
/// nothing, and does not fall back to an older key or to the clear.
fn seal_write(state: &mut State, data: &str) -> Result<String, String> {
    let event = "todo.write";
    let malformed = || format!("{event}: the data is not a write");
    let visible: Value = serde_json::from_str(data).map_err(|_| malformed())?;
    let signing = message::signing(&state.calendar, event, &visible)
        .and_then(Result::ok)
        .ok_or_else(malformed)?;
    let held = signing.message;
    let own = matches!(&state.own, Some((node, _)) if *node == signing.author);
    // What was made or admitted before leaves as it did: one write is one set of bytes.
    if let Some(wire) = state.wires.get(&held) {
        return wire.to_json().ok_or_else(malformed);
    }
    if !own {
        return Err(format!(
            "{event}: this host holds no signature of node {}'s for it, and signs none in their name",
            signing.author
        ));
    }
    let mut wire = visible.clone();
    if state.keeps_content() {
        let epoch = state.log.epoch();
        let key = state.epoch_keys.get(&epoch).ok_or_else(|| {
            format!(
                "{event}: this host holds no key of epoch {epoch} yet, and writes nothing until it does"
            )
        })?;
        let (number, text) = message::write_content(&visible).map_err(|_| malformed())?;
        let stamp = message::write_stamp(&visible).map_err(|_| malformed())?;
        let object = wire.as_object_mut().ok_or_else(malformed)?;
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
    let operation = make_operation(state, event, wire, signing.author)?;
    let sent = operation.to_json().ok_or_else(malformed)?;
    state.wires.insert(held, operation);
    Ok(sent)
}
