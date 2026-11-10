// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The rule a host puts in front of its machine and behind it.
//!
//! Who may write the list is what the membership log says (`crate::membership`), and nothing a
//! peer says about itself: a change that arrives is admitted when its author, the node it names as
//! the one that made it (`by`), was a member whose role let them write at the time they made it,
//! and the signature it carries is true under the key the log gave them for that time. A change
//! that leaves carries its author's signature: made here when the author is this host's own
//! person, and otherwise the one it arrived with, which is how a host hands on what a member wrote
//! without being able to make it.
//!
//! The list it guards is the standard shared record list (`crate::message::LIST_DOCUMENT`), and
//! the event it signs, admits and refuses is that part's change, `record_list.change`
//! ([`crate::message::SIGNED`]): one change, whole, is one signed operation of its author's chain
//! (docs/adr/0085, section 1), and the content of each of its writes is what is sealed when the
//! list keeps it from the hub. The signature and the change's place in its chain are the host's:
//! they are written into the event as it leaves and taken out as it arrives, and a machine sees
//! none of them.
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
//! A change is **stable** once every member of the list has been told it (docs/adr/0085, sections
//! 3 and 4): what a member has been told is read from the causal context of its latest operation,
//! and a member that has written nothing since it was told more signs a mark of it
//! ([`Trust::sign_mark`], `message::MARK_EVENT`), an operation of its chain no machine is told.
//! The stable cut is the pointwise minimum of what is known of every member
//! ([`Trust::stable_cut`]), at which a host folds its journal.
//!
//! A change a host keeps a journal of is signed as any other ([`Trust::keep_made`]) and held back
//! from the relay until the host says it recorded the operation ([`Trust::recorded`]): the
//! write-ahead rule of docs/adr/0083, section 3.1, whose records a restarted host gives back
//! ([`Trust::restore_records`]). What a machine built again is given is the history in causal order
//! ([`Trust::history`]).
//!
//! One [`Trust`] is both halves of one host: it is cloned, and a clone is put in front of the
//! machine as its [`Admission`] and another behind it as its [`Sealer`].

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex, MutexGuard};

use sce_rust_mesh::endpoint::{Admission, Sealer};
use serde_json::{json, Value};
use zeroize::Zeroize;

use crate::chain::{self, Chain, Held, Why, Wire};
use crate::checkpoint::{Checkpoint, Head, Settlement};
use crate::encoding;
use crate::identity::{Identity, PublicKey};
use crate::membership::{Body, Draft, Entry, Hash, Log, LogError, Profile, READER, WRITER};
use crate::message::{
    self, Link, CHANGE_EVENT, FLOOR_EVENT, MARK_EVENT, OPERATION_EVENTS, ROLE_EVENT,
    SETTLEMENT_FIELD, SIGNATURE_FIELD,
};
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
    /// Whether this host's person's own chain may be behind what the device signed and published:
    /// its state came back from a store that may be older than the device's last operation. Until
    /// the chain is read back from where it was published ([`Trust::confirm_own_chain`]), the device
    /// signs nothing new, since the next number it would sign at may be one it used already.
    awaiting_own_chain: bool,
    /// What this host's machine sent that its person may not sign yet, in the order it was sent
    /// ([`Trust::keep_sent`]): signed in that order as soon as the person may sign
    /// ([`Trust::sign_waiting`]), or refused, each, once the log says the person never will.
    waiting: Vec<Waiting>,
    /// The checkpoints the host made or took in, one for each device and each instant and digest
    /// it speaks of: what a settlement is made from, and what shows an operation withheld.
    checkpoints: BTreeMap<(u64, i64, u32), Checkpoint>,
    /// Each change that was admitted or made here, by the bytes it is signed as when the machine
    /// holds it (its content opened, [`message::Signing::message`]): what a change the machine
    /// sends again leaves as, with the number and the signature it has. It is the operation as it
    /// travels.
    changes: BTreeMap<Vec<u8>, Wire>,
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
    /// Whether this host's list is keyed by a group this host's device is in
    /// ([`Trust::keyed_by_a_group`]): its person's writes are sealed from the first, and none is
    /// signed while the log names no epoch.
    keyed_by_a_group: bool,
    /// The events that arrive with no signature and make a copy forget, which this host does
    /// not take ([`UNSIGNED_REFUSED`], and [`Trust::allow_unsigned`]).
    unsigned_refused: Vec<String>,
    /// What this host's machine has been told of each device, by its node: the highest number of
    /// the device's chain told, every one below it told too (docs/adr/0085, section 3: the
    /// delivered vector). The device's own column moves when its person signs, since the machine
    /// made what it signs; every other column moves only by [`Trust::deliver`].
    delivered: BTreeMap<u64, u64>,
    /// The operations that hold but whose content this host cannot open, by node and number, with
    /// the epoch each is sealed under: what [`Trust::deliver`] passes over without telling, when
    /// the epoch is one before this host's first, of which a member is given no history.
    unopened: BTreeMap<u64, BTreeMap<u64, u32>>,
    /// The numbers of this host's person's own operations that were signed for a change its host
    /// keeps a journal of ([`Trust::keep_made`]) and that the host has not said it recorded
    /// ([`Trust::recorded`]): the outbox (docs/adr/0083, section 3.1). No operation of the
    /// person's chain from the lowest of them on is given to a relay ([`Trust::released_through`]).
    unrecorded: BTreeSet<u64>,
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

/// The events a host that trusts nobody refuses unless it was told to take them:
/// `record_list.floor` is the word that an instant is settled, on which a copy forgets the
/// tombstones up to it, and a copy that forgot a tombstone takes a write it named in again. It
/// carries no settlement by the members, so a host that relays it could settle an instant its
/// copies have not, and bring back with the old signed writes what was deleted. A host that trusts
/// the one that settles (its own hub) allows it.
pub const UNSIGNED_REFUSED: [&str; 1] = [FLOOR_EVENT];

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
                awaiting_own_chain: false,
                waiting: Vec::new(),
                checkpoints: BTreeMap::new(),
                changes: BTreeMap::new(),
                ops: BTreeMap::new(),
                evicted: Vec::new(),
                restored: Vec::new(),
                epoch_keys: BTreeMap::new(),
                keyed_by_a_group: false,
                unsigned_refused: UNSIGNED_REFUSED.iter().map(|e| e.to_string()).collect(),
                delivered: BTreeMap::new(),
                unopened: BTreeMap::new(),
                unrecorded: BTreeSet::new(),
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
    /// different entries at one position are a fork, which freezes the host: it takes no change or
    /// settled instant until an administrator resolves it ([`Self::forked`]).
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

    /// The device's secrets this host holds, as the contents of a copy of them ([`crate::backup`]):
    /// its person (the node, and the seed of their key, which is read nowhere else) and the keys of
    /// the epochs it holds, none of an epoch the list had destroyed. A copy under a passphrase and
    /// the state a device seals under its platform's key are made from it. `None` when the host has
    /// no person.
    pub fn secrets(&self) -> Option<crate::backup::Contents> {
        let contents = {
            let state = self.state();
            let (node, identity) = state.own.as_ref()?;
            crate::backup::Contents::of_identity(&state.calendar, *node, identity)
        };
        Some(contents.with_epoch_keys(self))
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

    /// An administrator's entry of the log: drafted after the head (the genesis, when the log holds
    /// none), signed by `administrator`, appended here, and returned for the relay and every other
    /// host. An entry that needs more signatures than one (a change to the administrators under a
    /// quorum above one) is collected across hosts with [`Self::draft`] instead: here the log refuses
    /// it, and nothing is appended.
    pub fn administer(&self, administrator: &Identity, body: Body) -> Result<Entry, LogError> {
        let entry = self.draft(body).sign(administrator).finish();
        self.append(&entry)?;
        Ok(entry)
    }

    /// The grant that makes the device of `node` a member with `role` that signs with `key`. It
    /// carries where the device's chain ends (`final_seq`, the last operation of it this host has
    /// admitted, [`Self::last_seq`]) when it takes writing away from a device that could write or
    /// gives it another key, which the log requires of such a grant, and nothing otherwise.
    pub fn grant_body(&self, node: u64, role: u8, key: PublicKey) -> Body {
        let takes_writing_away = self.state().log.latest().any(|(held, era)| {
            held == node && era.role >= WRITER && (role < WRITER || era.key != key)
        });
        Body::Grant {
            node,
            role,
            key,
            final_seq: takes_writing_away.then(|| self.last_seq(node)),
        }
    }

    /// The grant that takes the device of `node` off, ending its chain at the last operation of it
    /// this host has admitted; `None` when the log makes it no member now.
    pub fn removal_body(&self, node: u64) -> Option<Body> {
        let key = self
            .current_members()
            .into_iter()
            .find(|(held, _)| *held == node)
            .map(|(_, key)| key)?;
        Some(Body::Grant {
            node,
            role: 0,
            key,
            final_seq: Some(self.last_seq(node)),
        })
    }

    /// The hash of the list's genesis entry, which a person is invited with; `None` while the log
    /// holds no entry.
    pub fn genesis(&self) -> Option<Hash> {
        self.state().log.hash_at(0)
    }

    /// The hash of the genesis this host holds, or, before it holds one, the one it was invited
    /// with: what an invitation of this list names ([`crate::invitation`]).
    pub fn genesis_taken(&self) -> Option<Hash> {
        self.state().log.genesis_taken()
    }

    /// Whether the log has ever granted the device of `node` a role, a removal included: an
    /// invitation is for a node it never granted.
    pub fn has_granted(&self, node: u64) -> bool {
        self.state().log.has_granted(node)
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

    /// This host's person, as [`Self::set_identity`] gave it: their node and their public key.
    pub fn person(&self) -> Option<(u64, PublicKey)> {
        self.state()
            .own
            .as_ref()
            .map(|(node, identity)| (*node, identity.public()))
    }

    /// The name of the list this host holds.
    pub fn list(&self) -> String {
        self.state().calendar.clone()
    }

    /// This host's person's signature of a login at the relay named `audience`, against the relay's
    /// `nonce` that expires at `expires_at_ms` (`crate::login`), and the node it is for; `None` when
    /// the host has no person. The audience is the one the device was configured with, never the
    /// one a challenge said.
    pub fn sign_login(
        &self,
        audience: &str,
        nonce: &[u8],
        expires_at_ms: u64,
    ) -> Option<(u64, [u8; 64])> {
        let state = self.state();
        let (node, identity) = state.own.as_ref()?;
        let bytes =
            crate::login::signing_bytes(audience, &state.calendar, *node, nonce, expires_at_ms);
        Some((*node, identity.sign(&bytes)))
    }

    /// The entry of the log that made the latest grant of `node`, when the host holds one: what a
    /// device brings to a relay it logs in at, which may not have been given it yet.
    pub fn latest_grant(&self, node: u64) -> Option<Entry> {
        let state = self.state();
        let index = state
            .log
            .latest()
            .find(|(held, _)| *held == node)
            .map(|(_, era)| era.index)?;
        state
            .log
            .entries()
            .get(usize::try_from(index).ok()?)
            .cloned()
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

    /// Hold this host's person's signing until their own chain is read back from where the device
    /// published it: what a host calls when the state it holds came back from a store that may be
    /// older than the device's last operation (a save, or a copy of the device's secrets and no
    /// state at all). A device that signed operations 1 to 7 and came back with a state that holds
    /// 1 to 5 would sign its next operation at 6, a second operation at a number the hosts that hold
    /// the first take as the device signing two histories, and freeze it for. What the host holds
    /// already still leaves as it is ([`Self::seal_sent`]); only a new operation waits, and
    /// [`Self::confirm_own_chain`] ends the wait.
    pub fn await_own_chain(&self) {
        self.state().awaiting_own_chain = true;
    }

    /// This host's list is keyed by a group its device is in (an MLS group, `sce-rust-mls`): its
    /// person's writes are sealed from the first, and none is signed while the log names no epoch.
    /// The log alone decides whether the content of a write that arrives is sealed (from the entry
    /// that begins the first epoch), and a list keyed by its group names its first epoch when it is
    /// opened, before anything is written; what this adds is the device's own side of it, for a
    /// log that names no epoch yet while the group exists (a list opened by a core whose opening
    /// named none): a write of this host's person waits ([`Sent::Waiting`]) until the log names an epoch
    /// and this host holds its key, and is not signed in the clear. It is the host's to say, since
    /// only the host knows it holds the group's device; nothing that arrives is judged by it.
    pub fn keyed_by_a_group(&self) {
        self.state().keyed_by_a_group = true;
    }

    /// Whether this host's person signs nothing new until their own chain is read back
    /// ([`Self::await_own_chain`]).
    pub fn awaits_own_chain(&self) -> bool {
        self.state().awaiting_own_chain
    }

    /// What `published`, the heads a relay holds (its answer to `Heads`), names of this host's
    /// person's own chain past the highest number of it this host holds: the device's node and the
    /// first and last number to read back. `None` when the host holds as much of the chain as the
    /// relay does, or has no person.
    pub fn own_chain_behind(&self, published: &[Head]) -> Option<(u64, u64, u64)> {
        own_chain_behind(&self.state(), published)
    }

    /// Where a device that waits for its own chain ([`Self::await_own_chain`]) reads it back from:
    /// its node, and the number after the last at which `published` (the heads a relay holds) and
    /// this host agree. That is the number after the lower of the relay's head of the device and the
    /// highest number of the device's chain this host holds, or the relay's head itself when the
    /// relay names there an operation of the device this host does not hold. The relay may hold
    /// more of the chain from there than its heads say: a head stops below the first conflict
    /// among a device's operations and at the first gap, and what the device signed past either is
    /// what a device restored from an older store would otherwise sign again. `None` when the host
    /// has no person.
    pub fn own_chain_walk(&self, published: &[Head]) -> Option<(u64, u64)> {
        own_chain_walk(&self.state(), published)
    }

    /// The chain this host holds of its person reaches `published`, the heads a relay holds: the
    /// wait [`Self::await_own_chain`] began ends, and the device signs again from the number after
    /// the last it holds. `false`, and the wait goes on, while the relay holds more of the chain
    /// than this host does ([`Self::own_chain_behind`]). A host with no person waits for nothing.
    ///
    /// What it rests on: the relay's word of how far the device's chain goes. A relay that withholds
    /// the device's own operations from the heads it gives as well as from what it hands back makes
    /// the device sign at a number it used, and every host that is given both holds the proof and
    /// freezes the device; a relay that names operations and does not give them keeps the device
    /// waiting, and another relay, or a member, gives them.
    pub fn confirm_own_chain(&self, published: &[Head]) -> bool {
        let mut state = self.state();
        if own_chain_behind(&state, published).is_some() {
            return false;
        }
        state.awaiting_own_chain = false;
        true
    }

    /// The devices this host holds conflicting operations of, and the number of the first conflict:
    /// two operations at one number, one that does not follow the one before it, a change whose time
    /// goes back along the chain or whose dot another change has. Proof that the device's chain
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

    /// The operations this host's machine is to be told now, each as the event and the data the
    /// machine is told (the content opened), in the order it is to be told them: causal delivery
    /// (docs/adr/0085, section 3; Birman, Schiper and Stephenson 1991). An operation of a device
    /// whose machine was told up to `D` of it is told when it is the device's next, `D + 1`, and
    /// the machine was told every operation its context names; one below the next was told
    /// already and is not told again; one past the next, or whose context names an operation the
    /// machine was not told, waits here until it is, and is never skipped ([`Self::lacking`] says
    /// what to ask for). The order told is a linear extension of the contexts, whatever order the
    /// operations were taken in.
    ///
    /// What lies at or past where a device's chain is frozen is told to no machine, and nor is what
    /// the log puts past the last operation the administrators accepted of a device that may write
    /// no more: a context that names such an operation is met once the machine was told the device
    /// up to that point, since waiting for the rest would wait for good.
    pub fn deliver(&self) -> Vec<(String, String)> {
        let mut state = self.state();
        let mut vector = state.delivered.clone();
        let told = deliver_from(&state, &mut vector);
        state.delivered = vector;
        told.into_iter().map(|one| (one.event, one.data)).collect()
    }

    /// Every operation this host holds that a machine told up to `after` (a stable cut its host
    /// folded its journal at, docs/adr/0085, section 4.3; empty for the whole history) is to be
    /// told, in the causal order [`Self::deliver`] tells them in, each with its tag read from its
    /// link, the device's own changes included; then what this host's person sent that waits to be
    /// signed, in the order it was sent, with no tag: what a host recalls into a machine built
    /// again (docs/adr/0083, section 2.2), a change that waits recalled as provisional
    /// (docs/adr/0093, section 6.1). It reads no clock, asks nothing, and moves nothing: what a
    /// round tells the machine is told by [`Self::deliver`] as before. A mark is passed over, as
    /// [`Self::deliver`] passes it.
    pub fn history(&self, after: &[(u64, u64)]) -> Vec<Told> {
        let state = self.state();
        let mut vector: BTreeMap<u64, u64> = after.iter().copied().collect();
        let mut history = deliver_from(&state, &mut vector);
        history.extend(state.waiting.iter().map(|send| Told {
            event: send.event.to_string(),
            data: send.data.clone(),
            tag: None,
        }));
        history
    }

    /// The stable cut (docs/adr/0085, sections 3 and 3.1): for each device this host's machine was
    /// told anything of, the highest number every member of the list has been told of it, lowest
    /// node first, a device of which some member was told nothing left out. What a member was told
    /// is read from its latest operation this host holds: its causal context, and its own number
    /// for its own column (a mark when it wrote nothing since, [`Self::sign_mark`]); a member of
    /// which this host holds no operation was told nothing, and holds every column at 0 until it
    /// writes or marks. What this host's own machine was told bounds it too, so a host never folds
    /// past what its own copy holds. A host folds its journal at the cut (docs/adr/0085, section
    /// 4.3), and recalls only what comes after it ([`Self::history`]).
    pub fn stable_cut(&self) -> Vec<(u64, u64)> {
        let state = self.state();
        let own = state.own.as_ref().map(|(node, _)| *node);
        let mut vectors: Vec<BTreeMap<u64, u64>> = vec![state.delivered.clone()];
        for (node, era) in state.log.latest() {
            if era.role >= READER && Some(node) != own {
                vectors.push(known_vector(&state, node));
            }
        }
        state
            .delivered
            .keys()
            .filter_map(|node| {
                let low = vectors
                    .iter()
                    .map(|vector| vector.get(node).copied().unwrap_or(0))
                    .min()
                    .unwrap_or(0);
                (low > 0).then_some((*node, low))
            })
            .collect()
    }

    /// Sign a mark of what this host's machine has been told, when it has been told a change of
    /// another device that the latest operation of this host's person does not say it was
    /// (docs/adr/0085, section 3.1): the number the mark takes, or `None` when none is owed or the
    /// person may not sign one now. A mark carries no change and is told to no machine; its
    /// context is the vector, and the stable cut moves by it ([`Self::stable_cut`]); the next round
    /// gives it to the relay. A mark is owed for a change told and not for a mark told, so two
    /// members that only mark do not answer each other's marks for ever. Every member may sign one,
    /// a reader included; a device that is no member, that is frozen, or that waits for its own
    /// chain signs none. When to ask is the host's: "a member that has not written for a while" is
    /// time, which the trust does not read. A mark is no record of a host's journal: it carries
    /// nothing a host keeps, and a mark lost in a crash is read back with the chain
    /// ([`Self::await_own_chain`]).
    pub fn sign_mark(&self) -> Option<u64> {
        let mut state = self.state();
        let node = state.own.as_ref().map(|(node, _)| *node)?;
        if state.forked.is_some() || !mark_owed(&state, node) || !may_mark(&state, node) {
            return None;
        }
        let travelling = json!({ "by": node.to_string() });
        make_operation(&mut state, MARK_EVENT, travelling, node, None)
            .ok()
            .map(|wire| wire.link.seq)
    }

    /// The operations of this host's person's chain signed for a change its host keeps a journal of
    /// ([`Self::keep_made`]) that the host has not said it recorded, in the order of their numbers,
    /// each with its number, its event and the data it travels with: exactly what the relay is
    /// given (docs/adr/0083, section 3.1). They stay until [`Self::recorded`] says so, and none of
    /// them, nor any operation of the chain after the first of them, is given to a relay before.
    pub fn unrecorded(&self) -> Vec<OwnRecord> {
        let state = self.state();
        let Some(chain) = state.own.as_ref().and_then(|(node, _)| state.ops.get(node)) else {
            return Vec::new();
        };
        state
            .unrecorded
            .iter()
            .filter_map(|seq| {
                let held = chain.range(*seq, *seq).next()?;
                Some(OwnRecord {
                    seq: *seq,
                    event: held.event.to_string(),
                    data: held.wire.to_json()?,
                })
            })
            .collect()
    }

    /// The host recorded every operation of its person's chain up to `through` in its journal:
    /// they may be given to a relay ([`Self::released_through`]).
    pub fn recorded(&self, through: u64) {
        self.state().unrecorded.retain(|seq| *seq > through);
    }

    /// The highest number of this host's person's chain a relay may be given, when one is held
    /// back: the number before the first operation signed for a change the host has not said it
    /// recorded ([`Self::unrecorded`]). `None` when nothing is held back.
    pub fn released_through(&self) -> Option<u64> {
        self.state()
            .unrecorded
            .first()
            .map(|lowest| lowest.saturating_sub(1))
    }

    /// Take back the records a host's journal holds of its person's own operations, past what the
    /// state it restored holds (docs/adr/0083, section 3.4): each `(event, data)` as
    /// [`Self::unrecorded`] gave it, verified as an operation from anyone is (its signature, its
    /// place in the chain, the entry of the log it names) and held as the person's own, recorded
    /// already, its machine counted as told it, since the machine made it. Returns how many it did
    /// not hold. A record that is not an operation of this host's person, or that does not hold,
    /// is refused with why, and the records before it are kept.
    pub fn restore_records(&self, records: &[(String, String)]) -> Result<usize, String> {
        let mut state = self.state();
        let own = state
            .own
            .as_ref()
            .map(|(node, _)| *node)
            .ok_or_else(|| "this host has no person".to_string())?;
        let mut kept = 0;
        for (event, data) in records {
            let event = signed_name(event)
                .ok_or_else(|| format!("{event}: it is not an operation of a chain"))?;
            let taken = take(&mut state, event, data).map_err(|error| match error {
                TakeError::NotReached(reason)
                | TakeError::NoKey(reason)
                | TakeError::Refused(reason) => reason,
            })?;
            if taken.node != own {
                return Err(format!(
                    "{event}: operation {} is node {}'s, and a host's journal holds its own person's alone",
                    taken.seq, taken.node
                ));
            }
            state.unrecorded.remove(&taken.seq);
            let told = state.delivered.entry(own).or_insert(0);
            *told = (*told).max(taken.seq);
            kept += usize::from(taken.new);
        }
        Ok(kept)
    }

    /// The highest stamp of a change of this host's person's in the chain it holds, the proof
    /// included (docs/adr/0093, section 4.3): what a new stamp of the device's must exceed. `None`
    /// when it holds none, or has no person.
    pub fn own_stamp(&self) -> Option<(i64, u64)> {
        let state = self.state();
        let (node, _) = state.own.as_ref()?;
        state
            .ops
            .get(node)?
            .all()
            .filter_map(|held| held.stamp)
            .max()
    }

    /// The machine lost what it was told (it is built again, and every operation is to be told
    /// again): the delivered vector starts over, its own column included, and the next
    /// [`Self::deliver`] tells the machine every operation this host holds, in causal order.
    pub fn forget_delivered(&self) {
        self.state().delivered.clear();
    }

    /// What this host's machine has been told of each device: its node and the highest number
    /// told, for every device it was told anything of, lowest node first.
    pub fn delivered(&self) -> Vec<(u64, u64)> {
        self.state()
            .delivered
            .iter()
            .filter(|(_, seq)| **seq > 0)
            .map(|(node, seq)| (*node, *seq))
            .collect()
    }

    /// The operations this host does not hold that one it holds waits on before its machine may be
    /// told it, as ranges (node, from, to) of a device's chain, lowest node first: the gap before a
    /// device's next operation held, and what the context of each device's next operation names
    /// that the host does not hold. What a host asks a relay for, so that an operation lost on its
    /// way is repaired and never skipped (docs/adr/0085, section 3).
    pub fn lacking(&self) -> Vec<(u64, u64, u64)> {
        let state = self.state();
        let mut wanted: BTreeMap<u64, (u64, u64)> = BTreeMap::new();
        let mut want = |node: u64, from: u64, to: u64| {
            if from <= to {
                let range = wanted.entry(node).or_insert((from, to));
                range.0 = range.0.min(from);
                range.1 = range.1.max(to);
            }
        };
        for (node, chain) in &state.ops {
            let next = delivered_through(&state, *node) + 1;
            if chain.frozen().is_some_and(|from| next >= from) {
                continue;
            }
            match chain.at(next) {
                // The next is held: what its context names that the host does not hold.
                Some(held) => {
                    for (other, seq) in &held.wire.link.context {
                        if dependency_met(&state, *other, *seq) {
                            continue;
                        }
                        let to = told_ceiling(&state, *other).map_or(*seq, |top| top.min(*seq));
                        let from =
                            first_not_held(&state, *other, delivered_through(&state, *other) + 1);
                        want(*other, from, to);
                    }
                }
                // The next is not held while a later one is: the gap before it.
                None => {
                    if let Some(later) = chain.range(next, u64::MAX).next() {
                        want(*node, first_not_held(&state, *node, next), later.seq() - 1);
                    }
                }
            }
        }
        wanted
            .into_iter()
            .map(|(node, (from, to))| (node, from, to))
            .collect()
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
    /// raise: a `record_list.role` for each node the log has a grant for, a member that was taken
    /// off included, so that a role the machine held is overwritten.
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
    /// with the checkpoints it made or took in, and every operation it holds (the changes it
    /// made, took in and relays, every number of every chain), each as the
    /// event and the data it goes on the wire with, what its machine sent that waits to be
    /// signed ([`Self::keep_sent`]), in order, as the machine sent it and with whether its host
    /// keeps a journal of it, and the numbers of the records its host has not recorded
    /// ([`Self::unrecorded`]). It holds no key: a host keeps those where it keeps its person's
    /// secret.
    pub fn to_text(&self) -> String {
        let state = self.state();
        json!({
            "checkpoints": state.checkpoints.values().map(Checkpoint::to_value).collect::<Vec<_>>(),
            "operations": state.ops.values()
                .flat_map(Chain::all)
                .filter_map(|held| Some(json!([held.event, held.wire.to_json()?])))
                .collect::<Vec<_>>(),
            "waiting": state.waiting.iter()
                .map(|send| json!([send.event, send.data, send.journalled]))
                .collect::<Vec<_>>(),
            "delivered": state.delivered.iter()
                .filter(|(_, seq)| **seq > 0)
                .map(|(node, seq)| json!([node.to_string(), seq.to_string()]))
                .collect::<Vec<_>>(),
            "unrecorded": state.unrecorded.iter()
                .map(u64::to_string)
                .collect::<Vec<_>>(),
        })
        .to_string()
    }

    /// Take back what [`Self::to_text`] wrote: each checkpoint and each operation that today's log
    /// verifies is kept, and the rest are dropped, since what a host reads back from its own
    /// storage is not taken for true. The chain of this host's own person is what was kept of it,
    /// so that what it writes next is the next one. What the machine sent that waited to be signed
    /// waits again, in its order; it is judged when it is signed, as any send is. Returns how many
    /// operations and checkpoints were kept.
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
            let Some(event) = signed_name(event) else {
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
        for row in rows("waiting") {
            let (Some(event), Some(data)) = (
                row.get(0).and_then(Value::as_str),
                row.get(1).and_then(Value::as_str),
            ) else {
                continue;
            };
            // Whether its host keeps a journal of it: a row written before the outbox holds none,
            // and is a send of a host that keeps none.
            let journalled = row.get(2).and_then(Value::as_bool).unwrap_or(false);
            let Some(send) = Waiting::of(&state.calendar, event, data, journalled) else {
                continue;
            };
            if !state.waiting.iter().any(|held| held.is_the_same(&send)) {
                state.waiting.push(send);
            }
        }
        // The operations of the person's chain signed for a change the host keeps a journal of and
        // has not recorded: held back from the relay still. A number this host no longer holds an
        // operation at (the chain did not verify under today's log) holds nothing back.
        for row in rows("unrecorded") {
            let Some(seq) = row.as_str().and_then(|text| text.parse::<u64>().ok()) else {
                continue;
            };
            if own_chain_holds(&state, seq) {
                state.unrecorded.insert(seq);
            }
        }
        // What the machine was told, as it was when the text was written: the machine the host
        // keeps beside this text was told that much. A text written before the host kept it holds
        // none, and the next delivery tells the machine every operation held again, which the list
        // takes in once each.
        for row in rows("delivered") {
            let number = |at: usize| {
                row.get(at)
                    .and_then(Value::as_str)
                    .and_then(|text| text.parse::<u64>().ok())
            };
            if let (Some(node), Some(seq)) = (number(0), number(1)) {
                let told = state.delivered.entry(node).or_insert(0);
                *told = (*told).max(seq);
            }
        }
        kept
    }
}

/// Whether this host holds an operation of its person's own chain at `seq`.
fn own_chain_holds(state: &State, seq: u64) -> bool {
    state
        .own
        .as_ref()
        .and_then(|(node, _)| state.ops.get(node))
        .is_some_and(|chain| chain.range(seq, seq).next().is_some())
}

/// What `published` holds of this host's person's chain past the highest number of it the host
/// holds (the proof of a conflict included, so that a device frozen by one does not read back what
/// it holds already): the node, and the first and last number of it.
fn own_chain_behind(state: &State, published: &[Head]) -> Option<(u64, u64, u64)> {
    let node = state.own.as_ref()?.0;
    let theirs = published.iter().find(|head| head.node == node)?.seq;
    let ours = state
        .ops
        .get(&node)
        .and_then(|chain| chain.all().next_back())
        .map_or(0, Held::seq);
    (theirs > ours).then_some((node, ours + 1, theirs))
}

/// What [`Trust::own_chain_walk`] says, on the state the caller holds locked.
fn own_chain_walk(state: &State, published: &[Head]) -> Option<(u64, u64)> {
    let node = state.own.as_ref()?.0;
    let chain = state.ops.get(&node);
    let ours = chain
        .and_then(|chain| chain.all().next_back())
        .map_or(0, Held::seq);
    let named = published.iter().find(|head| head.node == node);
    let theirs = named.map_or(0, |head| head.seq);
    // The relay names, at a number this host holds of the chain, an operation it does not hold
    // there: the device signed another there, and the relay holds it.
    let differs = named
        .filter(|head| {
            head.seq <= ours && !chain.is_some_and(|chain| chain.has(head.seq, &head.id))
        })
        .map(|head| head.seq);
    Some((
        node,
        differs.unwrap_or_else(|| theirs.min(ours).saturating_add(1)),
    ))
}

/// What the machine is told of `node`: the number and id of each operation, in order.
fn told_of(state: &State, node: u64) -> Vec<(u64, [u8; 32])> {
    state
        .ops
        .get(&node)
        .map(|chain| chain.view().map(|held| (held.seq(), held.id)).collect())
        .unwrap_or_default()
}

/// Put what the host keeps changes by in line with what the machine is told of `node`: an
/// operation that is not told is not what the host sends a change on as, and one that is, is.
fn sync_tables(state: &mut State, node: u64) {
    let Some(chain) = state.ops.get(&node) else {
        return;
    };
    let signatures: std::collections::BTreeSet<[u8; 64]> =
        chain.all().map(|held| held.wire.signature).collect();
    let told: Vec<(Vec<u8>, Wire)> = chain
        .view()
        .map(|held| (held.key.clone(), held.wire.clone()))
        .collect();
    state
        .changes
        .retain(|_, wire| !signatures.contains(&wire.signature));
    for (key, wire) in told {
        state.changes.entry(key).or_insert(wire);
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
        // What was held and is not now is not what the host sends a change on as, told or not.
        if !gone.is_empty() {
            let signatures: Vec<[u8; 64]> = gone.iter().map(|(_, _, sig)| *sig).collect();
            state
                .changes
                .retain(|_, wire| !signatures.contains(&wire.signature));
        }
        tell_what_changed(state, node, &before, None);
    }
}

/// What putting an operation among its device's did ([`hold`]): whether the host did not hold it
/// before, whether the machine is told it now, and how it conflicts with what was held, when it does.
struct Placed {
    new: bool,
    applied: bool,
    conflict: Option<Conflict>,
}

/// The conflict `inserted` says an operation of a chain made, with the operations it conflicts
/// with as they travel.
fn conflict_of(chain: &Chain, inserted: &chain::Inserted) -> Option<Conflict> {
    let found = inserted.conflict.as_ref()?;
    Some(Conflict {
        at: found.at,
        before: inserted.before,
        after: inserted.after.unwrap_or(found.at),
        with: found
            .with
            .iter()
            .filter_map(|(seq, id)| {
                let other = chain.find(*seq, id)?;
                Some((other.event.to_string(), other.wire.to_json()?))
            })
            .collect(),
    })
}

/// Put an operation among its device's: the one place that decides what the machine is told of a
/// device.
fn hold(state: &mut State, node: u64, held: Held) -> Placed {
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
        return Placed {
            new: false,
            applied: told,
            conflict: None,
        };
    }
    let conflict = conflict_of(chain, &inserted);
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
            .map(|one| (one.key.clone(), one.wire.clone()));
        if let Some((key, wire)) = table_entry {
            state.changes.entry(key).or_insert(wire);
        }
    }
    Placed {
        new: true,
        applied: told,
        conflict,
    }
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

/// What a machine told up to `vector` was told of `node`: the highest number, 0 for nothing.
fn through_in(vector: &BTreeMap<u64, u64>, node: u64) -> u64 {
    vector.get(&node).copied().unwrap_or(0)
}

/// What this host's machine has been told of `node`: the highest number, 0 for nothing.
fn delivered_through(state: &State, node: u64) -> u64 {
    through_in(&state.delivered, node)
}

/// What a machine told up to `vector` is to be told now, in an order that is a linear extension of
/// the contexts, each with its tag; `vector` moves past every operation told, and past every one
/// passed over (docs/adr/0085, section 3: the rule, the same for a round and for a history).
fn deliver_from(state: &State, vector: &mut BTreeMap<u64, u64>) -> Vec<Told> {
    let mut told = Vec::new();
    loop {
        let mut moved = false;
        let nodes: BTreeSet<u64> = state
            .ops
            .keys()
            .chain(state.unopened.keys())
            .copied()
            .collect();
        for node in nodes {
            while let Some(next) = next_deliverable(state, vector, node) {
                let column = vector.entry(node).or_insert(0);
                *column += 1;
                let seq = *column;
                if let Next::Tell {
                    event,
                    data,
                    context,
                } = next
                {
                    told.push(Told {
                        event: event.to_string(),
                        data,
                        tag: Some(Tag {
                            author: node,
                            seq,
                            context,
                        }),
                    });
                }
                moved = true;
            }
        }
        if !moved {
            return told;
        }
    }
}

/// What is known of what the member `node`'s machine has been told: the causal context of its
/// latest operation this host holds, with that operation's own number as its own column; nothing
/// when this host holds none of it.
fn known_vector(state: &State, node: u64) -> BTreeMap<u64, u64> {
    let Some(last) = state.ops.get(&node).and_then(Chain::last) else {
        return BTreeMap::new();
    };
    let mut vector: BTreeMap<u64, u64> = last.wire.link.context.iter().copied().collect();
    vector.insert(node, last.seq());
    vector
}

/// The highest number of `node`'s chain up to `through` that is a change, or one passed over as
/// sealed under an epoch this host never opens: what a mark is owed for. A mark told is not.
fn changes_through(state: &State, node: u64, through: u64) -> u64 {
    if through == 0 {
        return 0;
    }
    let held = state
        .ops
        .get(&node)
        .and_then(|chain| {
            chain
                .range(1, through)
                .rev()
                .find(|held| held.event != MARK_EVENT)
        })
        .map_or(0, Held::seq);
    let passed = state
        .unopened
        .get(&node)
        .and_then(|seqs| seqs.range(..=through).next_back())
        .map_or(0, |(seq, _)| *seq);
    held.max(passed)
}

/// Whether this host's person `own` owes a mark: its machine was told a change of another device
/// that the context of its latest operation does not name.
fn mark_owed(state: &State, own: u64) -> bool {
    let said: BTreeMap<u64, u64> = state
        .ops
        .get(&own)
        .and_then(Chain::last)
        .map(|last| last.wire.link.context.iter().copied().collect())
        .unwrap_or_default();
    state.delivered.iter().any(|(node, through)| {
        *node != own && changes_through(state, *node, *through) > through_in(&said, *node)
    })
}

/// Whether this host's person `own` may sign a mark now: the log the host holds names them a
/// member under the key the device signs with, at any role, and the device neither waits for its
/// own chain nor is frozen ([`make_operation`] says the last two again).
fn may_mark(state: &State, own: u64) -> bool {
    let Some((_, identity)) = state.own.as_ref() else {
        return false;
    };
    let Some((tip, _)) = state.log.tip() else {
        return false;
    };
    !state.awaiting_own_chain
        && state
            .log
            .era_for(own, tip)
            .is_some_and(|era| era.role >= READER && era.key == identity.public())
}

/// The highest number of `node`'s chain any machine may ever be told, when one is known: below
/// where the chain is frozen, and no further than the last operation the administrators accepted
/// of a device whose latest grant lets it write no more.
fn told_ceiling(state: &State, node: u64) -> Option<u64> {
    let frozen = state
        .ops
        .get(&node)
        .and_then(Chain::frozen)
        .map(|from| from.saturating_sub(1));
    let cut = state
        .log
        .latest()
        .find(|(granted, _)| *granted == node)
        .and_then(|(_, era)| (era.role < WRITER).then_some(era.cutoff).flatten());
    match (frozen, cut) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (one, other) => one.or(other),
    }
}

/// Whether the machine was told what a context's entry for `node` at `seq` names: every operation
/// of the device up to `seq`, or, when no machine will ever be told the device up to `seq`
/// ([`told_ceiling`]), every operation of it any machine will be told.
fn dependency_met(state: &State, node: u64, seq: u64) -> bool {
    dependency_met_in(state, &state.delivered, node, seq)
}

/// [`dependency_met`] for a machine told up to `vector`.
fn dependency_met_in(state: &State, vector: &BTreeMap<u64, u64>, node: u64, seq: u64) -> bool {
    let told = through_in(vector, node);
    told >= seq || told_ceiling(state, node).is_some_and(|top| top < seq && told >= top)
}

/// The first number of `node`'s chain from `from` on that this host holds no operation at, an
/// operation it holds and cannot open included: asking the relay for it again would bring the
/// same.
fn first_not_held(state: &State, node: u64, from: u64) -> u64 {
    let unopened = state.unopened.get(&node);
    let chain = state.ops.get(&node);
    let mut next = from;
    loop {
        let held = chain.is_some_and(|chain| chain.range(next, next).next().is_some());
        let sealed = unopened.is_some_and(|seqs| seqs.contains_key(&next));
        if !held && !sealed {
            return next;
        }
        next += 1;
    }
}

/// What becomes of the operation of a device after the highest its machine was told.
enum Next {
    /// It is told: its event, the data the machine is told, and its causal context.
    Tell {
        event: &'static str,
        data: String,
        context: Vec<(u64, u64)>,
    },
    /// It is passed over and told never: a mark, which is the trust layer's and no machine's; or
    /// an operation sealed under an epoch before this host's first, whose content this host was
    /// given no key of (`Trust::deliver`).
    Pass,
}

/// What becomes of the operation of `node` at the number after the highest a machine told up to
/// `vector` was told (docs/adr/0085, section 3), when anything does: told when it is held and
/// every operation its context names was told, and passed over then when it is a mark; passed
/// over when it cannot be opened here and never will ([`never_opened`]); nothing, and it waits,
/// otherwise. Never one past it.
fn next_deliverable(state: &State, vector: &BTreeMap<u64, u64>, node: u64) -> Option<Next> {
    let next = through_in(vector, node) + 1;
    let Some(held) = state.ops.get(&node).and_then(|chain| chain.at(next)) else {
        let epoch = state.unopened.get(&node)?.get(&next)?;
        return never_opened(state, *epoch).then_some(Next::Pass);
    };
    let met = held
        .wire
        .link
        .context
        .iter()
        .all(|(other, seq)| dependency_met_in(state, vector, *other, *seq));
    if !met {
        return None;
    }
    if held.event == MARK_EVENT {
        return Some(Next::Pass);
    }
    Some(Next::Tell {
        event: held.event,
        data: told_text(state, held)?,
        context: held.wire.link.context.clone(),
    })
}

/// Whether what is sealed under `epoch` is never opened by this host: an epoch before the first
/// one it holds the key of, which a member is given no history of (`TakeError::NoKey`). Such an
/// operation is never told to its machine, so what follows it in its chain is not held for it; a
/// host that holds no key at all holds everything for the key it waits for.
fn never_opened(state: &State, epoch: u32) -> bool {
    state
        .epoch_keys
        .keys()
        .next()
        .is_some_and(|first| epoch < *first)
}

/// An operation held as the machine is told it: its event as its author made it, the content
/// opened when this host keeps content from the hub, as [`Trust::take_operation`] tells it.
fn told_text(state: &State, held: &Held) -> Option<String> {
    let travelling: Value = serde_json::from_str(&held.wire.data).ok()?;
    let visible = open_change(state, &travelling, held.wire.link.log).ok()?;
    Some(visible.to_string())
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
        ROLE_EVENT.to_string(),
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
            if event == FLOOR_EVENT || message::SIGNED_EVENTS.contains(&event) {
                return Err(format!(
                    "{event}: the membership log forked at {index}, and nothing is taken in until an administrator resolves it"
                ));
            }
        }
        if event == FLOOR_EVENT {
            return admit_floor(&state, data);
        }
        if state.refuses_unsigned(event) {
            return Err(format!(
                "{event}: it carries no signature and this host does not take it on a peer's word"
            ));
        }
        // A mark travels between a device's core and a relay, and is told to no machine: a copy
        // on a Mesh route has its layer's own vectors (docs/adr/0085, section 3.2).
        if event == MARK_EVENT {
            return Err(format!(
                "{event}: a mark is the trust layer's, and no machine is told one"
            ));
        }
        let Some(event) = sent_name(event) else {
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
/// the first of them on), the event as the machine is told it, and, for an operation the host did
/// not hold that conflicts with what it held of its device, how (`conflict`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Taken {
    pub node: u64,
    pub seq: u64,
    pub new: bool,
    pub applied: bool,
    pub told: String,
    pub conflict: Option<Conflict>,
}

/// An operation as a history gives it ([`Trust::history`]): its event and the data a machine is
/// told, and its tag. A send of the host's person that waits to be signed has no tag: it has no
/// number yet, and its copy holds it provisionally (docs/adr/0093, section 6.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Told {
    pub event: String,
    pub data: String,
    pub tag: Option<Tag>,
}

impl Told {
    /// Whether it is a send of the host's person that waits to be signed: recalled provisionally.
    pub fn provisional(&self) -> bool {
        self.tag.is_none()
    }
}

/// What the delivery layer tagged a change with, read from its link (docs/adr/0085, section 3.1):
/// its dot, the author's node and its number in the author's chain, and its causal context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag {
    pub author: u64,
    pub seq: u64,
    pub context: Vec<(u64, u64)>,
}

/// An operation of a host's person's chain its host keeps a journal of and has not recorded
/// ([`Trust::unrecorded`]): its number, its event and the data it travels with, exactly what a
/// relay is given and what a restarted host gives back ([`Trust::restore_records`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnRecord {
    pub seq: u64,
    pub event: String,
    pub data: String,
}

/// How an operation taken in conflicts with the operations the host held of its device: two at one
/// number, one that does not follow the one before it or that the next does not follow, a change
/// whose time goes back along the chain or whose dot another change has. It is held as proof,
/// as every host that holds both holds it, and the device's chain is frozen from the lowest
/// position of any conflict among what is held.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Conflict {
    /// The lowest number at which it conflicts with an operation held.
    pub at: u64,
    /// Where the device's chain was frozen before it was taken in (`None`: it was not), and where
    /// it is frozen from now. They differ when this operation is what froze it there.
    pub before: Option<u64>,
    pub after: u64,
    /// The operations held that it conflicts with, each as the event and the data it travels with,
    /// in the order of their numbers: what a host given this one alone takes in to find the
    /// conflict itself.
    pub with: Vec<(String, String)>,
}

impl Conflict {
    /// Whether taking the operation in froze its device's chain where it was not frozen, or lower
    /// than it was.
    pub fn froze(&self) -> bool {
        self.before != Some(self.after)
    }
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
            let placed = hold_verified(state, event, verified, signature);
            Ok(Taken {
                node,
                seq,
                new: placed.new,
                applied: placed.applied,
                told,
                conflict: placed.conflict,
            })
        }
        Err(Refusal::NotReached(reason)) => Err(TakeError::NotReached(reason)),
        Err(Refusal::NoKey { reason, epoch, at }) => {
            // Its signature and its place hold, and only its content does not open here: kept, so
            // that the machine's delivery knows what sealed it ([`Trust::deliver`]).
            if let Some((node, seq)) = at {
                state.unopened.entry(node).or_default().insert(seq, epoch);
            }
            Err(TakeError::NoKey(reason))
        }
        Err(Refusal::Refused(reason)) => Err(TakeError::Refused(reason)),
    }
}

/// Hold an operation that was verified, whoever made it: whether it was new to the host, whether
/// the machine is told it, and how it conflicts with what was held.
fn hold_verified(
    state: &mut State,
    event: &'static str,
    verified: Verified,
    signature: [u8; 64],
) -> Placed {
    let prev = verified.link.prev;
    let wire = Wire {
        data: verified.travelling.to_string(),
        link: verified.link,
        signature,
    };
    let held = Held {
        id: verified.id,
        prev,
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
    /// It is sealed under `epoch`, whose key the host does not hold; `at` is its author and its
    /// number, once the operation is known to hold otherwise.
    NoKey {
        reason: String,
        epoch: u32,
        at: Option<(u64, u64)>,
    },
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
    /// The change's dot (its author and its stamp, [`message::Signing::subject`]), which every write
    /// of it is sealed under with its register, so that two changes under one dot are seen as the
    /// nonce used twice that they are.
    wid: Option<Vec<u8>>,
    travelling: Value,
    visible: Value,
    /// What the host keeps it under: the bytes the change is signed as when the machine holds it.
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
    // A mark changes nothing of the list, and every member signs one, a reader included: what a
    // member was told is what the stable cut is made of (docs/adr/0085, section 3.1).
    let mark = event == MARK_EVENT;
    if mark && era.role < READER {
        return Err(format!(
            "{event}: node {author} was no member at entry {} (role {})",
            link.log, era.role
        )
        .into());
    }
    if !mark && era.role < WRITER {
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
    // A context names what its author's machine had been told of other devices: never the author's
    // own column, which its number already orders, and only a device the log had granted by the
    // entry the operation names, since the author held no more of the log than that when it was
    // told the device's operations. A context naming a device no log granted would hold the
    // operation, and everything after it, waiting for good.
    for (node, _) in &link.context {
        if *node == author {
            return Err(format!("{event}: its context names its own author, node {author}").into());
        }
        if state.log.era_for(*node, link.log).is_none() {
            return Err(format!(
                "{event}: its context names node {node}, which the log had not granted by entry {}",
                link.log
            )
            .into());
        }
    }
    // Nothing is judged here of how the operation stands among the others of its device (a number
    // two are signed at, a chain that does not follow itself, a stamp that goes back or repeats): that
    // is what the set of them says, whatever order they were given in ([`Chain`]), and an operation
    // that conflicts is held as proof and not refused.

    // A mark has no stamp, no dot and no content: it is kept by its operation's bytes.
    if mark {
        return Ok(Verified {
            author,
            link,
            id,
            stamp: None,
            wid: None,
            travelling: value.clone(),
            visible: value,
            key: bytes,
        });
    }

    // A change's stamp is its author's clock when it made it, and what its writes are sealed under
    // is its dot: both are the author's own claims, which the chain checks.
    let stamp = message::stamp_of(&value).map_err(|error| format!("{event}: {error:?}"))?;

    let visible = open_change(state, &value, link.log).map_err(|refusal| match refusal {
        Refusal::NoKey { reason, epoch, .. } => Refusal::NoKey {
            reason,
            epoch,
            at: Some((author, link.seq)),
        },
        other => other,
    })?;
    let key = message::signing(&state.calendar, event, &visible)
        .and_then(Result::ok)
        .ok_or_else(|| format!("{event}: what it holds is not a change"))?
        .message;
    Ok(Verified {
        author,
        link,
        id,
        stamp: Some(stamp),
        wid: Some(signing.subject),
        travelling: value,
        visible,
        key,
    })
}

/// The change as the machine is told it: the content of each of its writes opened, when this host
/// keeps content from the hub, and the change as it is otherwise. Whether the list keeps content,
/// and which epoch a write may be sealed under, is judged at the entry of the log the change names
/// (`entry`): a device made it knowing the log up to there, and two hosts that were given the same
/// entries in another order judge it the same. A change that names an entry before the list began
/// to seal is plain, and holds whether the host took it before or after the entry that began the
/// epoch. A write-over carries no content, and is as it is. A change whose operation is of no kind
/// a change holds, or that writes one register twice, is no change, sealed or not.
fn open_change(state: &State, wire: &Value, entry: u64) -> Result<Value, Refusal> {
    let event = CHANGE_EVENT;
    let refused = |reason: String| Refusal::Refused(reason);
    let writes =
        message::writes_of(wire).map_err(|error| refused(format!("{event}: {error:?}")))?;
    let epoch_then = state.log.epoch_at(entry);
    let mut visible = wire.clone();
    for (at, stamp) in writes {
        let sealed = wire["ops"][at]
            .get("text")
            .and_then(Value::as_str)
            .unwrap_or("");
        if epoch_then == 0 {
            if secrecy::epoch_of(sealed).is_some() {
                return Err(refused(format!(
                    "{event}: its write {at} is sealed under an epoch, and the membership log it names has begun none"
                )));
            }
            continue;
        }
        let Some(epoch) = secrecy::epoch_of(sealed) else {
            return Err(refused(format!(
                "{event}: the content of its write {at} is not sealed, and the list keeps content from the hub"
            )));
        };
        if epoch == 0 || epoch > epoch_then {
            return Err(refused(format!(
                "{event}: its write {at} is sealed under epoch {epoch}, which the membership log it names has not begun"
            )));
        }
        // A hub that holds no key carries it as it is; a host that opens refuses what the list has
        // had destroyed the keys of, for good (it is not parked: no key will come).
        if state.log.is_shredded(epoch) && !state.relays_sealed() {
            return Err(refused(format!(
                "{event}: its write {at} is sealed under epoch {epoch}, whose keys the list had destroyed"
            )));
        }
        if state.relays_sealed() {
            continue;
        }
        let (value, text) = secrecy::open(&state.calendar, &state.epoch_keys, &stamp, sealed)
            .map_err(|error| match error {
                secrecy::OpenError::NotSealed => refused(format!(
                    "{event}: the content of its write {at} is not sealed, and this host keeps content from the hub"
                )),
                // A key this host does not hold yet (the group's commit has not reached it) or
                // never will (an epoch before it joined): the host asks again later, and does not
                // take the change for one that cannot hold.
                secrecy::OpenError::NoKey { epoch } => Refusal::NoKey {
                    reason: format!("{event}: this host holds no key of epoch {epoch}"),
                    epoch,
                    at: None,
                },
                secrecy::OpenError::Corrupt => {
                    refused(format!("{event}: the content of its write {at} does not open"))
                }
            })?;
        let object = visible["ops"][at]
            .as_object_mut()
            .ok_or_else(|| refused(format!("{event}: its write {at} is not an object")))?;
        object.insert("value".to_string(), Value::String(value.to_string()));
        object.insert("text".to_string(), Value::String(text));
    }
    Ok(visible)
}

/// A `record_list.floor` is admitted when the members settled it: its `cert` is the checkpoints of
/// every writer, each for this instant and this hash ([`verify_settlement`]). The machine is told
/// the floor without it. One with no settlement is admitted only when the host was told to take it
/// unsigned, and a settlement that does not hold is refused whatever the host was told.
fn admit_floor(state: &State, data: &str) -> Result<Option<String>, String> {
    let event = FLOOR_EVENT;
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
    key: Option<Vec<u8>>,
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
        return Err(chain_frozen(event, from));
    }
    // A device whose state came back from a store may have signed past what the store holds: the
    // number this would take may be one it used already, so nothing new is signed until its own
    // chain is read back from where it was published (`Trust::await_own_chain`).
    if state.awaiting_own_chain {
        return Err(awaiting_own_chain(event));
    }
    let (log, log_hash) = state.log.tip().ok_or_else(|| log_not_begun(event))?;
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
    // What the machine had been told of every other device when it made this: the operation is
    // told to no machine before all of that (docs/adr/0085, section 3.1). A send that waited to be
    // signed is given the context of when it is signed, which may name more than the machine had
    // seen when it made it; that only orders more than it needs to, and is never unsafe.
    let context = state
        .delivered
        .iter()
        .filter(|(node, seq)| **node != author && **seq > 0)
        .map(|(node, seq)| (*node, *seq))
        .collect();
    let link = Link {
        seq,
        prev,
        log,
        log_hash,
        context,
    };
    let signing = message::signing(&state.calendar, event, &travelling)
        .and_then(Result::ok)
        .ok_or_else(|| format!("{event}: the data is not what it names"))?;
    let bytes = message::operation_bytes(&signing.message, &link);
    let signature = identity.sign(&bytes);
    // A change has a stamp and a dot; a mark has neither, and is kept by its operation's bytes,
    // since one device's marks are one message and their links tell them apart.
    let (stamp, wid) = if event == MARK_EVENT {
        (None, None)
    } else {
        let stamp =
            message::stamp_of(&travelling).map_err(|error| format!("{event}: {error:?}"))?;
        (Some(stamp), Some(signing.subject))
    };
    let key = key.unwrap_or_else(|| bytes.clone());
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
    // What this device signs it signs once and in order: a change whose stamp goes back along its
    // chain, or whose dot another change of it has, is not signed. Its writes would be sealed under
    // the nonces of the other's, and what a relay knows of the one gives it the other.
    let conflict = state
        .ops
        .get(&author)
        .and_then(|chain| chain.probe(&held, &cutoffs));
    if let Some((at, why)) = conflict {
        return Err(match why {
            Why::Repeats => format!(
                "{event}: its dot names a change this device signed already (at {at}), and a dot is sealed under once"
            ),
            Why::GoesBack => format!(
                "{event}: its stamp goes back along this device's chain (a change at {at} has a later time)"
            ),
            Why::Forked => format!(
                "{event}: it would not follow the chain this device has signed (conflict at {at})"
            ),
        });
    }
    hold(state, author, held);
    // The machine made what its person signs, so it is told none of it again.
    let told = state.delivered.entry(author).or_insert(0);
    *told = (*told).max(seq);
    Ok(wire)
}

/// The name of an operation's event as the host keeps it, when `event` is one: a change or a mark.
fn signed_name(event: &str) -> Option<&'static str> {
    OPERATION_EVENTS.iter().copied().find(|name| *name == event)
}

/// The name of an event a machine sends that is signed, as the host keeps it, when `event` is one:
/// a change. A mark is the trust layer's own, and no machine sends one.
fn sent_name(event: &str) -> Option<&'static str> {
    message::SIGNED_EVENTS
        .iter()
        .copied()
        .find(|name| *name == event)
}

/// What an event the host's machine sent leaves as ([`Trust::seal_sent`]), and whether a new
/// operation of the host's person's chain was signed for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sealed {
    /// The data the event leaves with.
    pub data: String,
    /// Whether a new operation was signed and numbered for it. When not, the event leaves as what
    /// the host held already: the operation made or admitted before for the same change (the
    /// machine sent it again, as a copy given its own change back does), or a settled instant with
    /// the members' settlement.
    pub signed: bool,
}

impl Trust {
    /// What the host's machine sent, as it leaves, and whether that signed a new operation: the one
    /// place a send becomes an operation of the host's person's chain (the [`Sealer`] a host puts
    /// behind its machine is this, without the second part). `Ok(None)` for an event that is not
    /// signed.
    ///
    /// **A change is signed once, whole.** One change is one operation of its author's chain
    /// (docs/adr/0085, section 1), and what the host keeps a change by is the bytes it is signed
    /// as, its content opened: the same change sent again leaves as the operation the host holds,
    /// with the number and the signature it has, and nothing is signed. So a machine that sends a
    /// change again makes no operation and gives the relay nothing new. A change of the host's
    /// person under a dot (its author and its stamp) the device signed another change under is
    /// refused before anything is sealed or signed: one dot names one change, and a second content
    /// under it would be the device signing two things for one change, sealed under one nonce, which
    /// every host given both holds as proof and freezes the device for.
    pub fn seal_sent(&self, event: &str, data: &str) -> Result<Option<Sealed>, String> {
        seal_in(&mut self.state(), event, data)
    }
}

/// What [`Trust::seal_sent`] does, on the state the caller holds locked.
fn seal_in(state: &mut State, event: &str, data: &str) -> Result<Option<Sealed>, String> {
    if let Some(index) = state.forked {
        if event == FLOOR_EVENT || message::SIGNED_EVENTS.contains(&event) {
            return Err(log_forked(event, index));
        }
    }
    if event == FLOOR_EVENT {
        return Ok(seal_floor(state, data).map(|data| Sealed {
            data,
            signed: false,
        }));
    }
    if event == MARK_EVENT {
        return Err(format!(
            "{event}: a mark is signed by the trust layer itself, and no machine sends one"
        ));
    }
    if !message::SIGNED_EVENTS.contains(&event) {
        return Ok(None);
    }
    seal_change(state, data).map(Some)
}

/// Why nothing is sent: the membership log forked at `index`.
fn log_forked(event: &str, index: u64) -> String {
    format!(
        "{event}: the membership log forked at {index}, and nothing is sent until an administrator resolves it"
    )
}

/// Why nothing new is signed: the device's chain was shown to conflict at `from`.
fn chain_frozen(event: &str, from: u64) -> String {
    format!(
        "{event}: this device's chain conflicts at {from}, and it signs nothing more until an administrator has taken it off"
    )
}

/// Why nothing new is signed yet: the device's state came back from a store
/// ([`Trust::await_own_chain`]).
fn awaiting_own_chain(event: &str) -> String {
    format!(
        "{event}: this device's state came back from a store that may be behind what it signed, and it signs nothing new until a round has read its own chain back from the relay"
    )
}

/// Why nothing new is signed yet: the host holds no entry of the log.
fn log_not_begun(event: &str) -> String {
    format!("{event}: the membership log has not begun")
}

/// Whether this host's list is keyed by a group its device is in and the log names no epoch of it
/// yet ([`Trust::keyed_by_a_group`]): what its person's write waits for, and is not signed before.
fn names_no_epoch_of_its_group(state: &State) -> bool {
    state.keyed_by_a_group && state.log.epoch() == 0
}

/// Why a write is not signed yet: the list is keyed by its group and the log names no epoch.
fn no_epoch_of_the_group(event: &str) -> String {
    format!(
        "{event}: the list is keyed by its group and the membership log names no epoch of it yet, and a write is sealed from the first"
    )
}

/// Why a write is not sealed yet: the host holds no key of the epoch the log is in.
fn no_key_of_the_epoch(event: &str, epoch: u32) -> String {
    format!(
        "{event}: this host holds no key of epoch {epoch} yet, and writes nothing until it does"
    )
}

/// Why a change of `author`'s under `dot` (its author and its stamp) signed as `content` is not
/// signed: the device signed another change under that dot. One dot names one change, and a second
/// content under it would be the device signing two things for one change.
fn another_content_signed(
    state: &State,
    author: u64,
    dot: &[u8],
    content: &[u8],
) -> Option<String> {
    let at = state
        .ops
        .get(&author)
        .and_then(|chain| chain.stamped(dot))
        .filter(|other| other.key.as_slice() != content)
        .map(Held::seq)?;
    Some(format!(
        "{CHANGE_EVENT}: this device signed another content under this dot (its author and its stamp) at {at}, and one change is signed with one content"
    ))
}

/// A send of this host's machine that its person may not sign yet ([`Trust::keep_sent`]).
#[derive(Debug, Clone)]
struct Waiting {
    /// The event, as the host keeps the name of a signed one.
    event: &'static str,
    /// Its payload, as the machine sent it.
    data: String,
    /// The bytes it is signed as ([`message::Signing::message`]): one change waits once.
    content: Vec<u8>,
    /// Its dot ([`message::Signing::subject`]), which names one change.
    dot: Vec<u8>,
    /// The node of the person it names as its author.
    author: u64,
    /// Whether its host keeps a journal of it ([`Trust::keep_made`]): once it is signed, the
    /// operation is held back from the relay until the host says it recorded it.
    journalled: bool,
}

impl Waiting {
    /// The send `event` with `data` in the list `list`, when it is a signed event the data names
    /// whole; `None` otherwise. `journalled` says whether its host keeps a journal of it.
    fn of(list: &str, event: &str, data: &str, journalled: bool) -> Option<Self> {
        let event = sent_name(event)?;
        let value: Value = serde_json::from_str(data).ok()?;
        let signing = message::signing(list, event, &value)?.ok()?;
        Some(Self {
            event,
            data: data.to_string(),
            content: signing.message,
            dot: signing.subject,
            author: signing.author,
            journalled,
        })
    }

    /// Whether `other` is this send made again: the same change.
    fn is_the_same(&self, other: &Waiting) -> bool {
        self.event == other.event && self.content == other.content
    }
}

/// Whether this host's person may sign a new operation now ([`readiness`]).
enum Readiness {
    /// It may.
    Ready,
    /// Not yet, and why: a round that reads the log, the device's own chain or the key of an epoch
    /// can change it.
    NotYet(String),
    /// Not under the log the host holds, and why: the person may not write, or has no key the log
    /// grants, or the device is frozen, or there is no person.
    Never(String),
}

/// Whether this host's person may sign `event` as a new operation now: the person is a member the
/// log the host holds lets write, with the key the host signs with; the device is not waiting for
/// its own chain; for a list keyed by a group the host's device is in, the log names an epoch
/// ([`Trust::keyed_by_a_group`]); and, for a list that keeps its content, the host holds the key of
/// the epoch the log is in. A change is one unit, sealed whole under one epoch, so a change that
/// holds no write waits for the key as one that holds writes does. What a round can change is
/// [`Readiness::NotYet`]; what only a later entry of an administrator's could, and a device that
/// may never sign again, is [`Readiness::Never`].
fn readiness(state: &State, event: &str) -> Readiness {
    let Some((node, identity)) = state.own.as_ref() else {
        return Readiness::Never(format!("{event}: this host has no person"));
    };
    let node = *node;
    if let Some(index) = state.forked {
        return Readiness::Never(log_forked(event, index));
    }
    if let Some(from) = state.ops.get(&node).and_then(Chain::frozen) {
        return Readiness::Never(chain_frozen(event, from));
    }
    if state.awaiting_own_chain {
        return Readiness::NotYet(awaiting_own_chain(event));
    }
    let Some((tip, _)) = state.log.tip() else {
        return Readiness::NotYet(log_not_begun(event));
    };
    let key = identity.public();
    let Some(era) = state.log.era_for(node, tip) else {
        return Readiness::NotYet(format!(
            "{event}: the membership log grants node {node} nothing yet"
        ));
    };
    if era.key != key {
        let granted_before = state
            .log
            .eras()
            .any(|(held, eras)| held == node && eras.iter().any(|earlier| earlier.key == key));
        return if granted_before {
            Readiness::Never(format!(
                "{event}: the membership log gave node {node} another key than the one this device signs with"
            ))
        } else {
            Readiness::NotYet(format!(
                "{event}: the membership log grants the key this device signs with nothing yet"
            ))
        };
    }
    match era.role {
        role if role < WRITER => Readiness::Never(format!(
            "{event}: node {node} may not write (role {role} at entry {tip})"
        )),
        _ if names_no_epoch_of_its_group(state) => Readiness::NotYet(no_epoch_of_the_group(event)),
        _ if state.keeps_content() && !state.epoch_keys.contains_key(&state.log.epoch()) => {
            Readiness::NotYet(no_key_of_the_epoch(event, state.log.epoch()))
        }
        _ => Readiness::Ready,
    }
}

/// What became of a send of this host's machine given to [`Trust::keep_sent`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Sent {
    /// A new operation of the person's chain was signed for it.
    Signed,
    /// The host holds it already, as the operation made or admitted before for the same change (or
    /// a settled instant with the members' settlement): nothing is signed.
    Held,
    /// It is no event that is signed: nothing is kept.
    NotSigned,
    /// The person may not sign it yet, and why: it is kept, and signed in its turn as soon as the
    /// person may ([`Trust::sign_waiting`]).
    Waiting(String),
    /// It is refused, and why: nothing is kept.
    Refused(String),
}

impl Sent {
    fn of(sealed: Result<Option<Sealed>, String>) -> Self {
        match sealed {
            Ok(Some(sealed)) if sealed.signed => Sent::Signed,
            Ok(Some(_)) => Sent::Held,
            Ok(None) => Sent::NotSigned,
            Err(reason) => Sent::Refused(reason),
        }
    }
}

/// What [`Trust::sign_waiting`] did with the sends that waited, each as the machine sent it.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Waited {
    /// The sends signed now, each a new operation of the person's chain, in the order they were
    /// sent: each `(event, data)`. The relay is given each with the rest of what it lacks.
    pub taken: Vec<(String, String)>,
    /// The sends the host held already when their turn came, as the operation made or admitted
    /// before for the same change (the device's own chain, read back, held them): nothing
    /// was signed for them, and the relay is given nothing new. Each `(event, data)`.
    pub held: Vec<(String, String)>,
    /// The sends withdrawn for good, each `(event, data, why)`, in the order they were sent: the
    /// first that never will be signed (the log the host now holds says the person may not write:
    /// a reader, a device taken off, a key the log replaced; or it would conflict with what the
    /// device signed), and every send after it (docs/adr/0093, section 6.2). The machine took each
    /// in, provisionally or as made, and no copy but its own will hold it: the host withdraws them.
    pub dropped: Vec<(String, String, String)>,
    /// How many sends still wait after this, 0 when none does.
    pub waiting: usize,
    /// Why the first send that still waits does (a round that reads the log, the device's own chain
    /// or the key of an epoch can change it); `None` when none waits.
    pub why_waiting: Option<String>,
}

impl Waited {
    /// Whether nothing waited that came to anything: nothing was signed, found held or refused.
    pub fn is_empty(&self) -> bool {
        self.taken.is_empty() && self.held.is_empty() && self.dropped.is_empty()
    }
}

impl Trust {
    /// What the host's machine sent, kept for a relay: [`Self::seal_sent`] for a host whose sends
    /// a courier carries, where a send the person may not sign yet is kept and not refused.
    ///
    /// **A send that cannot be signed yet waits.** A machine sends whenever its person edits, and an
    /// application that works offline edits before its device has read the log that lets it
    /// write (an invited device before its first round, a device the list keys whose group's key
    /// it has not read yet), or before a device restored from a store has read its own chain back
    /// ([`Self::await_own_chain`]). Such a send is kept ([`Sent::Waiting`]), and so is every send
    /// after it, in the order they were made, and [`Self::sign_waiting`] signs them once the person
    /// may. A send the machine makes again while it waits waits once; another content under the
    /// dot of a change that waits, or that the device signed, is refused at once.
    ///
    /// **What is final is refused.** An event that is not signed is [`Sent::NotSigned`]; a change
    /// the host holds already is [`Sent::Held`]; another person's that the host holds no
    /// signature for, a person that may not write under the log the host holds (a reader, a
    /// device taken off), a frozen chain, a forked log and a host with no person are
    /// [`Sent::Refused`]. A device whose log grants it writing signs the send at once
    /// ([`Sent::Signed`]), as [`Self::seal_sent`] does.
    pub fn keep_sent(&self, event: &str, data: &str) -> Sent {
        self.keep(event, data, false)
    }

    /// A change its host keeps a journal of (`record_list.made`, docs/adr/0083, section 3.1), kept
    /// as [`Self::keep_sent`] keeps a send: the outbox. The operation signed for it, now or when it
    /// waited and a round signs it, is a record ([`Self::unrecorded`]) that no relay is given, nor
    /// any operation of the chain after it, until the host says it recorded it in its journal
    /// ([`Self::recorded`]): what the person is shown as saved is in the journal before anyone
    /// else holds it. `data` is the change (`record_list.change`'s payload).
    pub fn keep_made(&self, data: &str) -> Sent {
        self.keep(CHANGE_EVENT, data, true)
    }

    fn keep(&self, event: &str, data: &str, journalled: bool) -> Sent {
        let mut state = self.state();
        let Some(send) = Waiting::of(&state.calendar, event, data, journalled) else {
            return Sent::of(seal_in(&mut state, event, data));
        };
        let held = state.changes.contains_key(&send.content);
        let own = matches!(&state.own, Some((node, _)) if *node == send.author);
        if held || !own {
            return Sent::of(seal_in(&mut state, event, data));
        }
        if state
            .waiting
            .iter()
            .any(|waiting| waiting.is_the_same(&send))
        {
            return Sent::Waiting(why_it_waits(&state, send.event));
        }
        let clash = state.waiting.iter().any(|waiting| waiting.dot == send.dot);
        if clash {
            return Sent::Refused(format!(
                "{CHANGE_EVENT}: a change of this device that waits to be signed has another content under this dot (its author and its stamp), and one change is signed with one content"
            ));
        }
        if let Some(reason) = another_content_signed(&state, send.author, &send.dot, &send.content)
        {
            return Sent::Refused(reason);
        }
        if !state.waiting.is_empty() {
            let why = why_it_waits(&state, send.event);
            state.waiting.push(send);
            return Sent::Waiting(why);
        }
        match readiness(&state, send.event) {
            Readiness::Ready => {
                let sent = Sent::of(seal_in(&mut state, event, data));
                if journalled && sent == Sent::Signed {
                    note_unrecorded(&mut state);
                }
                sent
            }
            Readiness::NotYet(why) => {
                state.waiting.push(send);
                Sent::Waiting(why)
            }
            Readiness::Never(why) => Sent::Refused(why),
        }
    }

    /// Sign what the machine sent that waits ([`Self::keep_sent`]), in the order it was sent, as
    /// far as the person may now: what a round calls once it has read the log and the device's own
    /// chain, before it gives the relay what it lacks. A send that still may not be signed, and
    /// every one after it, waits on. A send that never will be (the log the host holds says the
    /// person may not write: a reader, a device taken off, a key the log replaced; or it conflicts
    /// with what the device signed, a stamp that goes back along its chain among them) is
    /// withdrawn, and so is every send after it, in order, each with why (docs/adr/0093, section
    /// 6.2): each was made on a state that held the one before it, which no copy but the device's
    /// own will hold, and the copy withdraws the same suffix. What it did is returned, for the host
    /// to tell. A send of a host that keeps a journal of it ([`Self::keep_made`]) that is signed
    /// here is a record, held back from the relay until the host recorded it.
    ///
    /// **What was signed and what was held are told apart.** A send that waited may turn out, when
    /// its turn comes, to be a change the host holds already: a device restored from an
    /// older store kept it waiting because the store did not hold it, and the round then read the
    /// device's own chain back from the relay, which did. It leaves as the operation held, and is in
    /// [`Waited::held`], not in [`Waited::taken`], which holds only the sends a new operation was
    /// signed for.
    pub fn sign_waiting(&self) -> Waited {
        let mut state = self.state();
        let mut waited = Waited::default();
        while !state.waiting.is_empty() {
            let send = state.waiting.remove(0);
            let never = match readiness(&state, send.event) {
                Readiness::NotYet(why) => {
                    state.waiting.insert(0, send);
                    waited.why_waiting = Some(why);
                    break;
                }
                Readiness::Never(why) => why,
                Readiness::Ready => match seal_in(&mut state, send.event, &send.data) {
                    Ok(Some(sealed)) if sealed.signed => {
                        if send.journalled {
                            note_unrecorded(&mut state);
                        }
                        waited.taken.push((send.event.to_string(), send.data));
                        continue;
                    }
                    // Nothing new was signed: the change leaves as the operation the host held
                    // already.
                    Ok(_) => {
                        waited.held.push((send.event.to_string(), send.data));
                        continue;
                    }
                    Err(why) => why,
                },
            };
            waited
                .dropped
                .push((send.event.to_string(), send.data, never));
            for after in std::mem::take(&mut state.waiting) {
                let why = format!(
                    "{}: a change this device made before it was withdrawn, and a change made on a state no other copy will hold is withdrawn with it",
                    after.event
                );
                waited
                    .dropped
                    .push((after.event.to_string(), after.data, why));
            }
        }
        waited.waiting = state.waiting.len();
        waited
    }

    /// How many sends of the machine wait to be signed ([`Self::keep_sent`]).
    pub fn waiting(&self) -> usize {
        self.state().waiting.len()
    }
}

/// The operation this host's person signed last is a record its host keeps a journal of, which no
/// relay is given before the host recorded it ([`Trust::unrecorded`]).
fn note_unrecorded(state: &mut State) {
    let last = state
        .own
        .as_ref()
        .and_then(|(node, _)| state.ops.get(node))
        .and_then(Chain::last)
        .map(Held::seq);
    if let Some(seq) = last {
        state.unrecorded.insert(seq);
    }
}

/// Why a send of `event` waits: what keeps the person from signing now, or, when nothing does, the
/// sends before it that wait, since a device signs its sends in the order they were made.
fn why_it_waits(state: &State, event: &str) -> String {
    match readiness(state, event) {
        Readiness::NotYet(why) => why,
        Readiness::Ready | Readiness::Never(_) => format!(
            "{event}: sends of this device made before it wait to be signed, and a device signs its sends in the order they were made"
        ),
    }
}

impl Sealer for Trust {
    fn seal(&mut self, _target: &str, event: &str, data: &str) -> Result<Option<String>, String> {
        Ok(self.seal_sent(event, data)?.map(|sealed| sealed.data))
    }
}

/// A change leaves as it traveled to this host, with its author's signature, when it is another
/// person's; and as this host's person's own, each of its writes sealed when the host keeps content
/// from the hub, and signed here whole, as one operation, when it is theirs (docs/adr/0085, section
/// 1). A change this host cannot make leave as it must is not sent as something else: a host that
/// keeps content and holds no key of the epoch the log is in sends nothing, and does not fall back
/// to an older key or to the clear.
fn seal_change(state: &mut State, data: &str) -> Result<Sealed, String> {
    let event = CHANGE_EVENT;
    let malformed = || format!("{event}: the data is not a change");
    let visible: Value = serde_json::from_str(data).map_err(|_| malformed())?;
    let signing = match message::signing(&state.calendar, event, &visible) {
        Some(Ok(signing)) => signing,
        Some(Err(error)) => return Err(format!("{event}: {error:?}")),
        None => return Err(malformed()),
    };
    let held = signing.message;
    let own = matches!(&state.own, Some((node, _)) if *node == signing.author);
    // What was made or admitted before leaves as it did: one change is one set of bytes, and the
    // same change sent again signs nothing.
    if let Some(wire) = state.changes.get(&held) {
        let data = wire.to_json().ok_or_else(malformed)?;
        return Ok(Sealed {
            data,
            signed: false,
        });
    }
    if !own {
        return Err(format!(
            "{event}: this host holds no signature of node {}'s for it, and signs none in their name",
            signing.author
        ));
    }
    // One dot names one change. Another change under a dot this device has signed is refused here,
    // before its writes are sealed under the dot's nonces or it is signed: it would be the device
    // signing two things for one change. So is a change that writes one register twice, whose two
    // writes would be sealed under one nonce.
    if let Some(reason) = another_content_signed(state, signing.author, &signing.subject, &held) {
        return Err(reason);
    }
    let writes = message::writes_of(&visible).map_err(|error| format!("{event}: {error:?}"))?;
    if names_no_epoch_of_its_group(state) {
        return Err(no_epoch_of_the_group(event));
    }
    let mut wire = visible.clone();
    if state.keeps_content() {
        let epoch = state.log.epoch();
        let key = state
            .epoch_keys
            .get(&epoch)
            .ok_or_else(|| no_key_of_the_epoch(event, epoch))?;
        for (at, stamp) in writes {
            let (number, text) =
                message::write_content(&visible["ops"][at]).map_err(|_| malformed())?;
            let object = wire["ops"][at].as_object_mut().ok_or_else(malformed)?;
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
    }
    let operation = make_operation(state, event, wire, signing.author, Some(held))?;
    let data = operation.to_json().ok_or_else(malformed)?;
    Ok(Sealed { data, signed: true })
}
