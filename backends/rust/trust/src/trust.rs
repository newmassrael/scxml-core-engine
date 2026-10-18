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
use zeroize::Zeroize;

use crate::chain::{self, Chain, Held, Why, Wire};
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
    /// The list has had the keys of that epoch destroyed, and no host that keeps to it takes one.
    Shredded,
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
    /// Every operation held of each device, the proof of a conflict included: what the chains are
    /// checked against, in whatever order the operations came, and what decides what the machine is
    /// told of a device ([`crate::chain`]).
    ops: BTreeMap<u64, Chain>,
    /// Operations this host told its machine and then stopped holding of the device's chain: an
    /// entry of the log put them past the last accepted of their device, or a conflict among the
    /// device's operations froze its chain below them. For the host to rebuild its machine without
    /// them ([`Trust::take_evicted`]).
    evicted: Vec<(u64, u64)>,
    /// Operations that were not told to the machine and are now: a conflict that held a device's
    /// chain below it went away with the operations that made it (an entry of the log put them past
    /// a cutoff). The machine is built again as for `evicted` ([`Trust::take_restored`]).
    restored: Vec<(u64, u64)>,
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
                evicted: Vec::new(),
                restored: Vec::new(),
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
        // An epoch the list has had destroyed: what this host holds of its keys goes, and is zeroed.
        if let (Ok(true), Body::Shred { epoch }) = (&appended, &entry.body) {
            if let Some(mut key) = state.epoch_keys.remove(epoch) {
                key.zeroize();
            }
        }
        // An entry that takes a device off at a number: what this host admitted of it before it knew,
        // past that number, goes, so that every host holds the same of the device whatever order the
        // entry and the operations came in.
        if let (Ok(true), Body::Grant { .. } | Body::LinkedGrant { .. } | Body::Amend { .. }) =
            (&appended, &entry.body)
        {
            evict_past_the_cutoff(&mut state);
        }
        appended
    }

    /// The operations (device and number) this host held and no longer does, because an entry of the
    /// log put them past the last accepted of their device, since the last call. A machine that was
    /// told them holds their effects, and cannot take them back: the host builds it again from what
    /// the host holds ([`Self::all_operations`]) and tells it the list again.
    pub fn take_evicted(&self) -> Vec<(u64, u64)> {
        std::mem::take(&mut self.state().evicted)
    }

    /// The operations (device and number) that this host did not tell its machine and now does,
    /// since the last call: a conflict among a device's operations that held its chain below them
    /// went away with an operation an entry of the log put past a cutoff. The machine is built again
    /// as for [`Self::take_evicted`].
    pub fn take_restored(&self) -> Vec<(u64, u64)> {
        std::mem::take(&mut self.state().restored)
    }

    /// The keys of the epochs this host holds, for a copy of its device's secrets
    /// ([`crate::backup`]). An epoch the list had destroyed is not here: its key was zeroed when the
    /// entry came, and none is taken after.
    pub fn keys_for_backup(&self) -> Vec<(u32, secrecy::EpochKey)> {
        self.state()
            .epoch_keys
            .iter()
            .map(|(epoch, key)| (*epoch, *key))
            .collect()
    }

    /// What the log holds of each node: every grant of it, oldest first, with the entry that made it,
    /// its role, its key and where the chain it ended was cut (a grant that took writing away).
    pub fn member_history(&self) -> Vec<(u64, Vec<crate::membership::Era>)> {
        self.state()
            .log
            .eras()
            .map(|(node, eras)| (node, eras.into_iter().copied().collect()))
            .collect()
    }

    /// The operations this host tells its machine, as the event and the data each travels with,
    /// device by device and in the order of each chain: what the machine is built again from. A
    /// device whose operations conflict has none of them from the first conflict on
    /// ([`Self::evidence`]).
    pub fn all_operations(&self) -> Vec<(String, String)> {
        let state = self.state();
        state
            .ops
            .values()
            .flat_map(Chain::view)
            .filter_map(|held| Some((held.event.to_string(), held.wire.to_json()?)))
            .collect()
    }

    /// The operations this host holds of conflicting devices and does not tell its machine: the
    /// proof, both operations of every two that conflict and everything of the device after the first
    /// of them. A host that is not given both learns of the conflict from them, so they go to the
    /// relay and to other hosts as every operation does ([`Self::signed_operations`]).
    pub fn evidence(&self) -> Vec<(String, String)> {
        let state = self.state();
        state
            .ops
            .values()
            .flat_map(Chain::evidence)
            .filter_map(|held| Some((held.event.to_string(), held.wire.to_json()?)))
            .collect()
    }

    /// Every operation this host holds, the proof of a conflict included, as the event and the data
    /// it travels with, device by device and in the order of each chain: what is stored, what an
    /// audit takes, and what a host that was not given an operation can be given. A host that takes
    /// all of it holds what this host does.
    pub fn signed_operations(&self) -> Vec<(String, String)> {
        let state = self.state();
        state
            .ops
            .values()
            .flat_map(Chain::all)
            .filter_map(|held| Some((held.event.to_string(), held.wire.to_json()?)))
            .collect()
    }

    /// How many operations this host tells its machine of each device, and the highest number among
    /// them.
    pub fn chains(&self) -> Vec<(u64, usize, u64)> {
        self.state()
            .ops
            .iter()
            .map(|(node, chain)| {
                let numbers = chain.view_numbers();
                (*node, numbers.len(), numbers.last().copied().unwrap_or(0))
            })
            .collect()
    }

    /// The epochs whose keys the list has had destroyed.
    pub fn shredded(&self) -> Vec<u32> {
        self.state().log.shredded().collect()
    }

    /// The node of the first device of the person whose device `node` is (itself, when it was not
    /// linked by another).
    pub fn person_of(&self, node: u64) -> u64 {
        self.state().log.person_of(node)
    }

    /// Every device of the person whose device `node` is that the log holds: the first and those
    /// linked from it, members now or taken off.
    pub fn devices_of(&self, node: u64) -> Vec<u64> {
        self.state().log.devices_of(node)
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
            .and_then(|chain| chain.last().map(Held::seq))
            .unwrap_or(0)
    }

    /// The devices this host holds conflicting operations of, and the number of the first conflict:
    /// two operations at one number, one that does not follow the one before it, a write whose time
    /// goes back along the chain or whose stamp another write has. Proof that the device's chain
    /// forked or that it used a nonce twice, which every host that holds the operations
    /// ([`Self::evidence`]) finds at the same number.
    pub fn equivocating(&self) -> Vec<(u64, u64)> {
        self.state()
            .ops
            .iter()
            .filter_map(|(node, chain)| Some((*node, chain.frozen()?)))
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
                // The checkpoint names an operation this host holds none such of at that number: it
                // holds others there (two, when the device signed two) and not that one.
                let here: Vec<&Held> = state
                    .ops
                    .get(&head.node)
                    .map(|chain| chain.range(head.seq, head.seq).collect())
                    .unwrap_or_default();
                if !here.is_empty() && here.iter().all(|held| held.id != head.id) {
                    let differs = Divergence::Differs {
                        node: head.node,
                        seq: head.seq,
                        by: checkpoint.device,
                    };
                    if !found.contains(&differs) {
                        found.push(differs);
                    }
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
        let Some(chain) = state.ops.get(&node) else {
            return Vec::new();
        };
        chain
            .range(from, to)
            .filter_map(|held| Some((held.event.to_string(), held.wire.to_json()?)))
            .collect()
    }

    /// This host holds the key of `epoch`, which it kept from an earlier run or was given outside a
    /// grant. Whether the list keeps its content from the hub, and which epoch a write is sealed
    /// under, is the membership log's: a key opens what is sealed under its epoch, and a host
    /// holding none that it needs says so and takes or sends nothing in the clear.
    ///
    /// A key of an epoch the list has had destroyed is not kept: `false`.
    pub fn set_epoch_key(&self, epoch: u32, key: secrecy::EpochKey) -> bool {
        let mut state = self.state();
        if state.log.is_shredded(epoch) {
            return false;
        }
        state.epoch_keys.insert(epoch, key);
        true
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
        if state.log.is_shredded(epoch) {
            return Err(GrantError::Shredded);
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
                .flat_map(Chain::all)
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
                hold_verified(&mut state, event, verified, signature);
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

/// What the machine is told of `node`: the number and id of each operation, in order.
fn told_of(state: &State, node: u64) -> Vec<(u64, [u8; 32])> {
    state
        .ops
        .get(&node)
        .map(|chain| chain.view().map(|held| (held.seq(), held.id)).collect())
        .unwrap_or_default()
}

/// Put what the host keeps writes and notes by in line with what the machine is told of `node`: an
/// operation that is not told is not what the host sends a write on as, and one that is, is.
fn sync_tables(state: &mut State, node: u64) {
    let Some(chain) = state.ops.get(&node) else {
        return;
    };
    let signatures: std::collections::BTreeSet<[u8; 64]> =
        chain.all().map(|held| held.wire.signature).collect();
    let told: Vec<(&'static str, Vec<u8>, Wire)> = chain
        .view()
        .map(|held| (held.event, held.key.clone(), held.wire.clone()))
        .collect();
    state
        .wires
        .retain(|_, wire| !signatures.contains(&wire.signature));
    state
        .notes
        .retain(|_, wire| !signatures.contains(&wire.signature));
    for (event, key, wire) in told {
        let table = if event == "todo.write" {
            &mut state.wires
        } else {
            &mut state.notes
        };
        table.entry(key).or_insert(wire);
    }
}

/// What changed in what the machine is told of `node` between `before` and now: the operations that
/// left it are for the host to rebuild its machine without, and those that entered, with. `new` is
/// an operation that was just given and is told now: the machine is told it as it is taken in, and
/// it is no change to what was told before.
fn tell_what_changed(
    state: &mut State,
    node: u64,
    before: &[(u64, [u8; 32])],
    new: Option<(u64, [u8; 32])>,
) {
    let after = told_of(state, node);
    let left: Vec<u64> = before
        .iter()
        .filter(|one| !after.contains(one))
        .map(|(seq, _)| *seq)
        .collect();
    let entered: Vec<u64> = after
        .iter()
        .filter(|one| !before.contains(one) && Some(**one) != new)
        .map(|(seq, _)| *seq)
        .collect();
    if left.is_empty() && entered.is_empty() {
        return;
    }
    state
        .evicted
        .extend(left.into_iter().map(|seq| (node, seq)));
    state
        .restored
        .extend(entered.into_iter().map(|seq| (node, seq)));
    sync_tables(state, node);
}

/// Drop every operation held that the log now puts past the last accepted of its device, judged as
/// an operation is when it arrives (by the entry it names), and judge each device's operations
/// again: the cutoffs the log has are where the segments of a chain begin and end, and the
/// conflicts among them are as the set that is left has them. What the machine is told of a device
/// is what its set says, whatever order the entries and the operations came in.
fn evict_past_the_cutoff(state: &mut State) {
    let nodes: Vec<u64> = state.ops.keys().copied().collect();
    for node in nodes {
        let before = told_of(state, node);
        let cutoffs = state.log.cutoffs(node);
        let Some(chain) = state.ops.get(&node) else {
            continue;
        };
        let gone: Vec<(u64, [u8; 32], [u8; 64])> = chain
            .all()
            .filter(|held| {
                state
                    .log
                    .era_for(node, held.wire.link.log)
                    .and_then(|era| state.log.cutoff_after(node, era))
                    .is_some_and(|cutoff| held.seq() > cutoff)
            })
            .map(|held| (held.seq(), held.id, held.wire.signature))
            .collect();
        if let Some(chain) = state.ops.get_mut(&node) {
            chain.remove(
                |held| {
                    gone.iter()
                        .any(|(seq, id, _)| *seq == held.seq() && *id == held.id)
                },
                &cutoffs,
            );
        }
        // What was held and is not now is not what the host sends a write on as, told or not.
        if !gone.is_empty() {
            let signatures: Vec<[u8; 64]> = gone.iter().map(|(_, _, sig)| *sig).collect();
            state
                .wires
                .retain(|_, wire| !signatures.contains(&wire.signature));
            state
                .notes
                .retain(|_, wire| !signatures.contains(&wire.signature));
        }
        tell_what_changed(state, node, &before, None);
    }
}

/// Put an operation among its device's: the one place that decides what the machine is told of a
/// device. Returns whether the host did not hold it before, and whether the machine is told it now.
fn hold(state: &mut State, node: u64, held: Held) -> (bool, bool) {
    let (seq, id) = (held.seq(), held.id);
    let cutoffs = state.log.cutoffs(node);
    let chain = state.ops.entry(node).or_default();
    // An operation can only freeze its chain lower than it was, never raise it, so what the machine
    // was told and is told no more is what lies from the place the chain freezes at. That place is
    // known before the operation goes in, and only what lies from it is read: a write at the end of a
    // long chain, which freezes nothing, costs the chain nothing.
    let told_above: Vec<(u64, [u8; 32])> = match chain.probe(&held, &cutoffs) {
        Some((at, _)) if chain.frozen().is_none_or(|from| at < from) => {
            chain.view_from(at).map(|one| (one.seq(), one.id)).collect()
        }
        _ => Vec::new(),
    };
    let inserted = chain.insert(held, &cutoffs);
    if !inserted.added {
        let told = chain.at(seq).is_some_and(|one| one.id == id);
        return (false, told);
    }
    let left: Vec<u64> = told_above
        .into_iter()
        .filter(|(at, held_id)| !chain.at(*at).is_some_and(|one| one.id == *held_id))
        .map(|(at, _)| at)
        .collect();
    if !left.is_empty() {
        state
            .evicted
            .extend(left.into_iter().map(|seq| (node, seq)));
        sync_tables(state, node);
    }
    let told = state
        .ops
        .get(&node)
        .is_some_and(|chain| chain.at(seq).is_some_and(|one| one.id == id));
    if told {
        // The common case, an operation added at the end: its table entry, and no more.
        let table_entry = state
            .ops
            .get(&node)
            .and_then(|chain| chain.at(seq))
            .map(|one| (one.event, one.key.clone(), one.wire.clone()));
        if let Some((event, key, wire)) = table_entry {
            let table = if event == "todo.write" {
                &mut state.wires
            } else {
                &mut state.notes
            };
            table.entry(key).or_insert(wire);
        }
    }
    (true, told)
}

fn heads_of(state: &State) -> Vec<Head> {
    state
        .ops
        .iter()
        .filter_map(|(node, chain)| {
            let seq = chain.contiguous();
            Some(Head {
                node: *node,
                seq,
                id: chain.at(seq)?.id,
            })
        })
        .collect()
}

/// The highest number `n` such that `node`'s operations 1 to `n` are all told to the machine.
fn contiguous(state: &State, node: u64) -> u64 {
    state.ops.get(&node).map_or(0, Chain::contiguous)
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
            let here: Vec<&Held> = state
                .ops
                .get(&head.node)
                .map(|chain| chain.range(head.seq, head.seq).collect())
                .unwrap_or_default();
            if !here.is_empty() && here.iter().all(|held| held.id != head.id) {
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
            Ok(taken) if taken.applied => Ok(Some(taken.told)),
            // Held as proof that the device signed conflicting operations: the machine is told none
            // of what the device did from the first of them on.
            Ok(taken) => Err(format!(
                "{event}: node {} signed conflicting operations at or before {}, and this host holds it as proof and tells its machine nothing of the device from there",
                taken.node, taken.seq
            )),
            Err(
                TakeError::NotReached(reason)
                | TakeError::NoKey(reason)
                | TakeError::Refused(reason),
            ) => Err(reason),
        }
    }
}

/// An operation that was taken in: whose it is and the number it has, whether the host held it
/// already (a relay or a mirror hands on what it was given before), whether the machine is told it
/// (`applied`: it is not when it is held as proof that the device signed conflicting operations, from
/// the first of them on), and the event as the machine is told it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Taken {
    pub node: u64,
    pub seq: u64,
    pub new: bool,
    pub applied: bool,
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
            let told = verified.visible.to_string();
            let (new, applied) = hold_verified(state, event, verified, signature);
            Ok(Taken {
                node,
                seq,
                new,
                applied,
                told,
            })
        }
        Err(Refusal::NotReached(reason)) => Err(TakeError::NotReached(reason)),
        Err(Refusal::NoKey(reason)) => Err(TakeError::NoKey(reason)),
        Err(Refusal::Refused(reason)) => Err(TakeError::Refused(reason)),
    }
}

/// Hold an operation that was verified, whoever made it: whether it was new to the host and whether
/// the machine is told it.
fn hold_verified(
    state: &mut State,
    event: &'static str,
    verified: Verified,
    signature: [u8; 64],
) -> (bool, bool) {
    let wire = Wire {
        data: verified.travelling.to_string(),
        link: verified.link,
        signature,
    };
    let held = Held {
        id: verified.id,
        prev: verified.link.prev,
        stamp: verified.stamp,
        wid: verified.wid,
        key: verified.key,
        event,
        wire,
    };
    hold(state, verified.author, held)
}

/// Why an operation was not taken in.
enum Refusal {
    /// It names an entry of the log the host has not reached.
    NotReached(String),
    /// It is sealed under an epoch whose key the host does not hold.
    NoKey(String),
    /// It does not hold: its reason.
    Refused(String),
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
    /// What a write is sealed under (its id, field, time, counter and device), so that two writes
    /// under one stamp are seen as the nonce used twice that they are.
    wid: Option<Vec<u8>>,
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
    let (segment_start, _) = chain::segment(link.seq, &state.log.cutoffs(author));
    if link.seq == segment_start && link.prev != message::NO_OPERATION {
        return Err(format!("{event}: it begins a chain and names an operation before it").into());
    }
    // Nothing is judged here of how the operation stands among the others of its device (a number
    // two are signed at, a chain that does not follow itself, a stamp that goes back or repeats): that
    // is what the set of them says, whatever order they were given in ([`Chain`]), and an operation
    // that conflicts is held as proof and not refused.

    // A write's stamp is its author's clock when it wrote it, and what a write is sealed under is
    // the fields that make its stamp: both are the author's own claims, which the chain checks.
    let (stamp, wid) = if event == "todo.write" {
        (
            Some(message::stamp_of(&value).map_err(|error| format!("{event}: {error:?}"))?),
            Some(message::write_stamp(&value).map_err(|error| format!("{event}: {error:?}"))?),
        )
    } else {
        (None, None)
    };

    let visible = if event == "todo.write" {
        open_write(state, &value, link.log)?
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
        wid,
        travelling: value,
        visible,
        key,
    })
}

/// The write as the machine is told it: the content opened, when this host keeps content from
/// the hub, and the write as it is otherwise. Whether the list keeps content, and which epoch a
/// write may be sealed under, is judged at the entry of the log the write names (`entry`): a device
/// wrote it knowing the log up to there, and two hosts that were given the same entries in another
/// order judge it the same. A write that names an entry before the list began to seal is plain, and
/// holds whether the host took it before or after the entry that began the epoch.
fn open_write(state: &State, wire: &Value, entry: u64) -> Result<Value, Refusal> {
    let epoch_then = state.log.epoch_at(entry);
    let refused = |reason: String| Refusal::Refused(reason);
    let sealed = wire.get("text").and_then(Value::as_str).unwrap_or("");
    if epoch_then == 0 {
        if secrecy::epoch_of(sealed).is_some() {
            return Err(refused(
                "todo.write: it is sealed under an epoch, and the membership log it names has begun none"
                    .to_string(),
            ));
        }
        return Ok(wire.clone());
    }
    let Some(epoch) = secrecy::epoch_of(sealed) else {
        return Err(refused(
            "todo.write: its content is not sealed, and the list keeps content from the hub"
                .to_string(),
        ));
    };
    if epoch == 0 || epoch > epoch_then {
        return Err(refused(format!(
            "todo.write: it is sealed under epoch {epoch}, which the membership log it names has not begun"
        )));
    }
    // A hub that holds no key carries it as it is; a host that opens refuses what the list has had
    // destroyed the keys of, for good (it is not parked: no key will come).
    if state.log.is_shredded(epoch) && !state.relays_sealed() {
        return Err(refused(format!(
            "todo.write: it is sealed under epoch {epoch}, whose keys the list had destroyed"
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
    key: Vec<u8>,
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
    // A device that was shown to have signed conflicting operations signs nothing more: its chain
    // forked (or it used a nonce twice), every other host holds what follows as proof and tells its
    // machine none of it, and one more operation would only be one more of them. An administrator
    // takes the device off, and a new device is granted.
    if let Some(from) = state.ops.get(&author).and_then(Chain::frozen) {
        return Err(format!(
            "{event}: this device's chain conflicts at {from}, and it signs nothing more until an administrator has taken it off"
        ));
    }
    let (log, log_hash) = state
        .log
        .tip()
        .ok_or_else(|| format!("{event}: the membership log has not begun"))?;
    // The next number is past what this device has made and past every chain an earlier grant
    // ended; when it is past the end of one it begins a new chain and names nothing before it. An
    // operation numbered at the end an earlier grant set belongs to the chain that grant ended, and
    // the new key's first one is the number after it.
    let cutoffs = state.log.cutoffs(author);
    let floor = cutoffs.last().copied().unwrap_or(0);
    let (seq, prev) = match state.ops.get(&author).and_then(Chain::last) {
        Some(last) if last.seq() > floor => (last.seq() + 1, last.id),
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
    let (stamp, wid) = if event == "todo.write" {
        (
            Some(message::stamp_of(&travelling).map_err(|error| format!("{event}: {error:?}"))?),
            Some(message::write_stamp(&travelling).map_err(|error| format!("{event}: {error:?}"))?),
        )
    } else {
        (None, None)
    };
    let wire = Wire {
        data: travelling.to_string(),
        link,
        signature,
    };
    let event = signed_name(event).ok_or_else(|| format!("{event}: it is not a signed event"))?;
    let held = Held {
        id: message::operation_id(&bytes),
        prev,
        stamp,
        wid,
        key,
        event,
        wire: wire.clone(),
    };
    // What this device signs it signs once and in order: a write whose stamp goes back along its
    // chain, or that another write of it has, is not signed. The second would be sealed under the
    // nonce of the first, and what a relay knows of the one gives it the other.
    let conflict = state
        .ops
        .get(&author)
        .and_then(|chain| chain.probe(&held, &cutoffs));
    if let Some((at, why)) = conflict {
        return Err(match why {
            Why::Repeats => format!(
                "{event}: its stamp names a write this device signed already (at {at}), and a stamp is sealed under once"
            ),
            Why::GoesBack => format!(
                "{event}: its stamp goes back along this device's chain (a write at {at} has a later time)"
            ),
            Why::Forked => format!(
                "{event}: it would not follow the chain this device has signed (conflict at {at})"
            ),
        });
    }
    hold(state, author, held);
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
    let wire = make_operation(state, event, value, signing.author, signing.subject)?;
    let sent = wire.to_json().ok_or_else(malformed)?;
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
    let operation = make_operation(state, event, wire, signing.author, held)?;
    operation.to_json().ok_or_else(malformed)
}
