// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Where a host keeps the values of the key-value feature (ADR 0109, decision D13;
//! docs/SCE_HOST_FEATURES.md section 3).
//!
//! A store maps keys to values, each value at a **version**: a number of one counter of the store,
//! which every put, every put-if that wrote and every delete that removed a key takes, so that a
//! version once read names one stored state of a key and is never given to another. The counter
//! is the feature's own, apart from the session store's, so that a scenario's versions are exact
//! on every store (docs/SCE_HOST_FEATURES.md section 8).
//!
//! Beside the values a store keeps the **first result** of each put-if by the identity of the
//! invocation that asked for it (docs/SCE_HOST_FEATURES.md section 1, "Identity"), written in the
//! transaction that makes the write, or judges it, so that a put-if a restore starts again under
//! its identity is answered with its first result whether the process that made it stopped before
//! or after the write: either both the write and its result are durable, or neither is. The rows
//! of a session's identities are deleted in the transaction that removes the session from the
//! session store ([`crate::Write::sessions`]), when one store keeps both.
//!
//! The quota is the host's: a put or a put-if names the most keys the store may hold, and the
//! store judges it in the transaction that would write.
//!
//! * [`crate::MemoryStore`] keeps them in the process, beside its sessions: what a host keeps when
//!   it is given no durable store.
//! * `SqliteStore` (feature `sqlite`) keeps them in the session store's file (layout 6): a restart
//!   keeps them.
//! * [`crate::kv_conformance`] is what every implementation is held to.

use crate::Failure;

/// The identity of a host-run invocation (docs/SCE_HOST_FEATURES.md section 1): the session that
/// started it, the `<invoke>`'s id, and which start of that `<invoke>` in that session it is. A
/// restore starts a running invocation again under the identity it had, so two starts of one
/// identity are one operation asked twice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Identity<'a> {
    /// The id of the session that started it (`_sessionid`).
    pub session: &'a str,
    /// The `<invoke>`'s id.
    pub invoke: &'a str,
    /// Which start of the `<invoke>` in the session it is, counted from 1.
    pub start: u64,
}

/// A value as a store keeps it, with the version its write gave the key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KvEntry {
    /// The value's bytes, which the store keeps and never reads: the UTF-8 of the text a document
    /// wrote, or, in a store that wraps another and seals what it keeps (ADR 0109, D8), its sealed
    /// record.
    pub value: Vec<u8>,
    /// The version the write that made it gave the key; never `0`.
    pub version: u64,
}

/// What a put did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Put {
    /// It wrote the value at `version`; `replaced` is whether the key held a value.
    Written { replaced: bool, version: u64 },
    /// The key is new and the store holds as many keys as the quota allows: nothing was written.
    OverQuota,
}

/// What a put-if answered: the result a repeat of its invocation is answered with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PutIf {
    /// It wrote the value, and the key is at `version`.
    Written { version: u64 },
    /// The key was at another version than the one named, `version` (`0` for an absent key), and
    /// nothing was written: a lost race, which is an answer and not a failure.
    Unwritten { version: u64 },
    /// The version matched, the key is new and the store holds as many keys as the quota allows:
    /// nothing was written.
    OverQuota,
    /// The host failed the invocation with `code` before the store was asked
    /// ([`KvStore::keep_first`]): a failure the embedding injected.
    Failed { code: String },
}

impl PutIf {
    /// The text a store keeps the outcome as, beside the version it names.
    pub fn outcome(&self) -> String {
        match self {
            Self::Written { .. } => "written".to_string(),
            Self::Unwritten { .. } => "unwritten".to_string(),
            Self::OverQuota => "over_quota".to_string(),
            Self::Failed { code } => format!("failed:{code}"),
        }
    }

    /// The version the outcome names, `0` for one that names none.
    pub fn version(&self) -> u64 {
        match self {
            Self::Written { version } | Self::Unwritten { version } => *version,
            Self::OverQuota | Self::Failed { .. } => 0,
        }
    }

    /// The outcome a store kept as `outcome` with `version`, or `None` for a text this build does
    /// not write.
    pub fn from_kept(outcome: &str, version: u64) -> Option<Self> {
        match outcome {
            "written" => Some(Self::Written { version }),
            "unwritten" => Some(Self::Unwritten { version }),
            "over_quota" => Some(Self::OverQuota),
            failed => failed.strip_prefix("failed:").map(|code| Self::Failed {
                code: code.to_string(),
            }),
        }
    }
}

/// A store of the key-value feature. See the module documentation for what it keeps and why.
///
/// Every method is one transaction: it reads and writes what it names, or, when it returns an
/// error, writes nothing. A key is compared as its UTF-8 bytes. A handle may be shared between
/// threads; whether handles on one store may live in several processes is the implementation's to
/// say.
pub trait KvStore: Send + Sync {
    /// The value `key` names, or `None` for a key nothing was written under or whose value was
    /// removed.
    fn get(&self, key: &str) -> Result<Option<KvEntry>, Failure>;

    /// Write `value` under `key` at the counter's next number, unless `key` is new and the store
    /// holds `quota` keys or more already ([`Put::OverQuota`], nothing written). `None` is no
    /// quota.
    fn put(&self, key: &str, value: &[u8], quota: Option<u64>) -> Result<Put, Failure>;

    /// Write `value` under `key` only when the key's version is `expected`, `0` meaning that the key
    /// must be absent; `None` names no version (a number past 2^64 - 1) and is never matched. The
    /// quota is judged only for a write that would be made.
    ///
    /// With `identity`, the result is kept under it in the same transaction, and a put-if of an
    /// identity whose result is kept answers that result and writes nothing, whatever the store
    /// holds now: the first result is kept, never replaced.
    fn put_if(
        &self,
        key: &str,
        value: &[u8],
        expected: Option<u64>,
        quota: Option<u64>,
        identity: Option<&Identity<'_>>,
    ) -> Result<PutIf, Failure>;

    /// Remove the value of `key`: `true` when it held one, which takes the counter's next number;
    /// `false`, taking none, when it did not.
    fn delete(&self, key: &str) -> Result<bool, Failure>;

    /// The least key, in ascending order of UTF-8 bytes, that is greater than `after` and begins
    /// with `prefix`; an empty `after` starts at the least key. `None` when no key does.
    fn next_key(&self, after: &str, prefix: &str) -> Result<Option<String>, Failure>;

    /// At most `limit` of the keys the store holds, with their values, in ascending order of their
    /// UTF-8 bytes, each after `after` when it is given: how a host lists its store, and how a pass
    /// over every value reads them a page at a time.
    fn entries_after(
        &self,
        after: Option<&str>,
        limit: usize,
    ) -> Result<Vec<(String, KvEntry)>, Failure>;

    /// The first result kept under `identity`, or `None`.
    fn first_result(&self, identity: &Identity<'_>) -> Result<Option<PutIf>, Failure>;

    /// Keep `result` under `identity` unless a result is kept under it already, and answer the one
    /// kept afterwards: what a host keeps when it failed a put-if before the store was asked.
    fn keep_first(&self, identity: &Identity<'_>, result: &PutIf) -> Result<PutIf, Failure>;

    /// Replace the value of `key` with `value` when it is `held`, its version kept, and answer
    /// whether it did: `false`, writing nothing, when the key holds no value or another. This is
    /// how a store that seals what it keeps seals a value again under its active key (ADR 0109,
    /// D8) without moving its version.
    fn reseal(&self, key: &str, held: &[u8], value: &[u8]) -> Result<bool, Failure>;
}

/// The decisions every key-value store makes, written once, as the session store's are in
/// `crate::rules`: a store that wraps another (a sealing one, ADR 0109 D8) makes them by the same
/// functions.
pub mod rules {
    /// Whether writing a key the store does not hold would hold more keys than `quota` allows,
    /// for a store that holds `held` keys.
    pub fn over_quota(new_key: bool, held: u64, quota: Option<u64>) -> bool {
        new_key && quota.is_some_and(|quota| held >= quota)
    }

    /// Whether a put-if naming `expected` matches a key at `current` (`0` for an absent key).
    pub fn matches(expected: Option<u64>, current: u64) -> bool {
        expected == Some(current)
    }

    /// Whether `key` is one [`super::KvStore::next_key`] may answer for `after` and `prefix`: it
    /// is past `after` (any key, for an empty one) and begins with `prefix`. The answer is the
    /// least such key.
    pub fn walks_to(key: &str, after: &str, prefix: &str) -> bool {
        (after.is_empty() || key > after) && key.starts_with(prefix)
    }
}
