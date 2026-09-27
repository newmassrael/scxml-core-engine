// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What the router core tells the engine besides the envelopes themselves:
//! the `error.communication` rows of SCE_MESH.md §mesh-16.7 that the core,
//! rather than a transport, is in a position to observe.
//!
//! One enum for both halves because they are rows of one table and reach
//! the document as one event, `error.communication`, told apart by reason.
//! The core names the row and carries its extras; turning a signal into the
//! event is the host's, since only the host holds the engine.

use alloc::string::String;

/// One §mesh-16.7 row, with the extras the row carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Signal {
    /// ORDERING_GAP: `source`'s sequences `lost_lo..=lost_hi` waited out the
    /// gap timeout and will not be delivered.
    OrderingGap {
        source: String,
        lost_lo: u64,
        lost_hi: u64,
    },
    /// DEDUP_WINDOW_OVERFLOW (row 7): a novel id from `source` was admitted
    /// to a full window, so its oldest id is no longer remembered.
    DedupWindowOverflow { source: String },
    /// BACKPRESSURE_DROP: an envelope that could not be sent at once found
    /// its target's queue already holding `depth` envelopes, and was dropped.
    BackpressureDrop { depth: u32 },
    /// OUTBOUND_STALE_DROP (row 15): a queued envelope had waited `age_ms`
    /// when its transport became ready, past the bound of `max_age_ms`.
    OutboundStaleDrop { age_ms: i64, max_age_ms: i64 },
    /// TRANSPORT_UNAVAILABLE (row 1): a ready target stopped being ready.
    TransportUnavailable,
    /// SEND_FAILED (row 2): the transport declined a send and no retry is
    /// configured, so the envelope is gone. `transport_error` is the
    /// transport's own words, when it gave any.
    SendFailed { transport_error: Option<String> },
    /// DELIVERY_EXHAUSTED (row 3): a send was given up after `attempts`
    /// sends, because the failure was terminal or the retries ran out.
    DeliveryExhausted {
        attempts: u32,
        transport_error: Option<String>,
    },
}
