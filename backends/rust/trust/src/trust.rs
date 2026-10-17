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
use crate::message::{self, SIGNATURE_FIELD};
use crate::secrecy;

/// The field under which a note carries the time its author made it at.
const AT_FIELD: &str = "at";

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
    /// Who the administrators and members are, from when, and the epoch: the chain of entries the
    /// host holds and has verified.
    log: Log,
    /// Where the log forked, when the host was shown two histories: nothing is taken in until an
    /// administrator resolves it.
    forked: Option<u64>,
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
    /// What the log says of `node` at the time `at`.
    fn era_at(&self, node: u64, at: i64) -> Option<&crate::membership::Era> {
        self.log.era_at(node, at)
    }

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
/// made it, and the signature is true under the key the log gave them for that time.
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

/// A note leaves as who really wrote over the write and when, with their signature, when the
/// host holds that; and as the host's own person's, made now, when it does not and the note
/// names that person: a copy that holds a tombstone and sends it again vouches for it.
fn seal_note(state: &mut State, data: &str) -> Result<Option<String>, String> {
    let malformed = || "todo.supersede: the data is not a note".to_string();
    let mut value: Value = serde_json::from_str(data).map_err(|_| malformed())?;
    let object = value.as_object_mut().ok_or_else(malformed)?;
    // The subject is the same whoever wrote over and whenever, so a placeholder reads it.
    object.insert(AT_FIELD.to_string(), Value::String("0".to_string()));
    let placeholder = message::signing(&state.calendar, "todo.supersede", &value)
        .and_then(Result::ok)
        .ok_or_else(malformed)?;
    let subject = placeholder.subject;
    let (by, at, signature) = match state.notes.get(&subject) {
        Some(note) => (note.by, note.at, note.signature),
        None => {
            let (node, identity) = state.own.as_ref().ok_or_else(|| {
                "todo.supersede: this host holds no author for this note and signs none of its own"
                    .to_string()
            })?;
            if placeholder.author != *node {
                return Err(format!(
                    "todo.supersede: this host holds no author for this note, which names node {}",
                    placeholder.author
                ));
            }
            let at = (state
                .clock
                .as_ref()
                .ok_or("todo.supersede: this host has no clock to make a note at")?)(
            );
            let object = value.as_object_mut().ok_or_else(malformed)?;
            object.insert(AT_FIELD.to_string(), Value::String(at.to_string()));
            let signing = message::signing(&state.calendar, "todo.supersede", &value)
                .and_then(Result::ok)
                .ok_or_else(malformed)?;
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
    let object = value.as_object_mut().ok_or_else(malformed)?;
    object.insert("by".to_string(), Value::String(by.to_string()));
    object.insert(AT_FIELD.to_string(), Value::String(at.to_string()));
    object.insert(
        SIGNATURE_FIELD.to_string(),
        Value::String(encoding::encode(&signature)),
    );
    Ok(Some(value.to_string()))
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
    if let Some(wire) = state.wires.get(&held).cloned() {
        let mut value: Value = serde_json::from_str(&wire.data).map_err(|_| malformed())?;
        value.as_object_mut().ok_or_else(malformed)?.insert(
            SIGNATURE_FIELD.to_string(),
            Value::String(encoding::encode(&wire.signature)),
        );
        return Ok(value.to_string());
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
    let travelling = message::signing(&state.calendar, event, &wire)
        .and_then(Result::ok)
        .ok_or_else(malformed)?;
    let (_, identity) = state
        .own
        .as_ref()
        .ok_or_else(|| format!("{event}: this host has no person"))?;
    let signature = identity.sign(&travelling.message);
    state.wires.insert(
        held,
        Wire {
            data: wire.to_string(),
            signature,
        },
    );
    wire.as_object_mut().ok_or_else(malformed)?.insert(
        SIGNATURE_FIELD.to_string(),
        Value::String(encoding::encode(&signature)),
    );
    Ok(wire.to_string())
}
