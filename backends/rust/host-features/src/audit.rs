// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The audit log (docs/SCE_HOST_FEATURES.md, section 15; docs/adr/0109, D14):
//! a statechart appends a typed entry to a stream of its host's durable log.
//!
//! `sce-host-audit-append` takes the stream's name, the entry (a record of
//! the schema the invoke's `sce:entry` names, as its JSON payload) and the
//! marks its schema declares, and answers the sequence the entry was appended
//! at. Before the entry is written the host redacts it by the marks
//! ([`redact`]): a `secret` field is replaced by [`SECRET_MARKER`], a
//! `personal` text by [`PERSONAL_PREFIX`] and the lowercase hexadecimal of its
//! HMAC-SHA-256 under the key ring's pseudonymisation key, unless the stream
//! keeps personal values in the clear ([`Personal::Clear`]). The entry is then
//! written in its canonical form ([`canonical_json`]), chained, checkpointed
//! every `checkpoint_every` rows under the host's audit key, and durable
//! before the answer (`sce_rust_crypto::audit`).
//!
//! A stream is one the embedding configured ([`StreamPolicy`]), which states
//! how long its entries are kept; an append to another fails `unknown_stream`.
//! An append repeated under the identity of its invocation answers the first
//! sequence and appends nothing.

use std::collections::BTreeMap;

use sce_rust_crypto::audit::{AuditError, Writer};
use sce_rust_crypto::hex;
use sce_rust_crypto::keyring::KeyError;
use serde_json::{json, Value};

use crate::cose::UNAVAILABLE;
use crate::{failure, record, Invocation, Outcome, Params, ReferenceHost, Transport};

/// The operation that appends an entry to a stream of the audit log.
pub const AUDIT_APPEND: &str = "sce-host-audit-append";

/// The operation of the feature.
pub const OPERATIONS: [&str; 1] = [AUDIT_APPEND];

/// The bound of a request's `stream`, in bytes.
pub const STREAM_MAX: usize = 64;
/// The bound of a request's `entry`, in bytes (`audit_entry`).
pub const ENTRY_MAX: usize = 4096;
/// The bound of a request's `sensitivity`, in bytes (`audit_marks`).
pub const MARKS_MAX: usize = 1024;

/// What a `secret` field holds in a written entry.
pub const SECRET_MARKER: &str = "sce-secret";
/// What a `personal` text written as its keyed digest begins with.
pub const PERSONAL_PREFIX: &str = "sce-personal:";

/// The failure of an append to a stream the host keeps no policy for.
const UNKNOWN_STREAM: &str = "unknown_stream";

/// How a stream writes a `personal` field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Personal {
    /// As its keyed digest under the pseudonymisation key: equal values
    /// correlate, and none is in the clear.
    Pseudonymised,
    /// As it is: a stream whose readers may see who the entries are of.
    Clear,
}

/// What a host keeps of one stream: how often it signs a checkpoint, how long
/// its entries are kept, and how it writes a personal value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamPolicy {
    /// A checkpoint every this many rows; `0` signs only at a stop.
    pub checkpoint_every: u64,
    /// How many days an entry is kept before retention may delete its segment.
    pub retain_days: u32,
    pub personal: Personal,
}

/// The streams a host keeps, by name.
pub type AuditStreams = BTreeMap<String, StreamPolicy>;

/// A mark a schema declares on a field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mark {
    Secret,
    Personal,
}

/// Why an entry was not redacted.
#[derive(Debug)]
enum Unredacted {
    /// The request is no entry the marks can be applied to: `invalid_argument`.
    Refused,
    /// The pseudonymisation key could not be used.
    Key(KeyError),
}

/// The marks `text` writes, each the segments of a field's pointer and its
/// mark, or `None` for a text that is not marks: `/a=secret;/b/c=personal`,
/// a segment `*` naming every element of a list. Empty for no marks.
fn marks(text: &str) -> Option<Vec<(Vec<&str>, Mark)>> {
    if text.is_empty() {
        return Some(Vec::new());
    }
    text.split(';')
        .map(|one| {
            let (pointer, mark) = one.split_once('=')?;
            let mark = match mark {
                "secret" => Mark::Secret,
                "personal" => Mark::Personal,
                _ => return None,
            };
            let segments: Vec<&str> = pointer.strip_prefix('/')?.split('/').collect();
            segments
                .iter()
                .all(|segment| !segment.is_empty())
                .then_some((segments, mark))
        })
        .collect()
}

/// Apply `mark` to the member `segments` names in `value`. A member that is
/// `null` on the way holds nothing to redact; a member the entry lacks, or a
/// personal mark on a value that is no text, refuses the request.
fn apply(
    value: &mut Value,
    segments: &[&str],
    mark: Mark,
    personal: Personal,
    pseudonym: &mut dyn FnMut(&str) -> Result<String, KeyError>,
) -> Result<(), Unredacted> {
    let Some((first, rest)) = segments.split_first() else {
        return Err(Unredacted::Refused);
    };
    if value.is_null() {
        return Ok(());
    }
    if *first == "*" {
        let Value::Array(elements) = value else {
            return Err(Unredacted::Refused);
        };
        for element in elements {
            if rest.is_empty() {
                redact_value(element, mark, personal, pseudonym)?;
            } else {
                apply(element, rest, mark, personal, pseudonym)?;
            }
        }
        return Ok(());
    }
    let Value::Object(members) = value else {
        return Err(Unredacted::Refused);
    };
    let Some(member) = members.get_mut(*first) else {
        return Err(Unredacted::Refused);
    };
    if rest.is_empty() {
        redact_value(member, mark, personal, pseudonym)
    } else {
        apply(member, rest, mark, personal, pseudonym)
    }
}

/// Write `value` as `mark` says.
fn redact_value(
    value: &mut Value,
    mark: Mark,
    personal: Personal,
    pseudonym: &mut dyn FnMut(&str) -> Result<String, KeyError>,
) -> Result<(), Unredacted> {
    match mark {
        Mark::Secret => {
            *value = Value::String(SECRET_MARKER.to_string());
            Ok(())
        }
        // An absent personal value names no one.
        Mark::Personal if value.is_null() => Ok(()),
        Mark::Personal => {
            let Some(text) = value.as_str().map(str::to_owned) else {
                return Err(Unredacted::Refused);
            };
            if personal == Personal::Pseudonymised {
                let digest = pseudonym(&text).map_err(Unredacted::Key)?;
                *value = Value::String(format!("{PERSONAL_PREFIX}{digest}"));
            }
            Ok(())
        }
    }
}

/// The entry `entry`, a JSON object, redacted by `marks` (the text the build
/// writes into `sensitivity`), each personal text in a stream that
/// pseudonymises written as the lowercase hexadecimal `pseudonym` gives of it,
/// and written in its canonical form ([`canonical_json`]). `None` for an
/// entry that is no object or marks that are none, or name a member the entry
/// lacks.
pub fn redact(
    entry: &str,
    marks_text: &str,
    personal: Personal,
    pseudonym: &mut dyn FnMut(&str) -> Result<String, KeyError>,
) -> Result<Option<String>, KeyError> {
    match redacted(entry, marks_text, personal, pseudonym) {
        Ok(text) => Ok(Some(text)),
        Err(Unredacted::Refused) => Ok(None),
        Err(Unredacted::Key(error)) => Err(error),
    }
}

fn redacted(
    entry: &str,
    marks_text: &str,
    personal: Personal,
    pseudonym: &mut dyn FnMut(&str) -> Result<String, KeyError>,
) -> Result<String, Unredacted> {
    let mut value: Value = serde_json::from_str(entry).map_err(|_| Unredacted::Refused)?;
    if !value.is_object() {
        return Err(Unredacted::Refused);
    }
    for (segments, mark) in marks(marks_text).ok_or(Unredacted::Refused)? {
        apply(&mut value, &segments, mark, personal, pseudonym)?;
    }
    Ok(canonical_json(&value))
}

/// `value` written in the canonical form of an audit entry
/// (docs/SCE_AUDIT_LOG.md section 2): no whitespace, an object's members in
/// the order of their names' UTF-8 bytes at every depth, a string with `"`,
/// `\` and every character below U+0020 escaped (`\n`, `\r`, `\t`, and
/// `\u00XX` in lowercase for the rest) and everything else as it is, a number
/// as its JSON text.
pub fn canonical_json(value: &Value) -> String {
    let mut out = String::new();
    write_canonical(value, &mut out);
    out
}

fn write_canonical(value: &Value, out: &mut String) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(true) => out.push_str("true"),
        Value::Bool(false) => out.push_str("false"),
        Value::Number(number) => out.push_str(&number.to_string()),
        Value::String(text) => write_string(text, out),
        Value::Array(elements) => {
            out.push('[');
            for (n, element) in elements.iter().enumerate() {
                if n > 0 {
                    out.push(',');
                }
                write_canonical(element, out);
            }
            out.push(']');
        }
        Value::Object(members) => {
            let mut names: Vec<&String> = members.keys().collect();
            names.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
            out.push('{');
            for (n, name) in names.into_iter().enumerate() {
                if n > 0 {
                    out.push(',');
                }
                write_string(name, out);
                out.push(':');
                write_canonical(&members[name.as_str()], out);
            }
            out.push('}');
        }
    }
}

fn write_string(text: &str, out: &mut String) {
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if u32::from(c) < 0x20 => out.push_str(&format!("\\u{:04x}", u32::from(c))),
            c => out.push(c),
        }
    }
    out.push('"');
}

/// The request `params` makes, or `None` when the host refuses it: a stream
/// empty or past its bound, an entry past its bound, marks past theirs, or an
/// entry the marks cannot be applied to.
fn request(params: &Params) -> Option<(&str, &str, &str)> {
    let stream = params.get("stream").map(String::as_str).unwrap_or("");
    let entry = params.get("entry").map(String::as_str).unwrap_or("");
    let marks_text = params.get("sensitivity").map(String::as_str).unwrap_or("");
    if stream.is_empty()
        || stream.len() > STREAM_MAX
        || entry.len() > ENTRY_MAX
        || marks_text.len() > MARKS_MAX
    {
        return None;
    }
    // A dry run of the redaction, whose digests are never written.
    let mut nothing = |_: &str| -> Result<String, KeyError> { Ok(String::new()) };
    redacted(entry, marks_text, Personal::Pseudonymised, &mut nothing).ok()?;
    Some((stream, entry, marks_text))
}

/// Whether the host refuses an append with `params`.
pub(crate) fn refuses(params: &Params) -> bool {
    request(params).is_none()
}

/// Append the entry of `invocation`'s request, one [`refuses`] accepted.
pub(crate) fn perform<T: Transport>(
    host: &mut ReferenceHost<T>,
    invocation: &Invocation,
    params: &Params,
) -> Outcome {
    let Some((stream, entry, marks_text)) = request(params) else {
        return Outcome::Refuse(crate::INVALID_ARGUMENT);
    };
    let Some(policy) = host.audit_streams.get(stream).cloned() else {
        sce_rust_runtime::sce_log_info!(
            "{AUDIT_APPEND} {}: no stream '{stream}' is kept",
            invocation.id
        );
        return failure(AUDIT_APPEND, UNKNOWN_STREAM);
    };
    let Some(now_ms) = crate::clock::utc_now(host).and_then(|ms| u64::try_from(ms).ok()) else {
        sce_rust_runtime::sce_log_info!(
            "{AUDIT_APPEND} {}: the clock gave no time to append at",
            invocation.id
        );
        return failure(AUDIT_APPEND, UNAVAILABLE);
    };
    let ReferenceHost {
        keyring,
        random,
        audit,
        ..
    } = host;
    let identity = crate::kv::identity(invocation);
    // A repeat answers the first sequence before anything is drawn or digested.
    match audit.audit_first(&identity) {
        Ok(Some(sequence)) => return record(json!({ "sequence": sequence.to_string() })),
        Ok(None) => {}
        Err(error) => {
            sce_rust_runtime::sce_log_error!("{AUDIT_APPEND} {}: {error}", invocation.id);
            return failure(AUDIT_APPEND, UNAVAILABLE);
        }
    }
    let mut pseudonym = |text: &str| -> Result<String, KeyError> {
        keyring
            .audit_pseudonym(text.as_bytes(), now_ms, random.as_mut())
            .map(|digest| hex(&digest))
    };
    let written = match redact(entry, marks_text, policy.personal, &mut pseudonym) {
        Ok(Some(written)) => written,
        Ok(None) => return Outcome::Refuse(crate::INVALID_ARGUMENT),
        Err(error) => {
            sce_rust_runtime::sce_log_info!(
                "{AUDIT_APPEND} {}: no pseudonym, {error}",
                invocation.id
            );
            return failure(AUDIT_APPEND, AuditError::Key(error).code());
        }
    };
    let mut writer = Writer {
        store: audit.as_ref(),
        keyring,
        random: random.as_mut(),
        now_ms,
    };
    match writer.append(
        stream,
        written.as_bytes(),
        policy.checkpoint_every,
        Some(&identity),
    ) {
        Ok(sequence) => record(json!({ "sequence": sequence.to_string() })),
        Err(error) => {
            sce_rust_runtime::sce_log_error!("{AUDIT_APPEND} {}: {error}", invocation.id);
            failure(AUDIT_APPEND, error.code())
        }
    }
}
