// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A session store that keeps nothing past the process.

use std::collections::BTreeMap;
use std::ops::Bound;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crate::audit::{
    rules as audit_rules, AuditAppend, AuditCheckpoint, AuditHead, AuditRow, AuditStore,
    NoCheckpointAt,
};
use crate::kv::{rules as kv_rules, Identity, KvEntry, KvStore, Put, PutIf};
use crate::outbox::{
    Append, Appended, Change, Expired, Expiry, Note, Outbox, OutboxEntry, Write, Written,
};
use crate::rules::{self, Holding};
use crate::{
    moment, AcquireError, Due, Failure, Lease, LeaseError, SessionStore, Stored, WriteError,
};

/// A session store in memory, for the tests of a crate that is given a store, and the store of the
/// key-value feature a host keeps when it is given no durable one ([`KvStore`]).
///
/// It keeps the contract as the reference store does, and is held to the same conformance kit: a
/// test that passes over it passes over a durable store for the same reasons. A clone is another
/// handle on the same store, as a second connection is on a database, so a test can give each of
/// its threads one. One lock is a transaction: every call reads and writes under it, and writes
/// only after the rules admitted the call, so a call refused or failed leaves nothing changed.
#[derive(Clone, Default)]
pub struct MemoryStore {
    shared: Arc<Mutex<Records>>,
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
    session: String,
    note: Note,
}

#[derive(Default)]
struct Records {
    sessions: BTreeMap<String, Stored>,
    leases: BTreeMap<String, Holding>,
    /// Every client's outbox head, and its entries by client and number.
    heads: BTreeMap<String, Head>,
    entries: BTreeMap<(String, u64), OutboxEntry>,
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
}

impl Records {
    fn next_number(&mut self) -> u64 {
        self.last += 1;
        self.last
    }

    fn version_of(&self, key: &str) -> u64 {
        self.sessions.get(key).map_or(0, |stored| stored.version)
    }

    /// Append `appends` at `now_ms`, each to its client's outbox, opened in the append's
    /// incarnation when it is not held.
    fn append(&mut self, appends: &[Append<'_>], now_ms: u64) -> Vec<Appended> {
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

    /// Take the notes of `key`: those numbered at most `through`, or every one when `through` is
    /// `None`.
    fn take_notes(&mut self, key: &str, through: Option<u64>) {
        self.notes
            .retain(|id, row| row.session != key || through.is_some_and(|through| *id > through));
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

    /// The records, whatever a thread that panicked while it held them left: nothing, since every
    /// call changes them only after its checks, in steps that do not panic.
    fn records(&self) -> MutexGuard<'_, Records> {
        self.shared.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl SessionStore for MemoryStore {
    fn get(&self, key: &str) -> Result<Option<Stored>, Failure> {
        Ok(self.records().sessions.get(key).cloned())
    }

    fn write(&self, key: &str, write: &Write<'_>) -> Result<Written, WriteError> {
        let mut records = self.records();
        rules::admit_write(
            records.leases.get(key),
            write.fence,
            records.version_of(key),
            write.expected_version,
        )?;
        let version = match write.change {
            Change::Keep {
                state,
                earliest_due,
            } => {
                let version = records.next_number();
                records.sessions.insert(
                    key.to_string(),
                    Stored {
                        state: state.to_vec(),
                        version,
                        earliest_due: earliest_due.map(moment),
                    },
                );
                records.take_notes(key, Some(write.notes_through));
                version
            }
            Change::Remove => {
                records.sessions.remove(key);
                records.take_notes(key, None);
                records
                    .first_results
                    .retain(|(session, _, _), _| !write.sessions.contains(&session.as_str()));
                records
                    .audit_identities
                    .retain(|(session, _, _), _| !write.sessions.contains(&session.as_str()));
                0
            }
        };
        let appended = records.append(write.outbox, write.now_ms);
        Ok(Written { version, appended })
    }

    fn due_before(&self, ms: u64, limit: usize) -> Result<Vec<Due>, Failure> {
        let records = self.records();
        let mut listed: Vec<Due> = records
            .sessions
            .iter()
            .filter_map(|(key, stored)| {
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
        let from = after.map_or(Bound::Unbounded, Bound::Excluded);
        Ok(records
            .sessions
            .range::<str, _>((from, Bound::Unbounded))
            .take(limit)
            .map(|(key, _)| key.clone())
            .collect())
    }

    fn acquire(
        &self,
        key: &str,
        owner: &str,
        ttl_ms: u64,
        now_ms: u64,
    ) -> Result<Lease, AcquireError> {
        let mut records = self.records();
        rules::admit_acquire(records.leases.get(key), owner, now_ms)?;
        let lease = Lease {
            fence: records.next_number(),
            expires_ms: rules::expiry(now_ms, ttl_ms),
        };
        records.leases.insert(
            key.to_string(),
            Holding {
                owner: owner.to_string(),
                fence: lease.fence,
                expires_ms: lease.expires_ms,
            },
        );
        Ok(lease)
    }

    fn renew(&self, key: &str, fence: u64, ttl_ms: u64, now_ms: u64) -> Result<Lease, LeaseError> {
        let mut records = self.records();
        rules::admit_renew(records.leases.get(key), fence, now_ms)?;
        let expires_ms = rules::expiry(now_ms, ttl_ms);
        if let Some(held) = records.leases.get_mut(key) {
            held.expires_ms = expires_ms;
        }
        Ok(Lease { fence, expires_ms })
    }

    fn release(&self, key: &str, fence: u64) -> Result<(), LeaseError> {
        let mut records = self.records();
        rules::admit_release(records.leases.get(key), fence)?;
        records.leases.remove(key);
        Ok(())
    }

    fn append(&self, appends: &[Append<'_>], now_ms: u64) -> Result<Vec<Appended>, Failure> {
        Ok(self.records().append(appends, now_ms))
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
        let mut records = self.records();
        if !rules::acknowledges(
            records
                .heads
                .get(client)
                .map(|head| head.incarnation.as_str()),
            incarnation,
        ) {
            return Ok(0);
        }
        let covered: Vec<(String, u64)> = records
            .entries_of(client)
            .take_while(|entry| entry.number <= through)
            .map(|entry| (client.to_string(), entry.number))
            .collect();
        for key in &covered {
            records.entries.remove(key);
        }
        if !covered.is_empty() {
            if let Some(head) = records.heads.get_mut(client) {
                head.touched_ms = moment(now_ms);
            }
        }
        Ok(covered.len() as u64)
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
            records.entries.remove(&(client.to_string(), entry.number));
            let noted = entry
                .sender
                .clone()
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
        let records = self.records();
        Ok(records
            .notes
            .values()
            .filter(|row| row.session == key)
            .map(|row| row.note.clone())
            .collect())
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
}
