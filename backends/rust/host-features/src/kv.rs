// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The key-value store (docs/SCE_HOST_FEATURES.md, section 3), over the store
//! the host was built with (`sce_rust_session_store::KvStore`; docs/adr/0109,
//! D13): in memory unless the embedding gives a durable one.
//!
//! The semantics are the store contract's; this module reads a request's
//! arguments, asks the store, and writes the result record. A store that
//! cannot be used answers every operation `unavailable`, and changes nothing.

use sce_rust_session_store::{Failure, Identity, KvStore, Put, PutIf};
use serde_json::json;

use crate::{failure, record, Invocation, Outcome, ReferenceHost, Transport};

/// The operation whose first result a repeat of its invocation is answered
/// with (docs/SCE_HOST_FEATURES.md section 3).
pub(crate) const PUT_IF: &str = "sce-host-kv-put-if";

/// How many values one page of a listing reads.
const PAGE: usize = 256;

/// A key is one to 256 bytes (the record's own bound) and holds no U+0000.
pub(crate) fn valid_key(key: &str) -> bool {
    !key.is_empty() && !key.contains('\0')
}

/// An expected version is one or more of the digits `0` to `9`, and nothing
/// else.
pub(crate) fn valid_version(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit())
}

/// The identity a put-if's first result is kept under.
pub(crate) fn identity(invocation: &Invocation) -> Identity<'_> {
    Identity {
        session: &invocation.session_id,
        invoke: &invocation.id,
        start: invocation.start,
    }
}

/// The failure `unavailable` of `operation`, for a store that could not be
/// used: what it failed with is the host's log.
fn unavailable(operation: &str, why: &Failure) -> Outcome {
    sce_rust_runtime::sce_log_error!("{operation}: the key-value store failed: {why}");
    failure(operation, "unavailable")
}

/// The completion a put-if's result is: a record, or its failure.
pub(crate) fn put_if_outcome(result: &PutIf) -> Outcome {
    match result {
        PutIf::Written { version } => {
            record(json!({ "written": true, "version": version.to_string() }))
        }
        PutIf::Unwritten { version } => {
            record(json!({ "written": false, "version": version.to_string() }))
        }
        PutIf::OverQuota => failure(PUT_IF, "quota_exceeded"),
        PutIf::Failed { code } => failure(PUT_IF, code),
    }
}

/// The first result kept for `invocation`'s put-if, or `None` for a start
/// that is not a repeat; a store that cannot say answers `unavailable`, which
/// is no result and is judged again when asked again.
pub(crate) fn first<T: Transport>(
    host: &ReferenceHost<T>,
    invocation: &Invocation,
) -> Option<Outcome> {
    match host.kv.first_result(&identity(invocation)) {
        Ok(kept) => kept.as_ref().map(put_if_outcome),
        Err(why) => Some(unavailable(PUT_IF, &why)),
    }
}

/// Keep the failure `code` the embedding injected as the first result of
/// `invocation`'s put-if, and answer the result kept: a repeat is answered as
/// the first start was.
pub(crate) fn keep_failure<T: Transport>(
    host: &ReferenceHost<T>,
    invocation: &Invocation,
    code: &str,
) -> Outcome {
    let failed = PutIf::Failed {
        code: code.to_string(),
    };
    match host.kv.keep_first(&identity(invocation), &failed) {
        Ok(kept) => put_if_outcome(&kept),
        Err(why) => unavailable(PUT_IF, &why),
    }
}

pub(crate) fn get<T: Transport>(host: &ReferenceHost<T>, key: &str) -> Outcome {
    match host.kv.get(key) {
        Ok(Some(entry)) => match String::from_utf8(entry.value) {
            Ok(value) => record(json!({
                "found": true,
                "value": value,
                "version": entry.version.to_string(),
            })),
            Err(_) => unavailable(
                "sce-host-kv-get",
                &Failure::new(format!(
                    "the value of '{key}' is not the text it was written as"
                )),
            ),
        },
        Ok(None) => record(json!({ "found": false, "value": "", "version": "0" })),
        Err(why) => unavailable("sce-host-kv-get", &why),
    }
}

pub(crate) fn put<T: Transport>(host: &ReferenceHost<T>, key: &str, value: &str) -> Outcome {
    match host.kv.put(key, value.as_bytes(), host.quota) {
        Ok(Put::Written { replaced, .. }) => record(json!({ "replaced": replaced })),
        Ok(Put::OverQuota) => failure("sce-host-kv-put", "quota_exceeded"),
        Err(why) => unavailable("sce-host-kv-put", &why),
    }
}

/// Write `value` under `key` only when the key is at `expected`, a text of
/// digits the refusal already judged, and keep the result under the
/// invocation's identity in the same transaction. It is compared as a number,
/// so leading zeros do not count, and one past `u64::MAX` names no version.
pub(crate) fn put_if<T: Transport>(
    host: &ReferenceHost<T>,
    invocation: &Invocation,
    key: &str,
    value: &str,
    expected: &str,
) -> Outcome {
    match host.kv.put_if(
        key,
        value.as_bytes(),
        expected.parse::<u64>().ok(),
        host.quota,
        Some(&identity(invocation)),
    ) {
        Ok(result) => put_if_outcome(&result),
        Err(why) => unavailable(PUT_IF, &why),
    }
}

pub(crate) fn delete<T: Transport>(host: &ReferenceHost<T>, key: &str) -> Outcome {
    match host.kv.delete(key) {
        Ok(existed) => record(json!({ "existed": existed })),
        Err(why) => unavailable("sce-host-kv-delete", &why),
    }
}

pub(crate) fn next_key<T: Transport>(
    host: &ReferenceHost<T>,
    after: &str,
    prefix: &str,
) -> Outcome {
    match host.kv.next_key(after, prefix) {
        Ok(Some(key)) => record(json!({ "found": true, "key": key })),
        Ok(None) => record(json!({ "found": false, "key": "" })),
        Err(why) => unavailable("sce-host-kv-next-key", &why),
    }
}

/// Every key `store` holds with its value as text, in key order, or why the
/// store could not list them.
pub(crate) fn entries(store: &dyn KvStore) -> Result<Vec<(String, String)>, Failure> {
    let mut listed = Vec::new();
    let mut after: Option<String> = None;
    loop {
        let page = store.entries_after(after.as_deref(), PAGE)?;
        let Some((last, _)) = page.last() else {
            break;
        };
        after = Some(last.clone());
        listed.extend(page.into_iter().map(|(key, entry)| {
            let value = String::from_utf8_lossy(&entry.value).into_owned();
            (key, value)
        }));
    }
    Ok(listed)
}
