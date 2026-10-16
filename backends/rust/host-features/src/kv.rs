// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The key-value store (docs/SCE_HOST_FEATURES.md, section 3), in memory.

use std::collections::BTreeMap;
use std::ops::Bound;

use crate::{failure, record, Outcome, ReferenceHost, Transport};
use serde_json::json;

/// Keys and values, with the quota the embedding set. A `BTreeMap` of `String`
/// orders keys by their UTF-8 bytes, which is the order the specification gives.
#[derive(Default)]
pub(crate) struct Store {
    pub(crate) keys: BTreeMap<String, String>,
    pub(crate) quota: Option<usize>,
}

impl Store {
    pub(crate) fn entries(&self) -> Vec<(String, String)> {
        self.keys
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect()
    }
}

/// A key is one to 256 bytes (the record's own bound) and holds no U+0000.
pub(crate) fn valid_key(key: &str) -> bool {
    !key.is_empty() && !key.contains('\0')
}

pub(crate) fn get<T: Transport>(host: &mut ReferenceHost<T>, key: &str) -> Outcome {
    match host.store.keys.get(key) {
        Some(value) => record(json!({ "found": true, "value": value })),
        None => record(json!({ "found": false, "value": "" })),
    }
}

pub(crate) fn put<T: Transport>(host: &mut ReferenceHost<T>, key: &str, value: &str) -> Outcome {
    let store = &mut host.store;
    let replaced = store.keys.contains_key(key);
    if !replaced && store.quota.is_some_and(|quota| store.keys.len() >= quota) {
        return failure("sce-host-kv-put", "quota_exceeded");
    }
    store.keys.insert(key.to_string(), value.to_string());
    record(json!({ "replaced": replaced }))
}

pub(crate) fn delete<T: Transport>(host: &mut ReferenceHost<T>, key: &str) -> Outcome {
    let existed = host.store.keys.remove(key).is_some();
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
