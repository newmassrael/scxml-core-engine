// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A session store that keeps nothing past the process.

use std::collections::BTreeMap;
use std::ops::Bound;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crate::rules::{self, Holding};
use crate::{
    moment, AcquireError, Due, Failure, Lease, LeaseError, SessionStore, Stored, WriteError,
};

/// A session store in memory, for the tests of a crate that is given a store.
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

#[derive(Default)]
struct Records {
    sessions: BTreeMap<String, Stored>,
    leases: BTreeMap<String, Holding>,
    /// The last number the store handed out, a version or a fence.
    last: u64,
}

impl Records {
    fn next_number(&mut self) -> u64 {
        self.last += 1;
        self.last
    }

    fn version_of(&self, key: &str) -> u64 {
        self.sessions.get(key).map_or(0, |stored| stored.version)
    }
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

    fn put(
        &self,
        key: &str,
        state: &[u8],
        earliest_due: Option<u64>,
        expected_version: u64,
        fence: u64,
    ) -> Result<u64, WriteError> {
        let mut records = self.records();
        rules::admit_write(
            records.leases.get(key),
            fence,
            records.version_of(key),
            expected_version,
        )?;
        let version = records.next_number();
        records.sessions.insert(
            key.to_string(),
            Stored {
                state: state.to_vec(),
                version,
                earliest_due: earliest_due.map(moment),
            },
        );
        Ok(version)
    }

    fn delete(&self, key: &str, expected_version: u64, fence: u64) -> Result<(), WriteError> {
        let mut records = self.records();
        rules::admit_write(
            records.leases.get(key),
            fence,
            records.version_of(key),
            expected_version,
        )?;
        records.sessions.remove(key);
        Ok(())
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
}
