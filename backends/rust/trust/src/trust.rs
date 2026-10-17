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
//! An instant is settled, and every copy forgets the tombstones up to it, on the members' word and
//! no one else's: the checkpoints of every device that is a writer ([`Trust::checkpoint`]), all
//! agreeing on the instant and the digest of their copies, make a settlement that each host checks
//! alone ([`Trust::settlement`]). A hub holds none of the devices' keys and cannot make one, and an
//! administrator is not asked for anything. The same checkpoints name the head of each device's
//! chain, which is how a host learns that an operation was withheld from it
//! ([`Trust::divergences`]) and fetches it from another host that holds it
//! ([`Trust::operations`]).
//!
//! One [`Trust`] is both halves of one host: it is cloned, and a clone is put in front of the
//! machine as its [`Admission`] and another behind it as its [`Sealer`].

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard};

use sce_rust_mesh::endpoint::{Admission, Sealer};
use serde_json::{json, Value};

use crate::checkpoint::{Checkpoint, Head, Settlement};
use crate::encoding;
use crate::identity::{Identity, PublicKey};
use crate::membership::{Body, Draft, Entry, Hash, Log, LogError, Profile, WRITER};
use crate::message::{self, Link, SETTLEMENT_FIELD, SIGNATURE_FIELD};
use crate::secrecy;

/// Why a checkpoint was not made or not taken in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckpointError {
    /// This host has no person, so it signs no checkpoint.
    NoPerson,
    /// The membership log has no entry yet.
    NoLog,
    /// It is a checkpoint of another list.
    WrongList,
    /// It names an entry of the log that this host does not hold as it is: the host is behind, or
    /// holds another history. It can be taken in once the host holds that entry.
    LogNotHeld { log: u64 },
    /// The device was not a writer at the entry the checkpoint names.
    NotAWriter { device: u64, log: u64 },
    /// The key the log gave the device at that entry did not sign it.
    BadSignature,
}

/// What a checkpoint shows that this host does not hold as it should.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Divergence {
    /// `by` holds the chain of `node` up to `named` and this host only up to `held`: the
    /// operations after `held` exist, and this host was not given them. Ask another host for
    /// [`Trust::operations`] of `node` from `held + 1`.
    Missing {
        node: u64,
        held: u64,
        named: u64,
        by: u64,
    },
    /// `by` holds an operation of `node` at `seq` that is not the one this host holds there: the
    /// device signed two histories, or `by` did not.
    Differs { node: u64, seq: u64, by: u64 },
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
/// operations arrive in; and the operation itself, as it travels, to give to a host that was
/// not given it.
#[derive(Clone)]
struct Held {
    id: [u8; 32],
    prev: [u8; 32],
    stamp: Option<(i64, u64)>,
    event: &'static str,
    wire: Wire,
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
    /// The checkpoints the host made or took in, one for each device and each instant and digest
    /// it speaks of: what a settlement is made from, and what shows an operation withheld.
    checkpoints: BTreeMap<(u64, i64, u32), Checkpoint>,
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
/// settlement by the members, so a host that relays it could settle an instant its copies have
/// not, and bring back with the old signed writes what was deleted. A host that trusts the one
/// that settles (its own hub) allows it.
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
                checkpoints: BTreeMap::new(),
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

    /// Whether the list's epochs are an MLS group's, whose key a host takes from the group's
    /// exporter, and not a key an administrator drew (`Self::rotate_epoch`).
    pub fn is_mls(&self) -> bool {
        self.state().log.is_mls()
    }

    /// The hash of the commit message the log names as the one that began `epoch` of an MLS list.
    pub fn commit_of(&self, epoch: u32) -> Option<Hash> {
        self.state().log.commit_of(epoch)
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

    /// The head of every device's chain this host holds in full: the highest number `n` such that
    /// it holds the operations 1 to `n` of the device, with the id of the nth. What a checkpoint
    /// of this host names. A device of which the host holds nothing from number 1 has no head.
    pub fn heads(&self) -> Vec<Head> {
        heads_of(&self.state())
    }

    /// This host's person says that their copy holds, of everything up to `before`, the digest
    /// `digest` (the machine's `frozenHash` of that settle), and which operations of each device
    /// it holds. Signed with the person's key, under the entry of the log the host holds now, and
    /// kept: the host's own checkpoint is one of those a settlement is made from. Sent to the
    /// other hosts by any channel, a relay's included, for them to [`Self::take_checkpoint`].
    pub fn checkpoint(&self, before: i64, digest: u32) -> Result<Checkpoint, CheckpointError> {
        let mut state = self.state();
        let (device, identity) = state.own.as_ref().ok_or(CheckpointError::NoPerson)?;
        let (log, log_hash) = state.log.tip().ok_or(CheckpointError::NoLog)?;
        let mut checkpoint = Checkpoint {
            list: state.calendar.clone(),
            device: *device,
            log,
            log_hash,
            before,
            digest,
            heads: heads_of(&state),
            signature: [0; 64],
        };
        checkpoint.signature = identity.sign(&checkpoint.signing_bytes());
        verify_checkpoint(&state, &checkpoint)?;
        store_checkpoint(&mut state, checkpoint.clone());
        Ok(checkpoint)
    }

    /// Take a checkpoint another host made: held when it is for this list, names an entry of the
    /// log this host holds, and the key the log gave its device then signed it, and the device was
    /// a writer. `Ok(false)` when it is no fresher than one already held for that device and that
    /// instant and digest. Whether the heads it names show anything missing is
    /// [`Self::divergences`].
    pub fn take_checkpoint(&self, checkpoint: &Checkpoint) -> Result<bool, CheckpointError> {
        let mut state = self.state();
        verify_checkpoint(&state, checkpoint)?;
        Ok(store_checkpoint(&mut state, checkpoint.clone()))
    }

    /// The checkpoints this host holds, its own and the ones it took in.
    pub fn checkpoints(&self) -> Vec<Checkpoint> {
        self.state().checkpoints.values().cloned().collect()
    }

    /// The settlement of `before` with `digest`, when the checkpoints this host holds make one:
    /// those of every device that was a writer at one entry of the log, each for that instant and
    /// digest. A host that holds the log checks it alone. `None` while a writer's is missing, or
    /// when the heads the checkpoints name disagree on an operation.
    pub fn settlement(&self, before: i64, digest: u32) -> Option<Settlement> {
        settlement_of(&self.state(), before, digest)
    }

    /// What the checkpoints this host holds show that it does not hold as it should: an
    /// operation the others hold and this host was not given (a relay that withheld it, or one
    /// still in flight), and an operation that is not the one this host holds at that number.
    pub fn divergences(&self) -> Vec<Divergence> {
        let state = self.state();
        let mut found: Vec<Divergence> = Vec::new();
        let mut furthest: BTreeMap<u64, (u64, u64)> = BTreeMap::new();
        for checkpoint in state.checkpoints.values() {
            for head in &checkpoint.heads {
                match state.ops.get(&head.node).and_then(|ops| ops.get(&head.seq)) {
                    Some(held) if held.id != head.id => {
                        let differs = Divergence::Differs {
                            node: head.node,
                            seq: head.seq,
                            by: checkpoint.device,
                        };
                        if !found.contains(&differs) {
                            found.push(differs);
                        }
                    }
                    _ => {}
                }
                let entry = furthest.entry(head.node).or_insert((0, checkpoint.device));
                if head.seq > entry.0 {
                    *entry = (head.seq, checkpoint.device);
                }
            }
        }
        for (node, (named, by)) in furthest {
            let held = contiguous(&state, node);
            if named > held {
                found.push(Divergence::Missing {
                    node,
                    held,
                    named,
                    by,
                });
            }
        }
        found
    }

    /// Take an operation in as [`Admission::admit`] does, and say whose it is, what number it has,
    /// whether the host held it already, and what the machine is told. What a relay does with
    /// what it is given, and what a host does with what a relay gives it. Nothing is taken from
    /// a log that forked.
    pub fn take_operation(&self, event: &str, data: &str) -> Result<Taken, TakeError> {
        let mut state = self.state();
        if let Some(index) = state.forked {
            return Err(TakeError::Refused(format!(
                "{event}: the membership log forked at {index}, and nothing is taken in until an administrator resolves it"
            )));
        }
        let event = signed_name(event).ok_or_else(|| {
            TakeError::Refused(format!("{event}: it is not an event that is signed"))
        })?;
        take(&mut state, event, data)
    }

    /// How many entries the membership log this host holds has: the position the next one takes.
    pub fn log_len(&self) -> u64 {
        self.state().log.entries().len() as u64
    }

    /// The operations of `node` numbered `from` to `to` that this host holds, each as the event
    /// and the data it travels with: what a host that was not given them takes in by
    /// [`Admission::admit`], which checks each as it would from anyone, so that the host asked
    /// need not be trusted.
    pub fn operations(&self, node: u64, from: u64, to: u64) -> Vec<(String, String)> {
        if from > to {
            return Vec::new();
        }
        let state = self.state();
        let Some(ops) = state.ops.get(&node) else {
            return Vec::new();
        };
        ops.range(from..=to)
            .filter_map(|(_, held)| Some((held.event.to_string(), held.wire.to_json()?)))
            .collect()
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
    /// with the checkpoints it made or took in, and every operation it holds (the writes it
    /// relays and the notes of who wrote over what, every number of every chain), each as the
    /// event and the data it goes on the wire with. It holds no key: a host keeps those where it
    /// keeps its person's secret.
    pub fn to_text(&self) -> String {
        let state = self.state();
        json!({
            "checkpoints": state.checkpoints.values().map(Checkpoint::to_value).collect::<Vec<_>>(),
            "operations": state.ops.values()
                .flat_map(|ops| ops.values())
                .filter_map(|held| Some(json!([held.event, held.wire.to_json()?])))
                .collect::<Vec<_>>(),
        })
        .to_string()
    }

    /// Take back what [`Self::to_text`] wrote: each checkpoint and each operation that today's log
    /// verifies is kept, and the rest are dropped, since what a host reads back from its own
    /// storage is not taken for true. The chain of this host's own person is what was kept of it,
    /// so that what it writes next is the next one. Returns how many were kept.
    pub fn restore_from_text(&self, text: &str) -> usize {
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
        let mut kept = 0;
        let mut state = self.state();
        for row in rows("operations") {
            let (Some(event), Some(text)) = (
                row.get(0).and_then(Value::as_str),
                row.get(1).and_then(Value::as_str),
            ) else {
                continue;
            };
            let Some(event) = message::SIGNED_EVENTS
                .iter()
                .copied()
                .find(|name| *name == event)
            else {
                continue;
            };
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
        for row in rows("checkpoints") {
            let Some(checkpoint) = Checkpoint::from_value(&row) else {
                continue;
            };
            if verify_checkpoint(&state, &checkpoint).is_ok() {
                store_checkpoint(&mut state, checkpoint);
                kept += 1;
            }
        }
        kept
    }
}

/// The highest number `n` such that `node`'s operations 1 to `n` are all held.
fn contiguous(state: &State, node: u64) -> u64 {
    let Some(ops) = state.ops.get(&node) else {
        return 0;
    };
    let mut head = 0;
    while ops.contains_key(&(head + 1)) {
        head += 1;
    }
    head
}

fn heads_of(state: &State) -> Vec<Head> {
    state
        .ops
        .iter()
        .filter_map(|(node, ops)| {
            let seq = contiguous(state, *node);
            Some(Head {
                node: *node,
                seq,
                id: ops.get(&seq)?.id,
            })
        })
        .collect()
}

/// Whether `checkpoint` holds: it is for this list, names an entry of the log this host holds as
/// it is, its device was a writer under that entry, and the key the log gave the device then
/// signed it.
fn verify_checkpoint(state: &State, checkpoint: &Checkpoint) -> Result<(), CheckpointError> {
    if checkpoint.list != state.calendar {
        return Err(CheckpointError::WrongList);
    }
    if state.log.hash_at(checkpoint.log) != Some(checkpoint.log_hash) {
        return Err(CheckpointError::LogNotHeld {
            log: checkpoint.log,
        });
    }
    let not_a_writer = CheckpointError::NotAWriter {
        device: checkpoint.device,
        log: checkpoint.log,
    };
    let era = state
        .log
        .era_for(checkpoint.device, checkpoint.log)
        .ok_or_else(|| not_a_writer.clone())?;
    if era.role < WRITER {
        return Err(not_a_writer);
    }
    if !checkpoint.is_canonical()
        || !era
            .key
            .verifies(&checkpoint.signing_bytes(), &checkpoint.signature)
    {
        return Err(CheckpointError::BadSignature);
    }
    Ok(())
}

/// Keep a checkpoint that holds. For one device and one instant and digest only one is kept: a
/// later one replaces it when it names more of the devices' chains, and `false` is returned when
/// it does not.
fn store_checkpoint(state: &mut State, checkpoint: Checkpoint) -> bool {
    let key = (checkpoint.device, checkpoint.before, checkpoint.digest);
    let reach = |held: &Checkpoint| -> u64 { held.heads.iter().map(|head| head.seq).sum() };
    match state.checkpoints.get(&key) {
        Some(held) if reach(held) >= reach(&checkpoint) => false,
        _ => {
            state.checkpoints.insert(key, checkpoint);
            true
        }
    }
}

/// The devices that were writers under the entry of the log at `entry`.
fn writers_at(state: &State, entry: u64) -> Vec<u64> {
    state
        .log
        .eras()
        .filter_map(|(node, _)| {
            let era = state.log.era_for(node, entry)?;
            (era.role >= WRITER).then_some(node)
        })
        .collect()
}

/// Whether the heads two checkpoints name agree: the same number of one device is the same
/// operation.
fn heads_agree(checkpoints: &[&Checkpoint]) -> bool {
    let mut named: BTreeMap<(u64, u64), Hash> = BTreeMap::new();
    checkpoints
        .iter()
        .flat_map(|checkpoint| checkpoint.heads.iter())
        .all(|head| *named.entry((head.node, head.seq)).or_insert(head.id) == head.id)
}

/// The settlement the checkpoints a host holds make for `before` and `digest`: from the latest
/// entry of the log under which every device that was a writer there has one.
fn settlement_of(state: &State, before: i64, digest: u32) -> Option<Settlement> {
    let mut under: BTreeMap<u64, Vec<&Checkpoint>> = BTreeMap::new();
    for checkpoint in state.checkpoints.values() {
        if checkpoint.before == before && checkpoint.digest == digest {
            under.entry(checkpoint.log).or_default().push(checkpoint);
        }
    }
    under.into_iter().rev().find_map(|(log, checkpoints)| {
        let writers = writers_at(state, log);
        let complete = !writers.is_empty()
            && writers
                .iter()
                .all(|writer| checkpoints.iter().any(|c| c.device == *writer));
        (complete && heads_agree(&checkpoints)).then(|| Settlement {
            checkpoints: checkpoints.into_iter().cloned().collect(),
        })
    })
}

/// Whether `settlement` proves that `before` is settled with `digest`: every checkpoint holds,
/// each is for that instant and digest, they all speak under one entry of the log, they are from
/// the devices that were writers under it and from none other, no device twice, and the heads they
/// name agree with each other and with the operations this host holds.
fn verify_settlement(
    state: &State,
    before: i64,
    digest: u32,
    settlement: &Settlement,
) -> Result<(), String> {
    let checkpoints: Vec<&Checkpoint> = settlement.checkpoints.iter().collect();
    let Some(first) = checkpoints.first() else {
        return Err("it holds no checkpoint".to_string());
    };
    for checkpoint in &checkpoints {
        verify_checkpoint(state, checkpoint).map_err(|error| {
            format!(
                "the checkpoint of node {} does not hold: {error:?}",
                checkpoint.device
            )
        })?;
        if checkpoint.before != before || checkpoint.digest != digest {
            return Err(format!(
                "the checkpoint of node {} is for another instant or digest",
                checkpoint.device
            ));
        }
        if checkpoint.log != first.log {
            return Err("the checkpoints speak under different entries of the log".to_string());
        }
    }
    let mut devices: Vec<u64> = checkpoints.iter().map(|c| c.device).collect();
    devices.sort_unstable();
    if devices.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err("a device is in it twice".to_string());
    }
    let mut writers = writers_at(state, first.log);
    writers.sort_unstable();
    if devices != writers {
        return Err(format!(
            "it is from nodes {devices:?} and the writers at entry {} were {writers:?}",
            first.log
        ));
    }
    if !heads_agree(&checkpoints) {
        return Err("the checkpoints name different operations at one number".to_string());
    }
    for checkpoint in &checkpoints {
        for head in &checkpoint.heads {
            let held = state.ops.get(&head.node).and_then(|ops| ops.get(&head.seq));
            if held.is_some_and(|held| held.id != head.id) {
                return Err(format!(
                    "node {} names another operation than this host holds at number {} of node {}",
                    checkpoint.device, head.seq, head.node
                ));
            }
        }
    }
    Ok(())
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
        let Some(event) = signed_name(event) else {
            return Ok(None);
        };
        match take(&mut state, event, data) {
            Ok(taken) => Ok(Some(taken.told)),
            Err(
                TakeError::NotReached(reason)
                | TakeError::NoKey(reason)
                | TakeError::Refused(reason),
            ) => Err(reason),
        }
    }
}

/// An operation that was taken in: whose it is and the number it has, whether the host held it
/// already (a relay or a mirror hands on what it was given before), and the event as the machine
/// is told it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Taken {
    pub node: u64,
    pub seq: u64,
    pub new: bool,
    pub told: String,
}

/// Why an operation was not taken in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TakeError {
    /// It names an entry of the membership log that this host has not reached yet: it holds once
    /// the host has the log up to there, and a host that is fed in order stops and asks for the log.
    NotReached(String),
    /// It is sealed under an epoch whose key this host does not hold. It holds if the host comes to
    /// hold the key (the group's commit reaches it, or a member gives it the key), and never if it
    /// does not (an epoch before this host joined, which a new member is given no history of): a
    /// host that is fed in order parks it and goes on.
    NoKey(String),
    /// It does not hold, whenever it is looked at again.
    Refused(String),
}

fn take(state: &mut State, event: &'static str, data: &str) -> Result<Taken, TakeError> {
    let mut value: Value = serde_json::from_str(data)
        .map_err(|_| TakeError::Refused(format!("{event}: the data is not JSON")))?;
    let signature = signature_of(&mut value, event).map_err(TakeError::Refused)?;
    match verify_operation(state, event, value, &signature) {
        Ok(verified) => {
            let (node, seq) = (verified.author, verified.link.seq);
            let new = !state
                .ops
                .get(&node)
                .is_some_and(|ops| ops.contains_key(&seq));
            let told = verified.visible.to_string();
            record(state, event, verified, signature);
            Ok(Taken {
                node,
                seq,
                new,
                told,
            })
        }
        Err(Refusal::NotReached(reason)) => Err(TakeError::NotReached(reason)),
        Err(Refusal::NoKey(reason)) => Err(TakeError::NoKey(reason)),
        Err(Refusal::Refused(reason)) => Err(TakeError::Refused(reason)),
        Err(Refusal::Equivocation { node, seq, reason }) => {
            let from = state.frozen.get(&node).map_or(seq, |held| (*held).min(seq));
            state.frozen.insert(node, from);
            Err(TakeError::Refused(reason))
        }
    }
}

/// Why an operation was not taken in.
enum Refusal {
    /// It names an entry of the log the host has not reached.
    NotReached(String),
    /// It is sealed under an epoch whose key the host does not hold.
    NoKey(String),
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
        let reason = format!(
            "{event}: it names entry {} of the membership log, which this host does not hold as it is",
            link.log
        );
        return Err(if link.log >= state.log.entries().len() as u64 {
            Refusal::NotReached(reason)
        } else {
            Refusal::Refused(reason)
        });
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
fn record(state: &mut State, event: &'static str, verified: Verified, signature: [u8; 64]) {
    let wire = Wire {
        data: verified.travelling.to_string(),
        link: verified.link,
        signature,
    };
    state.ops.entry(verified.author).or_default().insert(
        verified.link.seq,
        Held {
            id: verified.id,
            prev: verified.link.prev,
            stamp: verified.stamp,
            event,
            wire: wire.clone(),
        },
    );
    let table = if event == "todo.write" {
        &mut state.wires
    } else {
        &mut state.notes
    };
    table.entry(verified.key).or_insert(wire);
}

/// The write as the machine is told it: the content opened, when this host keeps content from
/// the hub, and the write as it is otherwise.
fn open_write(state: &State, wire: &Value) -> Result<Value, Refusal> {
    if !state.keeps_content() {
        return Ok(wire.clone());
    }
    let refused = |reason: String| Refusal::Refused(reason);
    let sealed = wire.get("text").and_then(Value::as_str).unwrap_or("");
    let Some(epoch) = secrecy::epoch_of(sealed) else {
        return Err(refused(
            "todo.write: its content is not sealed, and the list keeps content from the hub"
                .to_string(),
        ));
    };
    if epoch == 0 || epoch > state.log.epoch() {
        return Err(refused(format!(
            "todo.write: it is sealed under epoch {epoch}, which the membership log has not begun"
        )));
    }
    if state.relays_sealed() {
        return Ok(wire.clone());
    }
    let stamp =
        message::write_stamp(wire).map_err(|error| refused(format!("todo.write: {error:?}")))?;
    let (value, text) =
        secrecy::open(&state.calendar, &state.epoch_keys, &stamp, sealed).map_err(|error| {
            match error {
            secrecy::OpenError::NotSealed => refused(
                "todo.write: its content is not sealed, and this host keeps content from the hub"
                    .to_string(),
            ),
            // A key this host does not hold yet (the group's commit has not reached it) or never
            // will (an epoch before it joined): the host asks again later, and does not take the
            // write for one that cannot hold.
            secrecy::OpenError::NoKey { epoch } => {
                Refusal::NoKey(format!("todo.write: this host holds no key of epoch {epoch}"))
            }
            secrecy::OpenError::Corrupt => {
                refused("todo.write: its content does not open".to_string())
            }
        }
        })?;
    let mut visible = wire.clone();
    let object = visible
        .as_object_mut()
        .ok_or_else(|| refused("todo.write: the data is not an object".to_string()))?;
    object.insert("value".to_string(), Value::String(value.to_string()));
    object.insert("text".to_string(), Value::String(text));
    Ok(visible)
}

/// A `todo.floor` is admitted when the members settled it: its `cert` is the checkpoints of every
/// writer, each for this instant and this hash ([`verify_settlement`]). The machine is told the
/// floor without it. One with no settlement is admitted only when the host was told to take it
/// unsigned, and a settlement that does not hold is refused whatever the host was told.
fn admit_floor(state: &State, data: &str) -> Result<Option<String>, String> {
    let event = message::FLOOR_EVENT;
    let mut value: Value =
        serde_json::from_str(data).map_err(|_| format!("{event}: the data is not JSON"))?;
    let carried = value
        .as_object_mut()
        .and_then(|object| object.remove(SETTLEMENT_FIELD));
    let Some(carried) = carried else {
        return if state.refuses_unsigned(event) {
            Err(format!(
                "{event}: it carries no settlement by the members and this host does not take it on a peer's word"
            ))
        } else {
            Ok(None)
        };
    };
    let (before, hash) =
        message::floor_fields(&value).map_err(|error| format!("{event}: {error:?}"))?;
    let settlement = Settlement::from_value(&carried)
        .ok_or_else(|| format!("{event}: its settlement is not checkpoints"))?;
    verify_settlement(state, before, hash, &settlement)
        .map_err(|reason| format!("{event}: the settlement does not hold: {reason}"))?;
    Ok(Some(value.to_string()))
}

/// The floor leaves with the settlement the members' checkpoints make of it, when the host holds
/// one, and as the machine made it when it does not.
fn seal_floor(state: &State, data: &str) -> Option<String> {
    let mut value: Value = serde_json::from_str(data).ok()?;
    let (before, hash) = message::floor_fields(&value).ok()?;
    let settlement = settlement_of(state, before, hash)?;
    value
        .as_object_mut()?
        .insert(SETTLEMENT_FIELD.to_string(), settlement.to_value());
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
    let wire = Wire {
        data: travelling.to_string(),
        link,
        signature,
    };
    let event = signed_name(event).ok_or_else(|| format!("{event}: it is not a signed event"))?;
    state.ops.entry(author).or_default().insert(
        seq,
        Held {
            id: message::operation_id(&bytes),
            prev,
            stamp,
            event,
            wire: wire.clone(),
        },
    );
    Ok(wire)
}

/// The name of a signed event as the host keeps it, when `event` is one.
fn signed_name(event: &str) -> Option<&'static str> {
    message::SIGNED_EVENTS
        .iter()
        .copied()
        .find(|name| *name == event)
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
