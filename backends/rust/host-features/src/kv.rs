// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The key-value store (docs/SCE_HOST_FEATURES.md, section 3), in memory.

use std::collections::BTreeMap;
use std::ops::Bound;

use crate::{failure, record, Outcome, ReferenceHost, Transport};
use serde_json::json;

/// A stored value and the version its write gave the key.
pub(crate) struct Entry {
    value: String,
    version: u64,
}

/// Keys and values, with the quota the embedding set. A `BTreeMap` of `String`
/// orders keys by their UTF-8 bytes, which is the order the specification gives.
#[derive(Default)]
pub(crate) struct Store {
    pub(crate) keys: BTreeMap<String, Entry>,
    pub(crate) quota: Option<usize>,
    /// The last number a write or a removal took: versions are numbered from
    /// this one counter of the store (docs/SCE_HOST_FEATURES.md, section 8), so a
    /// key is never given a version it had, even after it was removed.
    last_version: u64,
}

impl Store {
    pub(crate) fn entries(&self) -> Vec<(String, String)> {
        self.keys
            .iter()
            .map(|(key, entry)| (key.clone(), entry.value.clone()))
            .collect()
    }

    /// The counter's next number, taken.
    fn next_version(&mut self) -> u64 {
        self.last_version += 1;
        self.last_version
    }

    /// The version `key` is at: `0` for a key the store does not hold.
    fn version_of(&self, key: &str) -> u64 {
        self.keys.get(key).map_or(0, |entry| entry.version)
    }

    /// Whether writing `key` would hold one key more than the quota allows.
    fn over_quota(&self, key: &str) -> bool {
        !self.keys.contains_key(key) && self.quota.is_some_and(|quota| self.keys.len() >= quota)
    }

    /// Keep `value` under `key` at the counter's next number; that number.
    fn write(&mut self, key: &str, value: &str) -> u64 {
        let version = self.next_version();
        self.keys.insert(
            key.to_string(),
            Entry {
                value: value.to_string(),
                version,
            },
        );
        version
    }
}

/// A key is one to 256 bytes (the record's own bound) and holds no U+0000.
pub(crate) fn valid_key(key: &str) -> bool {
    !key.is_empty() && !key.contains('\0')
}

/// An expected version is one or more of the digits `0` to `9`, and nothing
/// else.
pub(crate) fn valid_version(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit())
}

pub(crate) fn get<T: Transport>(host: &mut ReferenceHost<T>, key: &str) -> Outcome {
    match host.store.keys.get(key) {
        Some(entry) => record(json!({
            "found": true,
            "value": entry.value,
            "version": entry.version.to_string(),
        })),
        None => record(json!({ "found": false, "value": "", "version": "0" })),
    }
}

pub(crate) fn put<T: Transport>(host: &mut ReferenceHost<T>, key: &str, value: &str) -> Outcome {
    let store = &mut host.store;
    if store.over_quota(key) {
        return failure("sce-host-kv-put", "quota_exceeded");
    }
    let replaced = store.keys.contains_key(key);
    store.write(key, value);
    record(json!({ "replaced": replaced }))
}

/// Write `value` under `key` only when the key is at `expected`, a text of
/// digits the refusal already judged. It is compared as a number, so leading
/// zeros do not count, and one past `u64::MAX` names no version.
pub(crate) fn put_if<T: Transport>(
    host: &mut ReferenceHost<T>,
    key: &str,
    value: &str,
    expected: &str,
) -> Outcome {
    let store = &mut host.store;
    let current = store.version_of(key);
    if expected.parse::<u64>().ok() != Some(current) {
        return record(json!({ "written": false, "version": current.to_string() }));
    }
    if store.over_quota(key) {
        return failure("sce-host-kv-put-if", "quota_exceeded");
    }
    let version = store.write(key, value);
    record(json!({ "written": true, "version": version.to_string() }))
}

pub(crate) fn delete<T: Transport>(host: &mut ReferenceHost<T>, key: &str) -> Outcome {
    let store = &mut host.store;
    let existed = store.keys.remove(key).is_some();
    if existed {
        store.next_version();
    }
    record(json!({ "existed": existed }))
}

pub(crate) fn next_key<T: Transport>(
    host: &mut ReferenceHost<T>,
    after: &str,
    prefix: &str,
) -> Outcome {
    // The least key past `after` (all keys when it is empty) that begins with the
    // prefix: keys with the prefix are contiguous in this order, so the first one
    // found is the answer, and the first one that does not begin with it, once
    // the prefix's own range has been left, ends the walk.
    let from = if after.is_empty() || after < prefix {
        Bound::Included(prefix)
    } else {
        Bound::Excluded(after)
    };
    let found = host
        .store
        .keys
        .range::<str, _>((from, Bound::Unbounded))
        .map(|(key, _)| key)
        .next()
        .filter(|key| key.starts_with(prefix));
    match found {
        Some(key) => record(json!({ "found": true, "key": key })),
        None => record(json!({ "found": false, "key": "" })),
    }
}
