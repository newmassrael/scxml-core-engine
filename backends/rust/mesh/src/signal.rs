// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What the router core tells the engine besides the envelopes themselves:
//! the `error.communication` rows of SCE_MESH.md §mesh-16.7 that the core,
//! rather than a transport, is in a position to observe.
//!
//! One enum for both halves because they are rows of one table and reach
//! the document as one event, `error.communication`, told apart by reason.
//! The core names the row and carries its extras; [`Signal::event_data`]
//! renders the `_event.data` the document reads, and raising the event is
//! the host's, since only the host holds the engine.

use alloc::string::String;

use sce_rust_runtime::helpers::event_data::escape_json_string;

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
    /// to a full window of `window_size` ids, so its oldest id is no longer
    /// remembered.
    DedupWindowOverflow { source: String, window_size: u32 },
    /// ENVELOPE_CORRUPT (row 4): what arrived could not be read. `codec` is
    /// the row's field: `"cbor"` when the bytes are not an envelope the
    /// standard document accepts, or the payload's own codec when the
    /// envelope decoded but its payload is not one the engine can be handed
    /// — in which case the envelope also named its `source`.
    EnvelopeCorrupt {
        source: Option<String>,
        codec: &'static str,
    },
    /// MISSING_SEQUENCE (row 11): an envelope reached an ordered binding
    /// without a `sequence_no`, so it cannot be placed (§10.6.3).
    MissingSequence { source: String },
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

/// The deployment binding a row is observed on: the peer it names and the
/// transport kind that carries it (`"wss"`, `"custom_tcp"`, ...). A send to
/// a peer the deployment never bound names the peer and no transport.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Binding<'a> {
    pub peer: &'a str,
    pub transport: Option<&'a str>,
}

impl Signal {
    /// The row's `reason`, as §mesh-16.7 spells it.
    pub fn reason(&self) -> &'static str {
        match self {
            Signal::OrderingGap { .. } => "ORDERING_GAP",
            Signal::DedupWindowOverflow { .. } => "DEDUP_WINDOW_OVERFLOW",
            Signal::EnvelopeCorrupt { .. } => "ENVELOPE_CORRUPT",
            Signal::MissingSequence { .. } => "MISSING_SEQUENCE",
            Signal::BackpressureDrop { .. } => "BACKPRESSURE_DROP",
            Signal::OutboundStaleDrop { .. } => "OUTBOUND_STALE_DROP",
            Signal::TransportUnavailable => "TRANSPORT_UNAVAILABLE",
            Signal::SendFailed { .. } => "SEND_FAILED",
            Signal::DeliveryExhausted { .. } => "DELIVERY_EXHAUSTED",
        }
    }

    /// The `_event.data` of the `error.communication` this row raises, as
    /// §mesh-10.7.1 and the row's own columns shape it.
    ///
    /// Byte for byte what the C++ core's `CommunicationError::toJsonBytes`
    /// (sce/include/mesh/CommunicationError.h) writes for the same row:
    /// `errorName`, `reason`, then each present field in that header's
    /// declaration order, absent ones omitted rather than `null`. A
    /// document reading `_event.data` must not be able to tell which core
    /// raised it.
    ///
    /// `binding` supplies `target` and `transport` to the rows whose columns
    /// name them; a row observed with no binding (a gap ended by a tick)
    /// has neither column.
    pub fn event_data(&self, binding: Option<Binding<'_>>) -> String {
        let mut json = Json::new(self.reason());
        let (source, sending, transport_row) = match self {
            Signal::OrderingGap { source, .. }
            | Signal::DedupWindowOverflow { source, .. }
            | Signal::MissingSequence { source } => (Some(source.as_str()), false, false),
            Signal::EnvelopeCorrupt { source, .. } => (source.as_deref(), false, true),
            Signal::BackpressureDrop { .. }
            | Signal::OutboundStaleDrop { .. }
            | Signal::TransportUnavailable
            | Signal::SendFailed { .. }
            | Signal::DeliveryExhausted { .. } => (None, true, true),
        };
        if let Some(source) = source {
            json.string("source", source);
        }
        if let Some(binding) = binding {
            if sending {
                json.string("target", binding.peer);
            }
            if let (true, Some(transport)) = (transport_row, binding.transport) {
                json.string("transport", transport);
            }
        }
        match self {
            Signal::SendFailed { transport_error } => {
                if let Some(error) = transport_error {
                    json.string("transport_error", error);
                }
            }
            Signal::DeliveryExhausted {
                attempts,
                transport_error,
            } => {
                if let Some(error) = transport_error {
                    json.string("transport_error", error);
                }
                json.number("attempts", *attempts);
            }
            Signal::EnvelopeCorrupt { codec, .. } => json.string("codec", codec),
            Signal::BackpressureDrop { depth } => json.number("queue_depth", *depth),
            Signal::DedupWindowOverflow { window_size, .. } => {
                json.number("window_size", *window_size)
            }
            Signal::OutboundStaleDrop { age_ms, max_age_ms } => {
                json.number("age_ms", *age_ms);
                json.number("max_age_ms", *max_age_ms);
            }
            Signal::OrderingGap {
                lost_lo, lost_hi, ..
            } => {
                json.number("lost_seq_lo", *lost_lo);
                json.number("lost_seq_hi", *lost_hi);
            }
            Signal::MissingSequence { .. } | Signal::TransportUnavailable => {}
        }
        json.finish()
    }
}

/// A flat JSON object written in the order its fields are added.
struct Json(String);

impl Json {
    fn new(reason: &str) -> Self {
        let mut json = Json(String::from("{"));
        json.field("errorName");
        json.0.push_str("\"communication\"");
        json.string("reason", reason);
        json
    }

    fn field(&mut self, key: &str) {
        if self.0.len() > 1 {
            self.0.push(',');
        }
        self.0.push('"');
        self.0.push_str(key);
        self.0.push_str("\":");
    }

    fn string(&mut self, key: &str, value: &str) {
        self.field(key);
        self.0.push('"');
        self.0.push_str(&escape_json_string(value));
        self.0.push('"');
    }

    fn number(&mut self, key: &str, value: impl core::fmt::Display) {
        use core::fmt::Write;
        self.field(key);
        // Writing to a String cannot fail.
        let _ = write!(self.0, "{value}");
    }

    fn finish(mut self) -> String {
        self.0.push('}');
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;

    const WSS_TO_HMI: Option<Binding<'static>> = Some(Binding {
        peer: "hmi",
        transport: Some("wss"),
    });

    // Each expectation is what the C++ core's CommunicationError renders for
    // the same row, fields in that header's declaration order.

    #[test]
    fn a_sending_row_names_its_target_then_its_transport() {
        assert_eq!(
            Signal::TransportUnavailable.event_data(WSS_TO_HMI),
            r#"{"errorName":"communication","reason":"TRANSPORT_UNAVAILABLE","target":"hmi","transport":"wss"}"#
        );
        assert_eq!(
            Signal::OutboundStaleDrop {
                age_ms: 900,
                max_age_ms: 500
            }
            .event_data(WSS_TO_HMI),
            r#"{"errorName":"communication","reason":"OUTBOUND_STALE_DROP","target":"hmi","transport":"wss","age_ms":900,"max_age_ms":500}"#
        );
        assert_eq!(
            Signal::BackpressureDrop { depth: 4 }.event_data(WSS_TO_HMI),
            r#"{"errorName":"communication","reason":"BACKPRESSURE_DROP","target":"hmi","transport":"wss","queue_depth":4}"#
        );
    }

    #[test]
    fn a_transport_error_precedes_the_attempt_count() {
        assert_eq!(
            Signal::DeliveryExhausted {
                attempts: 3,
                transport_error: Some("closed \"early\"".to_string()),
            }
            .event_data(WSS_TO_HMI),
            r#"{"errorName":"communication","reason":"DELIVERY_EXHAUSTED","target":"hmi","transport":"wss","transport_error":"closed \"early\"","attempts":3}"#
        );
        assert_eq!(
            Signal::SendFailed {
                transport_error: None
            }
            .event_data(WSS_TO_HMI),
            r#"{"errorName":"communication","reason":"SEND_FAILED","target":"hmi","transport":"wss"}"#
        );
    }

    #[test]
    fn a_receiving_row_names_its_source_and_no_target() {
        assert_eq!(
            Signal::EnvelopeCorrupt {
                source: None,
                codec: "cbor"
            }
            .event_data(WSS_TO_HMI),
            r#"{"errorName":"communication","reason":"ENVELOPE_CORRUPT","transport":"wss","codec":"cbor"}"#
        );
        assert_eq!(
            Signal::EnvelopeCorrupt {
                source: Some("ecu".to_string()),
                codec: "typed"
            }
            .event_data(WSS_TO_HMI),
            r#"{"errorName":"communication","reason":"ENVELOPE_CORRUPT","source":"ecu","transport":"wss","codec":"typed"}"#
        );
        assert_eq!(
            Signal::MissingSequence {
                source: "ecu".to_string()
            }
            .event_data(WSS_TO_HMI),
            r#"{"errorName":"communication","reason":"MISSING_SEQUENCE","source":"ecu"}"#
        );
        assert_eq!(
            Signal::DedupWindowOverflow {
                source: "ecu".to_string(),
                window_size: 256
            }
            .event_data(WSS_TO_HMI),
            r#"{"errorName":"communication","reason":"DEDUP_WINDOW_OVERFLOW","source":"ecu","window_size":256}"#
        );
    }

    #[test]
    fn a_gap_ended_by_a_tick_has_no_binding_to_name() {
        assert_eq!(
            Signal::OrderingGap {
                source: "ecu".to_string(),
                lost_lo: 4,
                lost_hi: 6
            }
            .event_data(None),
            r#"{"errorName":"communication","reason":"ORDERING_GAP","source":"ecu","lost_seq_lo":4,"lost_seq_hi":6}"#
        );
    }
}
