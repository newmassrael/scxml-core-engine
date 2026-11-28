// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A page of the event journal (docs/SCE_HOST_FEATURES.md, section 16;
//! docs/adr/0114, D5): `sce-host-journal-read` answers the entries of one
//! stream at or below a position whose valid time is at or before a moment,
//! ordered by when each holds from and then by position, after a cursor.
//!
//! The host reads the journal store it is given
//! ([`crate::ReferenceHost::set_journal_store`]): a read is one transaction
//! of it, and every answer names the store's horizon, the position at or below
//! which nothing can still become visible. A read as of position 0 is read as
//! of the last position the horizon has passed, which the answer names, so the
//! pages of one reading are of one moment and never change
//! (`sce_rust_session_store::JournalStore::as_of`). The page crosses as its
//! deterministic CBOR (`sce_rust_runtime::journal::write_page`), which the
//! machine opens into the facts of the union its read names.

use sce_rust_session_store::{AsOf, AsOfCursor, AsOfError, Stream};
use serde_json::json;

use crate::cose::UNAVAILABLE;
use crate::{failure, record, NamedBounds, Outcome, Params, ReferenceHost, Transport};

/// The operation that reads a page of the event journal.
pub const JOURNAL_READ: &str = "sce-host-journal-read";

/// The bound of a request's `stream`, in bytes.
pub const STREAM_MAX: usize = 64;
/// The bound of a request's `key`, in bytes.
pub const KEY_MAX: usize = 256;

/// The failure of a page whose bytes pass the bound of the result's `entries`:
/// the reader asks again for fewer entries.
const TOO_LARGE: &str = "response_too_large";

/// What a read asks, as its request's fields give it.
struct Read<'a> {
    stream: Stream<'a>,
    as_of_position: u64,
    valid_ms: i64,
    after: AsOfCursor,
    limit: usize,
}

/// The read `params` makes, or `None` when the host refuses it: a stream empty or
/// past its bound, a key on an observed stream, a number that is none, or a limit
/// of 0 or past `journal_page`. A machine's stream may be read under the empty
/// key: the stream of the one session of a pool that keeps one (deploy.yaml
/// `sessions: { key_from: none }`, docs/adr/0114, D4).
fn request<'a>(params: &'a Params, bounds: &NamedBounds) -> Option<Read<'a>> {
    fn text<'p>(params: &'p Params, name: &str) -> &'p str {
        params.get(name).map(String::as_str).unwrap_or("")
    }
    let stream = text(params, "stream");
    let key = text(params, "key");
    let observed = match text(params, "observed") {
        "true" => true,
        "false" => false,
        _ => return None,
    };
    if stream.is_empty() || stream.len() > STREAM_MAX || key.len() > KEY_MAX {
        return None;
    }
    if observed && !key.is_empty() {
        return None;
    }
    let as_of_position = text(params, "as_of_position").parse::<u64>().ok()?;
    let valid_ms = text(params, "valid_ms").parse::<i64>().ok()?;
    let after_valid_ms = text(params, "after_valid_ms").parse::<i64>().ok()?;
    let after_position = text(params, "after_position").parse::<u64>().ok()?;
    let limit = text(params, "limit").parse::<usize>().ok()?;
    if limit == 0 || limit > bounds.journal_page {
        return None;
    }
    let stream = if observed {
        Stream::Observed { name: stream }
    } else {
        Stream::Session {
            machine: stream,
            key,
        }
    };
    // No position is 0, so a cursor at 0 is before every entry.
    let after = if after_position == 0 {
        AsOfCursor::START
    } else {
        AsOfCursor {
            valid_ms: after_valid_ms,
            position: after_position,
        }
    };
    Some(Read {
        stream,
        as_of_position,
        valid_ms,
        after,
        limit,
    })
}

/// Whether the host refuses a read with `params`.
pub(crate) fn refuses(params: &Params, bounds: &NamedBounds) -> bool {
    request(params, bounds).is_none()
}

/// Read the page of the request, one [`refuses`] accepted.
pub(crate) fn perform<T: Transport>(host: &mut ReferenceHost<T>, params: &Params) -> Outcome {
    let bounds = host.bounds;
    let Some(read) = request(params, &bounds) else {
        return Outcome::Refuse(crate::INVALID_ARGUMENT);
    };
    let store = &host.journal;
    let position = if read.as_of_position == 0 {
        match store.position_at(u64::MAX) {
            Ok(position) => position,
            Err(error) => {
                sce_rust_runtime::sce_log_error!("{JOURNAL_READ}: {error}");
                return failure(JOURNAL_READ, UNAVAILABLE);
            }
        }
    } else {
        read.as_of_position
    };
    let as_of = AsOf {
        position,
        valid_ms: read.valid_ms,
    };
    let page = match store.as_of(read.stream, &as_of, read.after, read.limit) {
        Ok(page) => page,
        // A position above the horizon was never answered by a read: the
        // request names a moment the journal may still change at.
        Err(AsOfError::PastHorizon(_)) => return Outcome::Refuse(crate::INVALID_ARGUMENT),
        Err(AsOfError::Failed(error)) => {
            sce_rust_runtime::sce_log_error!("{JOURNAL_READ}: {error}");
            return failure(JOURNAL_READ, UNAVAILABLE);
        }
    };
    let entries: Vec<sce_rust_runtime::journal::PageEntry<'_>> = page
        .entries
        .iter()
        .map(|entry| sce_rust_runtime::journal::PageEntry {
            position: entry.position,
            recorded_ms: entry.recorded_ms,
            holds_ms: entry.holds_from(),
            origin: entry.origin.as_deref().unwrap_or(""),
            schema: &entry.schema,
            revision: entry.revision,
            data: &entry.data,
        })
        .collect();
    let max_entries = u32::try_from(bounds.journal_page).unwrap_or(u32::MAX);
    let max_bytes = u64::try_from(bounds.journal_page_bytes).unwrap_or(u64::MAX);
    match sce_rust_runtime::journal::write_page(&entries, max_entries, max_bytes) {
        Ok(bytes) => record(json!({
            "horizon": page.horizon.to_string(),
            "as_of_position": position.to_string(),
            "entries": sce_rust_runtime::event_payload::bytes_as_payload_text(&bytes),
        })),
        Err(why) => {
            sce_rust_runtime::sce_log_info!("{JOURNAL_READ}: the page {why}");
            failure(JOURNAL_READ, TOO_LARGE)
        }
    }
}
