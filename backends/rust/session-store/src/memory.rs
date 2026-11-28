// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A session store that keeps nothing past the process.

use std::collections::{BTreeMap, BTreeSet};
use std::ops::{Bound, RangeInclusive};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crate::audit::{
    rules as audit_rules, AuditAppend, AuditCheckpoint, AuditHead, AuditRow, AuditStore,
    NoCheckpointAt,
};
use crate::batch::{Batched, Committed, Decided};
use crate::journal::{
    recorded, Actor, AsOf, AsOfCursor, AsOfError, Fact, Journal, JournalEntry, JournalPage,
    JournalStore, Observation, ObservedActor, ObservedRead, ObservedStream, PastHorizon,
    SessionsStream, Stream, StreamName,
};
use crate::kv::{rules as kv_rules, Identity, KvEntry, KvStore, Put, PutIf};
use crate::membership::judge::{self, Judged};
use crate::membership::{
    Admission, Delegation, Ended, Joining, Member, Membership, MembershipStore, Offer, Offered,
    PersonKey, Renewal, Revocation, Terms,
};
use crate::outbox::{
    Append, Appended, Change, Expired, Expiry, Note, Outbox, OutboxEntry, Write, Written,
};
use crate::rules::{self, Holding};
use crate::{
    moment, AcquireError, Due, Failure, Lease, LeaseError, SessionStore, Stored, WriteError,
};

/// Every position there is: none is `0`.
const EVERY_POSITION: RangeInclusive<u64> = 1..=u64::MAX;

/// A session store in memory, for the tests of a crate that is given a store, and the store of the
/// key-value feature a host keeps when it is given no durable one ([`KvStore`]).
///
/// It keeps the contract as the reference store does, and is held to the same conformance kit: a
/// test that passes over it passes over a durable store for the same reasons. A clone is another
/// handle on the same store, as a second connection is on a database, so a test can give each of
/// its threads one. One lock is a transaction: every call reads and writes under it, and writes
/// only after the rules admitted the call, so a call refused or failed leaves nothing changed.
///
/// A handle keeps the sessions of one machine ([`SessionStore::machine`]): a new store's is of the
/// machine the empty name stands for, and [`Self::of_machine`] gives a handle on the same store of
/// another (ADR 0114, D2).
#[derive(Clone, Default)]
pub struct MemoryStore {
    shared: Arc<Mutex<Records>>,
    /// The machine whose sessions this handle keeps.
    machine: String,
}

/// What names a session: its machine and its key (ADR 0114, D2).
type Session = (String, String);

/// The session of `machine` that `key` names.
fn session(machine: &str, key: &str) -> Session {
    (machine.to_string(), key.to_string())
}

/// A client's outbox, without its entries: the incarnation it numbers in, its next number, and
/// when an append or an acknowledgement last changed it.
#[derive(Clone)]
struct Head {
    incarnation: String,
    next: u64,
    touched_ms: u64,
}

/// A note an expiry wrote, with the session it is for.
#[derive(Clone)]
struct NoteRow {
    session: Session,
    note: Note,
}

#[derive(Default)]
struct Records {
    sessions: BTreeMap<Session, Stored>,
    leases: BTreeMap<Session, Holding>,
    /// Every client's outbox head, and its entries by client and number.
    heads: BTreeMap<String, Head>,
    entries: BTreeMap<(String, u64), OutboxEntry>,
    /// The machine of the session that sent each entry, by client and number: which machine's
    /// session an expiry's note is for.
    senders: BTreeMap<(String, u64), String>,
    /// The notes of expiries, by their number.
    notes: BTreeMap<u64, NoteRow>,
    /// The last number the store handed out, a version, a fence or a note's.
    last: u64,
    /// The values of the key-value feature, by key (module [`crate::kv`]). A `BTreeMap` of
    /// `String` orders keys by their UTF-8 bytes.
    values: BTreeMap<String, KvEntry>,
    /// The last number a write or a removal of a value took: the feature's own counter.
    values_last: u64,
    /// The first result of each put-if, by its identity: session, invoke, start.
    first_results: BTreeMap<(String, String, u64), PutIf>,
    /// The audit log (module [`crate::audit`]): each stream's head, its rows and its
    /// checkpoints by stream and sequence, and the sequence each identity was appended at.
    audit_heads: BTreeMap<String, AuditHead>,
    audit_rows: BTreeMap<(String, u64), AuditRow>,
    audit_checkpoints: BTreeMap<(String, u64), AuditCheckpoint>,
    audit_identities: BTreeMap<(String, String, u64), u64>,
    /// The journal (module [`crate::journal`]): each entry by its position, the positions of each
    /// stream, the position each origin id of an observed stream is kept at, and the last position
    /// and recorded moment given.
    journal: BTreeMap<u64, JournalEntry>,
    journal_streams: BTreeSet<(StreamName, u64)>,
    journal_origins: BTreeMap<(String, String), u64>,
    journal_last: u64,
    journal_recorded_ms: u64,
    /// The members of open machines and the offers that wait for an exchange to confirm them
    /// (module [`crate::membership`]), by the member's name.
    members: BTreeMap<String, Member>,
    offers: BTreeMap<String, Offer>,
    /// How far each reader of the journal has taken it in, by its name.
    readers: BTreeMap<String, u64>,
}

impl Records {
    fn next_number(&mut self) -> u64 {
        self.last += 1;
        self.last
    }

    fn version_of(&self, session: &Session) -> u64 {
        self.sessions
            .get(session)
            .map_or(0, |stored| stored.version)
    }

    /// Append `appends` at `now_ms`, each to its client's outbox, opened in the append's
    /// incarnation when it is not held, each sent by a session of `machine`.
    fn append(&mut self, machine: &str, appends: &[Append<'_>], now_ms: u64) -> Vec<Appended> {
        let at_ms = moment(now_ms);
        appends
            .iter()
            .map(|append| {
                let head = self
                    .heads
                    .entry(append.client.to_string())
                    .or_insert_with(|| Head {
                        incarnation: append.incarnation.to_string(),
                        next: 1,
                        touched_ms: at_ms,
                    });
                let number = head.next;
                head.next += 1;
                head.touched_ms = at_ms;
                let incarnation = head.incarnation.clone();
                self.senders
                    .insert((append.client.to_string(), number), machine.to_string());
                self.entries.insert(
                    (append.client.to_string(), number),
                    OutboxEntry {
                        number,
                        at_ms,
                        sender: append.sender.map(str::to_string),
                        envelope: append.envelope.to_vec(),
                    },
                );
                Appended {
                    client: append.client.to_string(),
                    incarnation,
                    number,
                }
            })
            .collect()
    }

    /// The outbox of `client`, if one is held.
    fn outbox(&self, client: &str) -> Option<Outbox> {
        let head = self.heads.get(client)?;
        Some(Outbox {
            incarnation: head.incarnation.clone(),
            next: head.next,
            entries: self.entries_of(client).cloned().collect(),
        })
    }

    /// The entries of `client`'s outbox, oldest first.
    fn entries_of<'a>(&'a self, client: &str) -> impl Iterator<Item = &'a OutboxEntry> + 'a {
        let from = (client.to_string(), 0);
        let to = (client.to_string(), u64::MAX);
        self.entries.range(from..=to).map(|(_, entry)| entry)
    }

    /// What [`SessionStore::write`] does to the session of `machine` that `key` names, on these
    /// records: nothing unless the rules admit it and the bytes of each of its facts could be made.
    fn write(
        &mut self,
        machine: &str,
        key: &str,
        write: &Write<'_>,
    ) -> Result<Written, WriteError> {
        let named = session(machine, key);
        rules::admit_write(
            self.leases.get(&named),
            write.fence,
            self.version_of(&named),
            write.expected_version,
        )?;
        for under in write.under {
            judge::admit_under(
                self.members.get(under.member),
                under.incarnation,
                write.now_ms,
            )?;
        }
        // Before anything is changed, since a fact's bytes may fail to be made.
        let journaled = match &write.journal {
            Some(journal) => self.journaled(key, journal, write.now_ms)?,
            None => Vec::new(),
        };
        let version = match write.change {
            Change::Keep {
                state,
                earliest_due,
            } => {
                let version = self.next_number();
                self.sessions.insert(
                    named.clone(),
                    Stored {
                        state: state.to_vec(),
                        version,
                        earliest_due: earliest_due.map(moment),
                    },
                );
                self.take_notes(&named, Some(write.notes_through));
                version
            }
            Change::Remove => {
                self.sessions.remove(&named);
                self.take_notes(&named, None);
                self.first_results
                    .retain(|(session, _, _), _| !write.sessions.contains(&session.as_str()));
                self.audit_identities
                    .retain(|(session, _, _), _| !write.sessions.contains(&session.as_str()));
                0
            }
        };
        let appended = self.append(machine, write.outbox, write.now_ms);
        for entry in journaled {
            self.keep_journaled(entry);
        }
        for under in write.under {
            if let Some(member) = self.members.get_mut(under.member) {
                *member = judge::kept(member, write.now_ms);
            }
        }
        Ok(Written { version, appended })
    }

    /// The journal's horizon, the last position given. A position is given, and its entry kept,
    /// under the one lock that is the write's transaction, which a read takes too: a read never runs
    /// between the two, however many threads write, so every position given is visible and nothing
    /// can still appear at or below the last one.
    fn journal_horizon(&self) -> u64 {
        rules::horizon(self.journal_last)
    }

    /// The entries the facts of `journal` are, as a write of the session `key` at `now_ms` appends
    /// them; nothing is kept, and no position given, until each is kept
    /// ([`Self::keep_journaled`]).
    fn journaled(
        &self,
        key: &str,
        journal: &Journal<'_>,
        now_ms: u64,
    ) -> Result<Vec<JournalEntry>, Failure> {
        if journal.facts.is_empty() {
            return Ok(Vec::new());
        }
        if self.holds_observed(journal.machine) {
            return Err(Failure::new(ObservedStream {
                name: journal.machine.to_string(),
            }));
        }
        let stream = Stream::Session {
            machine: journal.machine,
            key,
        };
        let mut position = self.journal_last;
        journal
            .facts
            .iter()
            .map(|fact| {
                position = rules::next_position(position)?;
                self.entry_at(position, stream, None, fact, now_ms)
            })
            .collect()
    }

    /// The entry `fact` is in `stream` at `position`, recorded at `now_ms`.
    fn entry_at(
        &self,
        position: u64,
        stream: Stream<'_>,
        origin: Option<&str>,
        fact: &Fact<'_>,
        now_ms: u64,
    ) -> Result<JournalEntry, Failure> {
        Ok(JournalEntry {
            position,
            recorded_ms: recorded(self.journal_recorded_ms, now_ms),
            stream: stream.to_name(),
            origin: origin.map(str::to_string),
            schema: fact.schema.to_string(),
            revision: fact.revision,
            valid_ms: fact.valid_ms,
            actor: fact.actor.map(Actor::to_name),
            read: fact.read.to_vec(),
            data: fact.data.at(position)?.into_owned(),
        })
    }

    /// Keep `entry`, whose position is then the last one given.
    fn keep_journaled(&mut self, entry: JournalEntry) {
        self.journal_last = entry.position;
        self.journal_recorded_ms = entry.recorded_ms;
        self.journal_streams
            .insert((entry.stream.clone(), entry.position));
        if let (StreamName::Observed { name }, Some(origin)) = (&entry.stream, &entry.origin) {
            self.journal_origins
                .insert((name.clone(), origin.clone()), entry.position);
        }
        self.journal.insert(entry.position, entry);
    }

    /// Whether the journal holds an observed stream named `name`.
    fn holds_observed(&self, name: &str) -> bool {
        let observed = Stream::Observed { name }.to_name();
        self.journal_of(&observed, EVERY_POSITION).next().is_some()
    }

    /// Whether the journal holds entries the sessions of the machine `machine` made.
    fn holds_sessions_of(&self, machine: &str) -> bool {
        // The streams of a machine's sessions are next to each other, the one of the empty key
        // first.
        let first = Stream::Session { machine, key: "" }.to_name();
        self.journal_streams
            .range((first, 0_u64)..)
            .next()
            .is_some_and(|(stream, _)| match stream {
                StreamName::Session { machine: held, .. } => held == machine,
                StreamName::Observed { .. } => false,
            })
    }

    /// The entries of `stream` at `positions`, a range whose first position is not past its last,
    /// in the order of their positions.
    fn journal_of<'a>(
        &'a self,
        stream: &StreamName,
        positions: RangeInclusive<u64>,
    ) -> impl Iterator<Item = &'a JournalEntry> + 'a {
        let from = (stream.clone(), *positions.start());
        let to = (stream.clone(), *positions.end());
        self.journal_streams
            .range(from..=to)
            .filter_map(move |(_, position)| self.journal.get(position))
    }

    /// What [`SessionStore::acknowledge`] does, on these records: how many entries it deleted.
    fn acknowledge(&mut self, client: &str, incarnation: &str, through: u64, now_ms: u64) -> u64 {
        if !rules::acknowledges(
            self.heads.get(client).map(|head| head.incarnation.as_str()),
            incarnation,
        ) {
            return 0;
        }
        let covered: Vec<(String, u64)> = self
            .entries_of(client)
            .take_while(|entry| entry.number <= through)
            .map(|entry| (client.to_string(), entry.number))
            .collect();
        for key in &covered {
            self.entries.remove(key);
            self.senders.remove(key);
        }
        if !covered.is_empty() {
            if let Some(head) = self.heads.get_mut(client) {
                head.touched_ms = moment(now_ms);
            }
        }
        covered.len() as u64
    }

    /// Take the notes of `session`: those numbered at most `through`, or every one when `through`
    /// is `None`.
    fn take_notes(&mut self, session: &Session, through: Option<u64>) {
        self.notes.retain(|id, row| {
            row.session != *session || through.is_some_and(|through| *id > through)
        });
    }

    /// Keep `value` under `key` at the value counter's next number; that number.
    fn write_value(&mut self, key: &str, value: &[u8]) -> u64 {
        self.values_last += 1;
        let version = self.values_last;
        self.values.insert(
            key.to_string(),
            KvEntry {
                value: value.to_vec(),
                version,
            },
        );
        version
    }

    /// Whether writing `key` would hold more values than `quota` allows.
    fn values_over_quota(&self, key: &str, quota: Option<u64>) -> bool {
        kv_rules::over_quota(
            !self.values.contains_key(key),
            self.values.len() as u64,
            quota,
        )
    }
}

/// The key a first result is kept under.
fn identity_key(identity: &Identity<'_>) -> (String, String, u64) {
    (
        identity.session.to_string(),
        identity.invoke.to_string(),
        identity.start,
    )
}

impl MemoryStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// A handle on the same store that keeps the sessions of `machine` (ADR 0114, D2): every call
    /// that names a key names the session of `machine` and that key, and what the handles share
    /// (the journal, the outboxes, the counter that numbers versions and fences) is one.
    pub fn of_machine(&self, machine: &str) -> Self {
        Self {
            shared: Arc::clone(&self.shared),
            machine: machine.to_string(),
        }
    }

    /// The session of this handle's machine that `key` names.
    fn session(&self, key: &str) -> Session {
        session(&self.machine, key)
    }

    /// The records, whatever a thread that panicked while it held them left: nothing, since every
    /// call changes them only after its checks, in steps that do not panic.
    fn records(&self) -> MutexGuard<'_, Records> {
        self.shared.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl SessionStore for MemoryStore {
    fn machine(&self) -> &str {
        &self.machine
    }

    fn get(&self, key: &str) -> Result<Option<Stored>, Failure> {
        Ok(self.records().sessions.get(&self.session(key)).cloned())
    }

    fn write(&self, key: &str, write: &Write<'_>) -> Result<Written, WriteError> {
        self.records().write(&self.machine, key, write)
    }

    /// Every item under the one lock that is the batch's transaction, each decided by the same
    /// steps as a call of its own, which change nothing before the rules admitted the item.
    fn write_many(&self, batch: &[Batched<'_>]) -> Result<Vec<Decided>, Failure> {
        let mut records = self.records();
        Ok(batch
            .iter()
            .map(|item| match *item {
                Batched::Write { key, write } => records
                    .write(&self.machine, key, &write)
                    .map(Committed::Written),
                Batched::Acknowledge {
                    client,
                    incarnation,
                    through,
                    now_ms,
                } => Ok(Committed::Acknowledged(records.acknowledge(
                    client,
                    incarnation,
                    through,
                    now_ms,
                ))),
            })
            .collect())
    }

    fn due_before(&self, ms: u64, limit: usize) -> Result<Vec<Due>, Failure> {
        let records = self.records();
        let mut listed: Vec<Due> = records
            .sessions
            .iter()
            .filter(|((machine, _), _)| *machine == self.machine)
            .filter_map(|((_, key), stored)| {
                stored.earliest_due.filter(|&due| due <= ms).map(|due| Due {
                    key: key.clone(),
                    due,
                })
            })
            .collect();
        // The map is in the order of the keys, and a stable sort keeps it among equal dues.
        listed.sort_by_key(|entry| entry.due);
        listed.truncate(limit);
        Ok(listed)
    }

    fn indexed_after(&self, after: Option<&str>, limit: usize) -> Result<Vec<String>, Failure> {
        let records = self.records();
        let from = match after {
            Some(after) => Bound::Excluded(self.session(after)),
            None => Bound::Included(self.session("")),
        };
        Ok(records
            .sessions
            .range((from, Bound::Unbounded))
            .take_while(|((machine, _), _)| *machine == self.machine)
            .take(limit)
            .map(|((_, key), _)| key.clone())
            .collect())
    }

    fn acquire(
        &self,
        key: &str,
        owner: &str,
        ttl_ms: u64,
        now_ms: u64,
    ) -> Result<Lease, AcquireError> {
        let named = self.session(key);
        let mut records = self.records();
        rules::admit_acquire(records.leases.get(&named), owner, now_ms)?;
        let lease = Lease {
            fence: records.next_number(),
            expires_ms: rules::expiry(now_ms, ttl_ms),
        };
        records.leases.insert(
            named,
            Holding {
                owner: owner.to_string(),
                fence: lease.fence,
                expires_ms: lease.expires_ms,
            },
        );
        Ok(lease)
    }

    fn renew(&self, key: &str, fence: u64, ttl_ms: u64, now_ms: u64) -> Result<Lease, LeaseError> {
        let named = self.session(key);
        let mut records = self.records();
        rules::admit_renew(records.leases.get(&named), fence, now_ms)?;
        let expires_ms = rules::expiry(now_ms, ttl_ms);
        if let Some(held) = records.leases.get_mut(&named) {
            held.expires_ms = expires_ms;
        }
        Ok(Lease { fence, expires_ms })
    }

    fn release(&self, key: &str, fence: u64) -> Result<(), LeaseError> {
        let named = self.session(key);
        let mut records = self.records();
        rules::admit_release(records.leases.get(&named), fence)?;
        records.leases.remove(&named);
        Ok(())
    }

    fn append(&self, appends: &[Append<'_>], now_ms: u64) -> Result<Vec<Appended>, Failure> {
        Ok(self.records().append(&self.machine, appends, now_ms))
    }

    fn outbox(&self, client: &str) -> Result<Option<Outbox>, Failure> {
        Ok(self.records().outbox(client))
    }

    fn open_outbox(&self, client: &str, incarnation: &str, now_ms: u64) -> Result<Outbox, Failure> {
        let mut records = self.records();
        records
            .heads
            .entry(client.to_string())
            .or_insert_with(|| Head {
                incarnation: incarnation.to_string(),
                next: 1,
                touched_ms: moment(now_ms),
            });
        Ok(records
            .outbox(client)
            .expect("the outbox was opened in this transaction"))
    }

    fn acknowledge(
        &self,
        client: &str,
        incarnation: &str,
        through: u64,
        now_ms: u64,
    ) -> Result<u64, Failure> {
        Ok(self
            .records()
            .acknowledge(client, incarnation, through, now_ms))
    }

    fn expire(&self, client: &str, before_ms: u64, limit: usize) -> Result<Expiry, Failure> {
        let mut records = self.records();
        let old: Vec<OutboxEntry> = records
            .entries_of(client)
            .filter(|entry| entry.at_ms < before_ms)
            .take(limit)
            .cloned()
            .collect();
        let mut expiry = Expiry::default();
        for entry in old {
            let place = (client.to_string(), entry.number);
            records.entries.remove(&place);
            // A note is for the session of the machine that sent the entry, while the store holds
            // it: one that ended has no next run.
            let machine = records.senders.remove(&place).unwrap_or_default();
            let noted = entry
                .sender
                .as_deref()
                .map(|sender| session(&machine, sender))
                .filter(|sender| records.sessions.contains_key(sender));
            if let Some(sender) = noted {
                let id = records.next_number();
                records.notes.insert(
                    id,
                    NoteRow {
                        session: sender.clone(),
                        note: Note {
                            id,
                            client: client.to_string(),
                            number: entry.number,
                            at_ms: entry.at_ms,
                            envelope: entry.envelope.clone(),
                        },
                    },
                );
                if let Some(stored) = records.sessions.get_mut(&sender) {
                    stored.earliest_due = Some(rules::DUE_AT_ONCE);
                }
            }
            expiry.expired.push(Expired {
                client: client.to_string(),
                entry,
            });
        }
        let empty = records.entries_of(client).next().is_none();
        let untouched = records
            .heads
            .get(client)
            .is_some_and(|head| head.touched_ms < before_ms);
        if empty && untouched {
            records.heads.remove(client);
            expiry.closed = true;
        }
        Ok(expiry)
    }

    fn notes(&self, key: &str) -> Result<Vec<Note>, Failure> {
        let named = self.session(key);
        let records = self.records();
        Ok(records
            .notes
            .values()
            .filter(|row| row.session == named)
            .map(|row| row.note.clone())
            .collect())
    }

    fn outbox_clients_after(
        &self,
        after: Option<&str>,
        limit: usize,
    ) -> Result<Vec<String>, Failure> {
        let from = after.map_or(Bound::Unbounded, Bound::Excluded);
        Ok(self
            .records()
            .heads
            .range::<str, _>((from, Bound::Unbounded))
            .take(limit)
            .map(|(client, _)| client.clone())
            .collect())
    }

    fn reseal_entry(
        &self,
        client: &str,
        number: u64,
        held: &[u8],
        envelope: &[u8],
    ) -> Result<bool, Failure> {
        let mut records = self.records();
        Ok(
            match records.entries.get_mut(&(client.to_string(), number)) {
                Some(entry) if entry.envelope == held => {
                    entry.envelope = envelope.to_vec();
                    true
                }
                _ => false,
            },
        )
    }

    fn reseal_note(
        &self,
        key: &str,
        id: u64,
        held: &[u8],
        envelope: &[u8],
    ) -> Result<bool, Failure> {
        let named = self.session(key);
        let mut records = self.records();
        Ok(match records.notes.get_mut(&id) {
            Some(row) if row.session == named && row.note.envelope == held => {
                row.note.envelope = envelope.to_vec();
                true
            }
            _ => false,
        })
    }
}

/// The members, kept beside the sessions so that a write made under an incarnation is judged
/// against them under the one lock that is its transaction.
impl MembershipStore for MemoryStore {
    fn offer_as(
        &self,
        member: &str,
        nonce: &str,
        joining: Joining,
        ttl_ms: u64,
        now_ms: u64,
    ) -> Result<Offered, Failure> {
        let mut records = self.records();
        if let Some(offer) = records
            .offers
            .get(member)
            .filter(|offer| judge::offers_again(Some(*offer), nonce, joining, now_ms))
        {
            return Ok(Offered {
                incarnation: offer.incarnation,
                expires_ms: offer.expires_ms,
            });
        }
        let offered = Offered {
            incarnation: records.next_number(),
            expires_ms: rules::expiry(now_ms, ttl_ms),
        };
        records.offers.insert(
            member.to_string(),
            Offer {
                incarnation: offered.incarnation,
                nonce: nonce.to_string(),
                expires_ms: offered.expires_ms,
                joining,
            },
        );
        Ok(offered)
    }

    fn admit_as(
        &self,
        member: &str,
        incarnation: u64,
        mark: u64,
        delegation: Option<&Delegation>,
        terms: Terms,
        now_ms: u64,
    ) -> Result<Admission, Failure> {
        let mut records = self.records();
        let others = records
            .members
            .iter()
            .filter(|(name, held)| name.as_str() != member && judge::member_stands(held, now_ms))
            .count() as u64;
        let judged = judge::admit(
            records.members.get(member),
            records.offers.get(member),
            others,
            judge::Exchange {
                incarnation,
                mark,
                delegation,
            },
            terms,
            now_ms,
        );
        let admission = judged.admission();
        match judged {
            Judged::Renew { member: renewed } => {
                records.members.insert(member.to_string(), renewed);
            }
            Judged::Confirm {
                member: confirmed, ..
            } => {
                records.members.insert(member.to_string(), confirmed);
                records.offers.remove(member);
            }
            Judged::Refuse(_) => {}
        }
        Ok(admission)
    }

    fn membership(&self, member: &str) -> Result<Membership, Failure> {
        let records = self.records();
        Ok(Membership {
            member: records.members.get(member).cloned(),
            offer: records.offers.get(member).cloned(),
        })
    }

    fn end_expired(&self, before_ms: u64, limit: usize) -> Result<Vec<Ended>, Failure> {
        let mut records = self.records();
        let mut members: Vec<(u64, String, u64)> = records
            .members
            .iter()
            .filter(|(_, held)| held.ends_ms() <= before_ms)
            .map(|(name, held)| (held.ends_ms(), name.clone(), held.incarnation))
            .collect();
        members.sort();
        members.truncate(limit);
        let mut offers: Vec<(u64, String)> = records
            .offers
            .iter()
            .filter(|(_, offer)| offer.expires_ms <= before_ms)
            .map(|(name, offer)| (offer.expires_ms, name.clone()))
            .collect();
        offers.sort();
        offers.truncate(limit);
        for (_, name, _) in &members {
            records.members.remove(name);
        }
        for (_, name) in &offers {
            records.offers.remove(name);
        }
        Ok(members
            .into_iter()
            .map(|(_, member, incarnation)| Ended {
                member,
                incarnation,
            })
            .collect())
    }

    fn end_revoked(&self, revocation: Revocation<'_>) -> Result<Vec<Ended>, Failure> {
        let mut records = self.records();
        let ended: Vec<Ended> = records
            .members
            .iter()
            .filter(|(name, held)| judge::revokes(&revocation, name, held))
            .map(|(name, held)| Ended {
                member: name.clone(),
                incarnation: held.incarnation,
            })
            .collect();
        for gone in &ended {
            records.members.remove(&gone.member);
        }
        Ok(ended)
    }

    fn renew_delegation(
        &self,
        member: &str,
        incarnation: u64,
        person: PersonKey<'_>,
        until_ms: u64,
        now_ms: u64,
    ) -> Result<Renewal, Failure> {
        let mut records = self.records();
        let (renewal, renewed) = judge::renew(
            records.members.get(member),
            incarnation,
            person,
            until_ms,
            now_ms,
        );
        if let Some(renewed) = renewed {
            records.members.insert(member.to_string(), renewed);
        }
        Ok(renewal)
    }

    fn delegated_after(&self, after: Option<&str>, limit: usize) -> Result<Vec<String>, Failure> {
        let records = self.records();
        Ok(records
            .members
            .iter()
            .filter(|(name, held)| {
                held.delegation.is_some() && after.is_none_or(|after| name.as_str() > after)
            })
            .map(|(name, _)| name.clone())
            .take(limit)
            .collect())
    }

    fn reseal_delegation(
        &self,
        member: &str,
        incarnation: u64,
        held: &[u8],
        sealed: &[u8],
    ) -> Result<bool, Failure> {
        let mut records = self.records();
        let Some(delegation) = records
            .members
            .get_mut(member)
            .filter(|kept| kept.incarnation == incarnation)
            .and_then(|kept| kept.delegation.as_mut())
            .filter(|delegation| delegation.sealed == held)
        else {
            return Ok(false);
        };
        delegation.sealed = sealed.to_vec();
        Ok(true)
    }
}

/// The values of the key-value feature, kept beside the sessions so that a removal of a session
/// deletes its identities' first results under the one lock that is its transaction.
impl KvStore for MemoryStore {
    fn get(&self, key: &str) -> Result<Option<KvEntry>, Failure> {
        Ok(self.records().values.get(key).cloned())
    }

    fn put(&self, key: &str, value: &[u8], quota: Option<u64>) -> Result<Put, Failure> {
        let mut records = self.records();
        if records.values_over_quota(key, quota) {
            return Ok(Put::OverQuota);
        }
        let replaced = records.values.contains_key(key);
        let version = records.write_value(key, value);
        Ok(Put::Written { replaced, version })
    }

    fn put_if(
        &self,
        key: &str,
        value: &[u8],
        expected: Option<u64>,
        quota: Option<u64>,
        identity: Option<&Identity<'_>>,
    ) -> Result<PutIf, Failure> {
        let mut records = self.records();
        if let Some(identity) = identity {
            if let Some(first) = records.first_results.get(&identity_key(identity)) {
                return Ok(first.clone());
            }
        }
        let current = records.values.get(key).map_or(0, |entry| entry.version);
        let result = if !kv_rules::matches(expected, current) {
            PutIf::Unwritten { version: current }
        } else if records.values_over_quota(key, quota) {
            PutIf::OverQuota
        } else {
            PutIf::Written {
                version: records.write_value(key, value),
            }
        };
        if let Some(identity) = identity {
            records
                .first_results
                .insert(identity_key(identity), result.clone());
        }
        Ok(result)
    }

    fn delete(&self, key: &str) -> Result<bool, Failure> {
        let mut records = self.records();
        let existed = records.values.remove(key).is_some();
        if existed {
            records.values_last += 1;
        }
        Ok(existed)
    }

    fn next_key(&self, after: &str, prefix: &str) -> Result<Option<String>, Failure> {
        // Keys with the prefix are contiguous in this order, so the walk starts at the later of
        // the prefix and `after`, and the first key it meets answers or ends it.
        let from = if after.is_empty() || after < prefix {
            Bound::Included(prefix)
        } else {
            Bound::Excluded(after)
        };
        Ok(self
            .records()
            .values
            .range::<str, _>((from, Bound::Unbounded))
            .map(|(key, _)| key)
            .next()
            .filter(|key| kv_rules::walks_to(key, after, prefix))
            .cloned())
    }

    fn entries_after(
        &self,
        after: Option<&str>,
        limit: usize,
    ) -> Result<Vec<(String, KvEntry)>, Failure> {
        let from = after.map_or(Bound::Unbounded, Bound::Excluded);
        Ok(self
            .records()
            .values
            .range::<str, _>((from, Bound::Unbounded))
            .take(limit)
            .map(|(key, entry)| (key.clone(), entry.clone()))
            .collect())
    }

    fn first_result(&self, identity: &Identity<'_>) -> Result<Option<PutIf>, Failure> {
        Ok(self
            .records()
            .first_results
            .get(&identity_key(identity))
            .cloned())
    }

    fn keep_first(&self, identity: &Identity<'_>, result: &PutIf) -> Result<PutIf, Failure> {
        Ok(self
            .records()
            .first_results
            .entry(identity_key(identity))
            .or_insert_with(|| result.clone())
            .clone())
    }

    fn reseal(&self, key: &str, held: &[u8], value: &[u8]) -> Result<bool, Failure> {
        let mut records = self.records();
        Ok(match records.values.get_mut(key) {
            Some(entry) if entry.value == held => {
                entry.value = value.to_vec();
                true
            }
            _ => false,
        })
    }
}

/// The audit log, kept beside the sessions so that a removal of a session deletes its
/// identities under the one lock that is its transaction.
impl AuditStore for MemoryStore {
    fn audit_head(&self, stream: &str) -> Result<AuditHead, Failure> {
        Ok(self
            .records()
            .audit_heads
            .get(stream)
            .copied()
            .unwrap_or(AuditHead::EMPTY))
    }

    fn audit_append(
        &self,
        row: &AuditRow,
        checkpoint: Option<&[u8]>,
        identity: Option<&Identity<'_>>,
    ) -> Result<AuditAppend, Failure> {
        let mut records = self.records();
        if let Some(identity) = identity {
            if let Some(sequence) = records.audit_identities.get(&identity_key(identity)) {
                return Ok(AuditAppend::Repeated {
                    sequence: *sequence,
                });
            }
        }
        let head = records
            .audit_heads
            .get(&row.stream)
            .copied()
            .unwrap_or(AuditHead::EMPTY);
        if let Err(moved) = audit_rules::follows(&head, row) {
            return Ok(moved);
        }
        records.audit_heads.insert(
            row.stream.clone(),
            AuditHead {
                sequence: row.sequence,
                chain: row.chain,
            },
        );
        records
            .audit_rows
            .insert((row.stream.clone(), row.sequence), row.clone());
        if let Some(signature) = checkpoint {
            records.audit_checkpoints.insert(
                (row.stream.clone(), row.sequence),
                AuditCheckpoint {
                    stream: row.stream.clone(),
                    sequence: row.sequence,
                    chain: row.chain,
                    signature: signature.to_vec(),
                },
            );
        }
        if let Some(identity) = identity {
            records
                .audit_identities
                .insert(identity_key(identity), row.sequence);
        }
        Ok(AuditAppend::Appended)
    }

    fn audit_checkpoint(&self, checkpoint: &AuditCheckpoint) -> Result<bool, Failure> {
        let mut records = self.records();
        let key = (checkpoint.stream.clone(), checkpoint.sequence);
        if records.audit_checkpoints.contains_key(&key) {
            return Ok(false);
        }
        records.audit_checkpoints.insert(key, checkpoint.clone());
        Ok(true)
    }

    fn audit_rows(&self, stream: &str, after: u64, limit: usize) -> Result<Vec<AuditRow>, Failure> {
        let records = self.records();
        let Some(from) = after.checked_add(1) else {
            return Ok(Vec::new());
        };
        Ok(records
            .audit_rows
            .range((stream.to_string(), from)..=(stream.to_string(), u64::MAX))
            .take(limit)
            .map(|(_, row)| row.clone())
            .collect())
    }

    fn audit_checkpoints(&self, stream: &str) -> Result<Vec<AuditCheckpoint>, Failure> {
        let records = self.records();
        Ok(records
            .audit_checkpoints
            .range((stream.to_string(), 0)..=(stream.to_string(), u64::MAX))
            .map(|(_, checkpoint)| checkpoint.clone())
            .collect())
    }

    fn audit_drop_through(&self, stream: &str, through: u64) -> Result<u64, Failure> {
        let mut records = self.records();
        if !records
            .audit_checkpoints
            .contains_key(&(stream.to_string(), through))
        {
            return Err(Failure::new(NoCheckpointAt {
                stream: stream.to_string(),
                sequence: through,
            }));
        }
        let before = records.audit_rows.len();
        records
            .audit_rows
            .retain(|(of, sequence), _| of != stream || *sequence > through);
        records
            .audit_checkpoints
            .retain(|(of, sequence), _| of != stream || *sequence >= through);
        Ok((before - records.audit_rows.len()) as u64)
    }

    fn audit_first(&self, identity: &Identity<'_>) -> Result<Option<u64>, Failure> {
        Ok(self
            .records()
            .audit_identities
            .get(&identity_key(identity))
            .copied())
    }

    fn audit_streams_after(
        &self,
        after: Option<&str>,
        limit: usize,
    ) -> Result<Vec<String>, Failure> {
        let from = after.map_or(Bound::Unbounded, Bound::Excluded);
        Ok(self
            .records()
            .audit_heads
            .range::<str, _>((from, Bound::Unbounded))
            .take(limit)
            .map(|(stream, _)| stream.clone())
            .collect())
    }

    fn audit_reseal_row(
        &self,
        stream: &str,
        sequence: u64,
        held: &[u8],
        entry: &[u8],
    ) -> Result<bool, Failure> {
        let mut records = self.records();
        Ok(
            match records.audit_rows.get_mut(&(stream.to_string(), sequence)) {
                Some(row) if row.entry == held => {
                    row.entry = entry.to_vec();
                    true
                }
                _ => false,
            },
        )
    }

    fn audit_reseal_checkpoint(
        &self,
        stream: &str,
        sequence: u64,
        held: &[u8],
        signature: &[u8],
    ) -> Result<bool, Failure> {
        let mut records = self.records();
        Ok(
            match records
                .audit_checkpoints
                .get_mut(&(stream.to_string(), sequence))
            {
                Some(checkpoint) if checkpoint.signature == held => {
                    checkpoint.signature = signature.to_vec();
                    true
                }
                _ => false,
            },
        )
    }
}

/// The journal, kept beside the sessions so that a write keeps its state and its entries under the
/// one lock that is its transaction.
impl JournalStore for MemoryStore {
    fn after(&self, position: u64, limit: usize) -> Result<JournalPage, Failure> {
        let records = self.records();
        let horizon = records.journal_horizon();
        let entries = match rules::readable(position, horizon) {
            Some(positions) => records
                .journal
                .range(positions)
                .take(limit)
                .map(|(_, entry)| entry.clone())
                .collect(),
            None => Vec::new(),
        };
        Ok(JournalPage { entries, horizon })
    }

    fn stream(&self, stream: Stream<'_>, after: u64, limit: usize) -> Result<JournalPage, Failure> {
        let records = self.records();
        let horizon = records.journal_horizon();
        let entries = match rules::readable(after, horizon) {
            Some(positions) => records
                .journal_of(&stream.to_name(), positions)
                .take(limit)
                .cloned()
                .collect(),
            None => Vec::new(),
        };
        Ok(JournalPage { entries, horizon })
    }

    fn position_at(&self, ms: u64) -> Result<u64, Failure> {
        let records = self.records();
        let Some(positions) = rules::readable(0, records.journal_horizon()) else {
            return Ok(0);
        };
        // The moments of a store that gives its positions in the order of its commits rise with
        // the positions, so the first entry found from the horizon down is the one.
        Ok(records
            .journal
            .range(positions)
            .rev()
            .find(|(_, entry)| entry.recorded_ms <= ms)
            .map_or(0, |(position, _)| *position))
    }

    fn as_of(
        &self,
        stream: Stream<'_>,
        as_of: &AsOf,
        after: AsOfCursor,
        limit: usize,
    ) -> Result<JournalPage, AsOfError> {
        let records = self.records();
        let horizon = records.journal_horizon();
        as_of.admitted(horizon)?;
        let mut held: Vec<&JournalEntry> = records
            .journal_of(&stream.to_name(), EVERY_POSITION)
            .filter(|entry| as_of.includes(entry) && entry.cursor() > after)
            .collect();
        held.sort_by_key(|entry| entry.cursor());
        Ok(JournalPage {
            entries: held.into_iter().take(limit).cloned().collect(),
            horizon,
        })
    }

    fn observe(
        &self,
        name: &str,
        origin: &str,
        fact: &Fact<'_>,
        now_ms: u64,
    ) -> Result<Observation, Failure> {
        if fact.actor.is_some() {
            return Err(Failure::new(ObservedActor));
        }
        if !fact.read.is_empty() {
            return Err(Failure::new(ObservedRead));
        }
        let mut records = self.records();
        let kept = (name.to_string(), origin.to_string());
        if let Some(position) = records.journal_origins.get(&kept) {
            return Ok(Observation::Repeated {
                position: *position,
            });
        }
        if records.holds_sessions_of(name) {
            return Err(Failure::new(SessionsStream {
                name: name.to_string(),
            }));
        }
        let position = rules::next_position(records.journal_last)?;
        let entry = records.entry_at(
            position,
            Stream::Observed { name },
            Some(origin),
            fact,
            now_ms,
        )?;
        records.keep_journaled(entry);
        Ok(Observation::Kept { position })
    }

    fn reseal_journaled(&self, position: u64, held: &[u8], data: &[u8]) -> Result<bool, Failure> {
        let mut records = self.records();
        Ok(match records.journal.get_mut(&position) {
            Some(entry) if entry.data == held => {
                entry.data = data.to_vec();
                true
            }
            _ => false,
        })
    }

    fn read_through(&self, reader: &str) -> Result<u64, Failure> {
        Ok(self.records().readers.get(reader).copied().unwrap_or(0))
    }

    fn keep_read(&self, reader: &str, position: u64) -> Result<u64, Failure> {
        let mut records = self.records();
        let horizon = records.journal_horizon();
        if position > horizon {
            return Err(Failure::new(PastHorizon {
                asked: position,
                horizon,
            }));
        }
        let kept = records.readers.entry(reader.to_string()).or_insert(0);
        *kept = (*kept).max(position);
        Ok(*kept)
    }
}
