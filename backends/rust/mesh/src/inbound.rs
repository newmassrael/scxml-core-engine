// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The router core's receive half: an envelope off the wire, through
//! duplicate suppression (SCE_MESH.md §mesh-10.5) and receiver ordering
//! (§mesh-10.6), to the envelopes the engine may now see, in order.
//!
//! Every decision is a call into the generated standard rules
//! (`sce:std/mesh`): whether an id was seen, where an ahead-of-sequence
//! envelope is held, how far a release drains, when a gap has waited long
//! enough. What this module adds is the state those rules are applied to,
//! kept per sender, and the envelopes themselves while they are held — the
//! rules see only `{seq, arrived_at}`, because an envelope is not a value a
//! list of records can carry.
//!
//! The semantics are those of the C++ `OrderingBuffer` and `DedupRouter`
//! (sce/include/mesh/), so a Rust receiver and a C++ one deliver the same
//! stream the same way.
//!
//! A held envelope is kept as the bytes it arrived as, not as a decoded
//! value: the generated [`Envelope`] borrows from its input, and re-reading
//! the bytes on release costs one decode where an owned copy would be a
//! second shape of the envelope to keep in step with the document.

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use sce_forge_runtime::algorithm::AlgorithmError;
use sce_forge_runtime::codec::{CodecError, SceCursor};
use sce_portable_bytes::SceOwnedList;

use crate::generated::dedup_admit::dedup_admit;
use crate::generated::dedup_holds::dedup_holds;
use crate::generated::envelope::Envelope;
use crate::generated::envelope_id::EnvelopeIdPayload;
use crate::generated::ordering_drain::ordering_drain;
use crate::generated::ordering_gap_end::ordering_gap_end;
use crate::generated::ordering_hold::ordering_hold;
use crate::generated::ordering_prune::ordering_prune;
use crate::generated::ordering_slot::OrderingSlotPayload;
use crate::signal::Signal;

/// What one binding asks of the envelopes it receives. Deployment decides
/// both, per binding: dedup only where the transport does not already
/// suppress duplicates (§mesh-10.5), ordering only where the binding
/// declares it and the transport does not supply it (§mesh-10.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Delivery {
    /// Drop an envelope whose id this sender's window already holds.
    pub dedup: bool,
    /// Release envelopes in their sender's `sequence_no` order.
    pub ordered: bool,
}

/// An envelope the engine may now see.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Received {
    bytes: Vec<u8>,
}

impl Received {
    /// The envelope, read from the bytes it arrived as.
    pub fn envelope(&self) -> Envelope<'_> {
        // These bytes decoded once already, when they were admitted, and
        // decoding is a function of the bytes alone.
        Envelope::decode(&mut SceCursor::new(&self.bytes))
            .expect("admitted envelope bytes decode as they did on admission")
    }

    /// The bytes as they arrived.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// What one call released, in the order the engine must see it, and what
/// it signalled.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Outcome {
    pub released: Vec<Received>,
    pub signals: Vec<Signal>,
}

/// Why an envelope was not admitted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdmitError {
    /// The bytes are not an envelope the standard document accepts.
    Malformed(CodecError),
    /// The binding is ordered and the envelope carries no `sequence_no`
    /// (§mesh-10.6.3). Refused before the dedup window sees its id, so a
    /// refused envelope does not take a window slot.
    Unstamped,
    /// A rule refused its input: a sequence at the `u64` maximum, or a clock
    /// reading that cannot be subtracted.
    Rule(AlgorithmError),
    /// A sender has more envelopes held ahead of sequence than the rule's
    /// list can carry.
    HoldFull,
}

/// Why a receive core could not be built.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigError {
    /// deploy.yaml's `window_size` is 0; §mesh-10.5 needs at least one id.
    EmptyDedupWindow,
    /// deploy.yaml's `gap_timeout_ms` is negative.
    NegativeGapTimeout,
}

/// One sender's receive state.
struct Sender {
    window: SceOwnedList<EnvelopeIdPayload, 256>,
    /// The sequence this sender is expected to send next; `None` until its
    /// first ordered envelope, which anchors it (a receiver that joins
    /// mid-stream did not miss what was sent before it listened).
    next: Option<u64>,
    pending: SceOwnedList<OrderingSlotPayload, 256>,
    held: BTreeMap<u64, Received>,
}

impl Sender {
    fn new() -> Self {
        Self {
            window: SceOwnedList::new(),
            next: None,
            pending: SceOwnedList::new(),
            held: BTreeMap::new(),
        }
    }

    /// Release everything held that follows on from `start` without a gap,
    /// and expect what comes after it.
    fn release_from(&mut self, start: u64, out: &mut Outcome) -> Result<(), AdmitError> {
        let drained = ordering_drain(self.pending.as_slice(), start).map_err(AdmitError::Rule)?;
        let still_held = self.held.split_off(&drained);
        out.released
            .extend(core::mem::replace(&mut self.held, still_held).into_values());
        self.pending =
            ordering_prune(self.pending.as_slice(), drained).map_err(|_| AdmitError::HoldFull)?;
        self.next = Some(drained);
        Ok(())
    }
}

/// The receive half of a router: every sender's window and held envelopes.
pub struct Inbound {
    dedup_window: u32,
    gap_timeout_ms: i64,
    senders: BTreeMap<String, Sender>,
}

impl Inbound {
    /// A core with deploy.yaml's `window_size` (§mesh-10.5) and
    /// `gap_timeout_ms` (§mesh-10.6.1).
    pub fn new(dedup_window: u32, gap_timeout_ms: i64) -> Result<Self, ConfigError> {
        if dedup_window == 0 {
            return Err(ConfigError::EmptyDedupWindow);
        }
        if gap_timeout_ms < 0 {
            return Err(ConfigError::NegativeGapTimeout);
        }
        Ok(Self {
            dedup_window,
            gap_timeout_ms,
            senders: BTreeMap::new(),
        })
    }

    /// Admit the envelope `bytes` from a binding asking `delivery`, at
    /// `now_ms` on the host's monotonic clock.
    ///
    /// SCE_MESH.md §mesh-10.6.4: an ordered envelope below its sender's
    /// expected sequence is dropped, one above it is held, and the one it
    /// expects is released together with every held envelope that now
    /// follows on.
    pub fn admit(
        &mut self,
        bytes: Vec<u8>,
        delivery: Delivery,
        now_ms: i64,
    ) -> Result<Outcome, AdmitError> {
        let (source, id, sequence) = {
            let envelope =
                Envelope::decode(&mut SceCursor::new(&bytes)).map_err(AdmitError::Malformed)?;
            (
                envelope.source.to_string(),
                envelope_id(envelope.id),
                envelope.sequence_no,
            )
        };
        if delivery.ordered && sequence.is_none() {
            return Err(AdmitError::Unstamped);
        }

        let mut out = Outcome::default();
        let sender = self
            .senders
            .entry(source.clone())
            .or_insert_with(Sender::new);

        if delivery.dedup {
            if dedup_holds(sender.window.as_slice(), id) {
                return Ok(out);
            }
            let full = sender.window.as_slice().len() >= self.dedup_window as usize;
            sender.window = dedup_admit(sender.window.as_slice(), id, self.dedup_window)
                .map_err(AdmitError::Rule)?;
            if full {
                out.signals.push(Signal::DedupWindowOverflow {
                    source: source.clone(),
                });
            }
        }

        let Some(seq) = sequence.filter(|_| delivery.ordered) else {
            out.released.push(Received { bytes });
            return Ok(out);
        };
        let next = *sender.next.get_or_insert(seq);
        if seq < next {
            return Ok(out);
        }
        if seq > next {
            sender.pending = ordering_hold(sender.pending.as_slice(), seq, now_ms)
                .map_err(|_| AdmitError::HoldFull)?;
            // A re-delivery of a held sequence keeps its first copy, as
            // ordering_hold keeps its first arrival time.
            sender.held.entry(seq).or_insert(Received { bytes });
            return Ok(out);
        }
        out.released.push(Received { bytes });
        let start = seq
            .checked_add(1)
            .ok_or(AdmitError::Rule(AlgorithmError::Overflow))?;
        sender.release_from(start, &mut out)?;
        Ok(out)
    }

    /// End every gap that has waited `gap_timeout_ms` by `now_ms`: signal the
    /// sequences lost and release what was held behind them. The host calls
    /// this on its own schedule, so a gap ends even when nothing else
    /// arrives from that sender.
    pub fn tick(&mut self, now_ms: i64) -> Result<Outcome, AdmitError> {
        let mut out = Outcome::default();
        for (source, sender) in &mut self.senders {
            let Some(next) = sender.next else {
                continue;
            };
            let end =
                ordering_gap_end(sender.pending.as_slice(), next, now_ms, self.gap_timeout_ms)
                    .map_err(AdmitError::Rule)?;
            if end == next {
                continue;
            }
            out.signals.push(Signal::OrderingGap {
                source: source.clone(),
                lost_lo: next,
                lost_hi: end - 1,
            });
            sender.release_from(end, &mut out)?;
        }
        Ok(out)
    }
}

/// An envelope id as the rules hold it: its first eight bytes and its last
/// eight, big-endian (`sce:std/mesh/envelope_id`). The envelope document
/// fixes `id` at exactly 16 bytes, so a decoded id always splits.
fn envelope_id(id: &[u8]) -> EnvelopeIdPayload {
    let (hi, lo) = id.split_at(8);
    EnvelopeIdPayload {
        hi: u64::from_be_bytes(hi.try_into().expect("a decoded envelope id is 16 bytes")),
        lo: u64::from_be_bytes(lo.try_into().expect("a decoded envelope id is 16 bytes")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generated::pattern_kind::PatternKind;
    use crate::generated::payload_codec::PayloadCodec;

    const PLAIN: Delivery = Delivery {
        dedup: false,
        ordered: false,
    };
    const DEDUP: Delivery = Delivery {
        dedup: true,
        ordered: false,
    };
    const ORDERED: Delivery = Delivery {
        dedup: false,
        ordered: true,
    };

    fn wire(source: &str, id: u8, sequence_no: Option<u64>) -> Vec<u8> {
        let id = [id; 16];
        Envelope {
            id: &id,
            source,
            event_type: "evt",
            pattern: PatternKind::FireForget,
            datacontenttype: PayloadCodec::None,
            data: &[],
            sequence_no,
            ..Envelope::new()
        }
        .encode_to_vec()
        .expect("a well-formed envelope encodes")
    }

    fn sequences(out: &Outcome) -> Vec<u64> {
        out.released
            .iter()
            .map(|r| {
                r.envelope()
                    .sequence_no
                    .expect("ordered envelopes carry one")
            })
            .collect()
    }

    fn core() -> Inbound {
        Inbound::new(2, 50).expect("a valid configuration")
    }

    #[test]
    fn an_unordered_envelope_is_released_as_it_arrives() {
        let mut inbound = core();
        let bytes = wire("ecu", 1, None);
        let out = inbound.admit(bytes.clone(), PLAIN, 0).unwrap();
        assert_eq!(out.released.len(), 1);
        assert_eq!(out.released[0].bytes(), bytes.as_slice());
        assert_eq!(out.released[0].envelope().source, "ecu");
    }

    #[test]
    fn a_duplicate_id_from_the_same_sender_is_dropped_and_another_senders_is_not() {
        let mut inbound = core();
        assert_eq!(
            inbound
                .admit(wire("a", 1, None), DEDUP, 0)
                .unwrap()
                .released
                .len(),
            1
        );
        assert!(inbound
            .admit(wire("a", 1, None), DEDUP, 0)
            .unwrap()
            .released
            .is_empty());
        assert_eq!(
            inbound
                .admit(wire("b", 1, None), DEDUP, 0)
                .unwrap()
                .released
                .len(),
            1
        );
    }

    #[test]
    fn a_novel_id_admitted_to_a_full_window_signals_the_overflow() {
        let mut inbound = core();
        for id in [1, 2] {
            assert!(inbound
                .admit(wire("a", id, None), DEDUP, 0)
                .unwrap()
                .signals
                .is_empty());
        }
        let out = inbound.admit(wire("a", 3, None), DEDUP, 0).unwrap();
        assert_eq!(
            out.signals,
            vec![Signal::DedupWindowOverflow {
                source: "a".to_string()
            }]
        );
        // The oldest id left the window, so it is novel again.
        assert_eq!(
            inbound
                .admit(wire("a", 1, None), DEDUP, 0)
                .unwrap()
                .released
                .len(),
            1
        );
    }

    #[test]
    fn the_first_ordered_envelope_anchors_its_sender() {
        let mut inbound = core();
        let out = inbound.admit(wire("a", 7, Some(7)), ORDERED, 0).unwrap();
        assert_eq!(sequences(&out), vec![7]);
    }

    #[test]
    fn an_envelope_ahead_of_sequence_waits_for_the_gap_to_fill() {
        let mut inbound = core();
        assert_eq!(
            sequences(&inbound.admit(wire("a", 1, Some(1)), ORDERED, 0).unwrap()),
            vec![1]
        );
        assert!(inbound
            .admit(wire("a", 4, Some(4)), ORDERED, 0)
            .unwrap()
            .released
            .is_empty());
        assert!(inbound
            .admit(wire("a", 3, Some(3)), ORDERED, 0)
            .unwrap()
            .released
            .is_empty());
        let out = inbound.admit(wire("a", 2, Some(2)), ORDERED, 0).unwrap();
        assert_eq!(sequences(&out), vec![2, 3, 4]);
    }

    #[test]
    fn an_envelope_below_the_expected_sequence_is_dropped() {
        let mut inbound = core();
        inbound.admit(wire("a", 5, Some(5)), ORDERED, 0).unwrap();
        assert!(inbound
            .admit(wire("a", 4, Some(4)), ORDERED, 0)
            .unwrap()
            .released
            .is_empty());
        assert!(inbound
            .admit(wire("a", 5, Some(5)), ORDERED, 0)
            .unwrap()
            .released
            .is_empty());
    }

    #[test]
    fn a_gap_that_waits_out_its_timeout_is_signalled_and_released_past() {
        let mut inbound = core();
        inbound.admit(wire("a", 1, Some(1)), ORDERED, 0).unwrap();
        inbound.admit(wire("a", 4, Some(4)), ORDERED, 100).unwrap();
        inbound.admit(wire("a", 5, Some(5)), ORDERED, 120).unwrap();
        assert_eq!(inbound.tick(149).unwrap(), Outcome::default());
        let out = inbound.tick(150).unwrap();
        assert_eq!(
            out.signals,
            vec![Signal::OrderingGap {
                source: "a".to_string(),
                lost_lo: 2,
                lost_hi: 3
            }]
        );
        assert_eq!(sequences(&out), vec![4, 5]);
        // The sender now expects 6, and a late 3 is a straggler.
        assert!(inbound
            .admit(wire("a", 3, Some(3)), ORDERED, 200)
            .unwrap()
            .released
            .is_empty());
        assert_eq!(
            sequences(&inbound.admit(wire("a", 6, Some(6)), ORDERED, 200).unwrap()),
            vec![6]
        );
    }

    #[test]
    fn an_ordered_binding_refuses_an_unstamped_envelope_before_dedup_sees_it() {
        let mut inbound = core();
        let both = Delivery {
            dedup: true,
            ordered: true,
        };
        assert_eq!(
            inbound.admit(wire("a", 1, None), both, 0),
            Err(AdmitError::Unstamped)
        );
        // Its id took no window slot: the same id stamped is novel.
        assert_eq!(
            sequences(&inbound.admit(wire("a", 1, Some(1)), both, 0).unwrap()),
            vec![1]
        );
    }

    #[test]
    fn bytes_that_are_not_an_envelope_are_refused() {
        let mut inbound = core();
        assert!(matches!(
            inbound.admit(vec![0xFF], PLAIN, 0),
            Err(AdmitError::Malformed(_))
        ));
    }

    #[test]
    fn a_configuration_the_rules_cannot_run_is_refused() {
        assert!(matches!(
            Inbound::new(0, 50),
            Err(ConfigError::EmptyDedupWindow)
        ));
        assert!(matches!(
            Inbound::new(1, -1),
            Err(ConfigError::NegativeGapTimeout)
        ));
    }
}
