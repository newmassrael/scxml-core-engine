// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The router core's send half, for one target: readiness-gated buffering
//! and retry (SCE_MESH.md §mesh-10.10), with the semantics of the C++
//! `OutboundBuffer` and `RetryingDispatcher` (sce/include/mesh/).
//!
//! Sans-IO where the C++ classes call a dispatcher: every call here returns
//! what to send and what to signal, and the host does the sending. The C++
//! buffer dispatches under its own lock to keep FIFO against a concurrent
//! drain; a core that never sends has no such race to guard, and runs under
//! a test exactly as under a socket. Every decision — send now or queue,
//! drop on a full queue, drop a stale envelope, retry or give up, how long
//! to wait — is a call into the generated rules (`sce:std/mesh`), and what
//! is done with their verdicts (the order they are asked in, the readiness
//! edge, the count of sends) is the module `gate`'s, a core proved by Verus
//! (docs/adr/0115, D13).

mod gate;

use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

use sce_forge_runtime::algorithm::AlgorithmError;

use crate::generated::outbound_overflows::outbound_overflows;
use crate::generated::outbound_sends_now::outbound_sends_now;
use crate::generated::outbound_stale::outbound_stale;
use crate::generated::retry_exhausted::retry_exhausted;
use crate::generated::retry_jittered::retry_jittered;
use crate::generated::retry_next_backoff::retry_next_backoff;
use crate::signal::Signal;

pub use gate::Attempts;
use gate::{Admission, Gate};

/// What became of an envelope handed to [`Outbound::admit`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Admitted {
    /// Send these bytes now.
    Send(Vec<u8>),
    /// Held until the target is ready.
    Queued,
    /// Dropped: the queue was full (`Signal::BackpressureDrop`).
    Dropped(Signal),
}

/// What a target becoming ready releases: the queued envelopes still worth
/// sending, in the order they were admitted, and a signal for each one that
/// waited too long.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Drained {
    pub send: Vec<Vec<u8>>,
    pub signals: Vec<Signal>,
}

struct Queued {
    bytes: Vec<u8>,
    enqueued_at_ms: i64,
}

/// deploy.yaml's `outbound_buffer:` section (§mesh-10.10): how many envelopes
/// may wait for a target to become ready, and for how long.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutboundBuffer {
    /// `max_pending_per_target`.
    pub max_pending: u32,
    /// `max_age_ms`; 0 means no bound (deploy.yaml omits it).
    pub max_age_ms: i64,
}

/// One target's outbound side: its readiness, and the buffer that holds
/// envelopes until it is ready when the deployment declares one.
///
/// Every admission time in the queue is a moment on the host's monotonic
/// clock no earlier than the Unix epoch: the one the host gave the admit, or
/// one a checkpoint wrote and its restore placed exactly
/// (`Clocks::placed`, which refuses what it cannot place). So an envelope's
/// age at any now at or after its admission is at most the wall clock's count
/// of milliseconds then, and `outbound_stale` judges it at every such now
/// while that count fits an i64.
pub struct Outbound {
    /// `None` when the machine declares no `outbound_buffer:` section: then
    /// §mesh-10.10 dispatches every envelope directly, ready or not, and the
    /// transport reports what it could not send.
    buffer: Option<OutboundBuffer>,
    gate: Gate,
    queue: VecDeque<Queued>,
}

impl Outbound {
    /// A target's outbound side with deploy.yaml's `outbound_buffer:`, or
    /// `None` for direct dispatch. A target starts not ready: nothing has
    /// said it is.
    pub fn new(buffer: Option<OutboundBuffer>) -> Self {
        Self {
            buffer,
            gate: Gate::new(),
            queue: VecDeque::new(),
        }
    }

    /// Admit `bytes`, sent by the host at `now_ms`. Without a buffer it is
    /// sent at once (§mesh-10.10's opt-in gate). With one, it is sent at once
    /// only when the target is ready and nothing waits ahead of it — an
    /// envelope sent past a queue would overtake the ones waiting.
    pub fn admit(&mut self, bytes: Vec<u8>, now_ms: i64) -> Admitted {
        let depth = self.depth();
        let sends_now = outbound_sends_now(self.gate.is_ready(), depth);
        let overflows = self
            .buffer
            .is_some_and(|buffer| outbound_overflows(depth, buffer.max_pending));
        match Gate::admission(self.buffer.is_some(), sends_now, overflows) {
            Admission::Send => Admitted::Send(bytes),
            Admission::Drop => Admitted::Dropped(Signal::BackpressureDrop { depth }),
            Admission::Queue => {
                self.queue.push_back(Queued {
                    bytes,
                    enqueued_at_ms: now_ms,
                });
                Admitted::Queued
            }
        }
    }

    /// The target became ready at `now_ms`: release the queue in order, less
    /// what waited longer than the bound. Idempotent — a ready target's
    /// queue is empty, since admit only queues behind a queue or while not
    /// ready.
    ///
    /// All or nothing: every envelope that waits is judged before any leaves,
    /// so a rule that refuses one, which the admission times this type keeps
    /// leave it no cause to, still leaves the target as it was, not ready,
    /// with every envelope waiting in the order it was admitted, and nothing
    /// to send or tell. Taking each from the queue as it was judged would lose
    /// the ones taken before the refusal with the release the error discards.
    pub fn mark_ready(&mut self, now_ms: i64) -> Result<Drained, AlgorithmError> {
        // Without a buffer nothing was queued, so there is nothing to release.
        let max_age_ms = self.buffer.map_or(0, |b| b.max_age_ms);
        let stale = self
            .queue
            .iter()
            .map(|queued| outbound_stale(queued.enqueued_at_ms, now_ms, max_age_ms))
            .collect::<Result<Vec<bool>, AlgorithmError>>()?;
        self.gate.ready();
        let mut out = Drained::default();
        for (queued, stale) in self.queue.drain(..).zip(stale) {
            if stale {
                // The rule subtracted the same two times without overflow.
                out.signals.push(Signal::OutboundStaleDrop {
                    age_ms: now_ms - queued.enqueued_at_ms,
                    max_age_ms,
                });
                continue;
            }
            out.send.push(queued.bytes);
        }
        Ok(out)
    }

    /// The target stopped being ready. Signals only on the ready-to-not-ready
    /// edge: a target that was never ready had no connection to lose.
    pub fn mark_not_ready(&mut self) -> Option<Signal> {
        self.gate
            .not_ready()
            .then_some(Signal::TransportUnavailable)
    }

    /// Envelopes waiting, as the rules read them: a queue past what a u32
    /// counts (only a checkpoint can put one back that long) reads as full.
    pub fn depth(&self) -> u32 {
        Gate::depth(self.queue.len())
    }

    /// Whether the target is ready: something said it is, and nothing since
    /// said it stopped.
    pub fn is_ready(&self) -> bool {
        self.gate.is_ready()
    }

    /// What waits for the target, oldest first, each with when it was
    /// admitted on the host's monotonic clock: what a checkpoint holds of it.
    pub(crate) fn queued(&self) -> impl Iterator<Item = (&[u8], i64)> + '_ {
        self.queue
            .iter()
            .map(|queued| (queued.bytes.as_slice(), queued.enqueued_at_ms))
    }

    /// Put an envelope a checkpoint held as waiting back behind what waits,
    /// admitted at `enqueued_at_ms`, so its age bound still runs from when it
    /// was first admitted. The moment is one the restore placed exactly on
    /// the host's monotonic clock (`Clocks::placed`), as the type's
    /// admission times are.
    pub(crate) fn requeue(&mut self, bytes: Vec<u8>, enqueued_at_ms: i64) {
        self.queue.push_back(Queued {
            bytes,
            enqueued_at_ms,
        });
    }
}

/// deploy.yaml's `retry` block (§mesh-10.10). A target without one gives up
/// on the first failed send.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RetryPolicy {
    pub max_retries: u32,
    pub initial_backoff_ms: i64,
    pub backoff_multiplier: f64,
    pub max_backoff_ms: i64,
    pub jitter_pct: i64,
}

/// What to do after a send failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AfterFailure {
    /// Send the same envelope again after `wait_ms`.
    RetryIn { wait_ms: i64 },
    /// Stop; the envelope is lost, and this is why.
    GiveUp(Signal),
}

impl Attempts {
    /// The send just made failed. `retryable` is the transport's reading of
    /// its failure; `draw` is a non-negative number the host drew uniformly
    /// for jitter, so two backends given the same draw wait the same time.
    pub fn after_failure(
        &mut self,
        policy: Option<&RetryPolicy>,
        retryable: bool,
        transport_error: Option<String>,
        draw: i64,
    ) -> Result<AfterFailure, AlgorithmError> {
        let counted = self.failed();
        let Some(policy) = policy else {
            return Ok(AfterFailure::GiveUp(Signal::SendFailed { transport_error }));
        };
        // A count that cannot go on is past the last retry of every policy.
        let exhausted = match counted {
            Some(made) => retry_exhausted(made, policy.max_retries, retryable)?,
            None => true,
        };
        if exhausted {
            return Ok(AfterFailure::GiveUp(Signal::DeliveryExhausted {
                attempts: self.made(),
                transport_error,
            }));
        }
        let backoff = match self.last_backoff_ms() {
            None => policy.initial_backoff_ms,
            Some(prev) => {
                retry_next_backoff(prev, policy.backoff_multiplier, policy.max_backoff_ms)?
            }
        };
        self.chose(backoff);
        let wait_ms = retry_jittered(backoff, policy.jitter_pct, draw)?;
        Ok(AfterFailure::RetryIn { wait_ms })
    }
}

/// An envelope's history before its first send: none made, no wait chosen.
impl Default for Attempts {
    fn default() -> Self {
        Attempts::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;
    use alloc::vec;

    fn bytes(tag: u8) -> Vec<u8> {
        vec![tag]
    }

    fn buffered(max_pending: u32, max_age_ms: i64) -> Outbound {
        Outbound::new(Some(OutboundBuffer {
            max_pending,
            max_age_ms,
        }))
    }

    /// §mesh-10.10's opt-in gate: a machine with no `outbound_buffer:`
    /// section dispatches directly, whether or not the target is ready, and
    /// still reports losing a readiness it had.
    #[test]
    fn without_a_buffer_every_envelope_is_sent_at_once() {
        let mut out = Outbound::new(None);
        assert_eq!(out.admit(bytes(1), 0), Admitted::Send(bytes(1)));
        out.mark_ready(0).unwrap();
        assert_eq!(out.mark_not_ready(), Some(Signal::TransportUnavailable));
        assert_eq!(out.admit(bytes(2), 0), Admitted::Send(bytes(2)));
        assert_eq!(out.depth(), 0);
    }

    #[test]
    fn a_ready_target_with_nothing_waiting_sends_at_once() {
        let mut out = buffered(4, 0);
        out.mark_ready(0).unwrap();
        assert_eq!(out.admit(bytes(1), 0), Admitted::Send(bytes(1)));
    }

    #[test]
    fn a_target_not_yet_ready_queues_and_releases_in_order() {
        let mut out = buffered(4, 0);
        assert_eq!(out.admit(bytes(1), 0), Admitted::Queued);
        assert_eq!(out.admit(bytes(2), 0), Admitted::Queued);
        let drained = out.mark_ready(10).unwrap();
        assert_eq!(drained.send, vec![bytes(1), bytes(2)]);
        assert!(drained.signals.is_empty());
        assert_eq!(out.depth(), 0);
    }

    #[test]
    fn a_full_queue_drops_the_newest_and_says_how_deep_it_was() {
        let mut out = buffered(2, 0);
        out.admit(bytes(1), 0);
        out.admit(bytes(2), 0);
        assert_eq!(
            out.admit(bytes(3), 0),
            Admitted::Dropped(Signal::BackpressureDrop { depth: 2 })
        );
        assert_eq!(out.mark_ready(0).unwrap().send, vec![bytes(1), bytes(2)]);
    }

    #[test]
    fn an_envelope_that_waited_past_the_bound_is_dropped_not_sent() {
        let mut out = buffered(4, 50);
        out.admit(bytes(1), 100);
        out.admit(bytes(2), 130);
        let drained = out.mark_ready(151).unwrap();
        assert_eq!(drained.send, vec![bytes(2)]);
        assert_eq!(
            drained.signals,
            vec![Signal::OutboundStaleDrop {
                age_ms: 51,
                max_age_ms: 50
            }]
        );
    }

    /// A release a rule refuses part way loses nothing: an envelope put back
    /// at a moment no restore places, whose age overflows an i64, is refused
    /// after the one ahead of it was judged fit to send, and every envelope
    /// still waits, in order, with its admission time, behind which a new one
    /// queues; the target is not taken as ready. Red if the queue is drained
    /// while the verdicts are asked.
    #[test]
    fn a_release_a_rule_refuses_part_way_loses_and_reorders_nothing() {
        let mut out = buffered(4, 1_000);
        assert_eq!(out.admit(bytes(1), 90), Admitted::Queued);
        out.requeue(bytes(2), i64::MIN);
        out.requeue(bytes(3), 95);
        assert_eq!(out.mark_ready(100), Err(AlgorithmError::Overflow));
        assert!(!out.is_ready(), "a refused release does not take readiness");
        assert_eq!(out.depth(), 3);
        let waiting: Vec<(Vec<u8>, i64)> = out
            .queued()
            .map(|(envelope, since)| (envelope.to_vec(), since))
            .collect();
        assert_eq!(
            waiting,
            vec![(bytes(1), 90), (bytes(2), i64::MIN), (bytes(3), 95)]
        );
        assert_eq!(out.admit(bytes(4), 100), Admitted::Queued);
        assert_eq!(out.depth(), 4);
    }

    #[test]
    fn losing_readiness_signals_once_and_only_after_being_ready() {
        let mut out = buffered(4, 0);
        assert_eq!(out.mark_not_ready(), None);
        out.mark_ready(0).unwrap();
        assert_eq!(out.mark_not_ready(), Some(Signal::TransportUnavailable));
        assert_eq!(out.mark_not_ready(), None);
        assert_eq!(out.admit(bytes(1), 0), Admitted::Queued);
    }

    const POLICY: RetryPolicy = RetryPolicy {
        max_retries: 3,
        initial_backoff_ms: 100,
        backoff_multiplier: 2.0,
        max_backoff_ms: 300,
        jitter_pct: 0,
    };

    #[test]
    fn without_a_retry_policy_a_failed_send_is_lost_at_once() {
        let mut attempts = Attempts::default();
        assert_eq!(
            attempts.after_failure(None, true, Some("eof".to_string()), 0),
            Ok(AfterFailure::GiveUp(Signal::SendFailed {
                transport_error: Some("eof".to_string())
            }))
        );
    }

    #[test]
    fn retries_back_off_to_the_cap_and_then_give_up() {
        let mut attempts = Attempts::default();
        let waits: Vec<_> = (0..3)
            .map(|_| {
                attempts
                    .after_failure(Some(&POLICY), true, None, 0)
                    .unwrap()
            })
            .collect();
        assert_eq!(
            waits,
            vec![
                AfterFailure::RetryIn { wait_ms: 100 },
                AfterFailure::RetryIn { wait_ms: 200 },
                AfterFailure::RetryIn { wait_ms: 300 },
            ]
        );
        assert_eq!(
            attempts.after_failure(Some(&POLICY), true, None, 0),
            Ok(AfterFailure::GiveUp(Signal::DeliveryExhausted {
                attempts: 4,
                transport_error: None
            }))
        );
    }

    /// docs/adr/0115, D13: a count of sends never wraps. As many sends made
    /// as a u32 counts is past the last retry of every policy, the largest
    /// included, and the envelope is given up with that count; without a
    /// policy, as any failed send is. Red if the count is taken one further.
    #[test]
    fn a_count_of_sends_that_cannot_go_on_gives_up() {
        let policy = RetryPolicy {
            max_retries: u32::MAX,
            ..POLICY
        };
        let mut attempts = Attempts::resumed(u32::MAX, Some(300));
        assert_eq!(
            attempts.after_failure(Some(&policy), true, None, 0),
            Ok(AfterFailure::GiveUp(Signal::DeliveryExhausted {
                attempts: u32::MAX,
                transport_error: None
            }))
        );
        assert_eq!(attempts.made(), u32::MAX);
        assert_eq!(attempts.last_backoff_ms(), Some(300), "no wait was chosen");
        let mut alone = Attempts::resumed(u32::MAX, None);
        assert_eq!(
            alone.after_failure(None, true, None, 0),
            Ok(AfterFailure::GiveUp(Signal::SendFailed {
                transport_error: None
            }))
        );
    }

    /// docs/adr/0115, D13: what waits is read by the rules as a u32, and a
    /// queue longer than that reads as full, never as empty: an envelope
    /// admitted behind it waits or is dropped, and never overtakes it. Red
    /// if the length is cut to its low 32 bits.
    #[test]
    fn a_queue_too_long_to_count_reads_as_full() {
        assert_eq!(Gate::depth(0), 0);
        assert_eq!(Gate::depth(3), 3);
        assert_eq!(Gate::depth(u32::MAX as usize), u32::MAX);
        assert_eq!(Gate::depth(usize::MAX), u32::MAX);
    }

    #[test]
    fn a_terminal_failure_gives_up_on_the_first_attempt() {
        let mut attempts = Attempts::default();
        assert_eq!(
            attempts.after_failure(Some(&POLICY), false, None, 0),
            Ok(AfterFailure::GiveUp(Signal::DeliveryExhausted {
                attempts: 1,
                transport_error: None
            }))
        );
    }

    #[test]
    fn jitter_spreads_the_wait_by_the_hosts_draw() {
        let policy = RetryPolicy {
            jitter_pct: 10,
            ..POLICY
        };
        let mut low = Attempts::default();
        let mut high = Attempts::default();
        assert_eq!(
            low.after_failure(Some(&policy), true, None, 0),
            Ok(AfterFailure::RetryIn { wait_ms: 90 })
        );
        assert_eq!(
            high.after_failure(Some(&policy), true, None, 20),
            Ok(AfterFailure::RetryIn { wait_ms: 110 })
        );
    }
}
