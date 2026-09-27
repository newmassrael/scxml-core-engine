// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
//! to wait — is a call into the generated rules (`sce:std/mesh`).

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

/// One target's outbound buffer.
pub struct Outbound {
    max_pending: u32,
    /// 0 means no bound (deploy.yaml omits `max_age_ms`).
    max_age_ms: i64,
    ready: bool,
    queue: VecDeque<Queued>,
}

impl Outbound {
    /// A buffer with deploy.yaml's `max_pending_per_target` and `max_age_ms`
    /// (0 for none). A target starts not ready: nothing has said it is.
    pub fn new(max_pending: u32, max_age_ms: i64) -> Self {
        Self {
            max_pending,
            max_age_ms,
            ready: false,
            queue: VecDeque::new(),
        }
    }

    /// Admit `bytes`, sent by the host at `now_ms`: sent at once only when
    /// the target is ready and nothing waits ahead of it — an envelope sent
    /// past a queue would overtake the ones waiting.
    pub fn admit(&mut self, bytes: Vec<u8>, now_ms: i64) -> Admitted {
        let depth = self.depth();
        if outbound_sends_now(self.ready, depth) {
            return Admitted::Send(bytes);
        }
        if outbound_overflows(depth, self.max_pending) {
            return Admitted::Dropped(Signal::BackpressureDrop { depth });
        }
        self.queue.push_back(Queued {
            bytes,
            enqueued_at_ms: now_ms,
        });
        Admitted::Queued
    }

    /// The target became ready at `now_ms`: release the queue in order, less
    /// what waited longer than the bound. Idempotent — a ready target's
    /// queue is empty, since admit only queues behind a queue or while not
    /// ready.
    pub fn mark_ready(&mut self, now_ms: i64) -> Result<Drained, AlgorithmError> {
        self.ready = true;
        let mut out = Drained::default();
        while let Some(queued) = self.queue.pop_front() {
            if outbound_stale(queued.enqueued_at_ms, now_ms, self.max_age_ms)? {
                out.signals.push(Signal::OutboundStaleDrop {
                    age_ms: now_ms - queued.enqueued_at_ms,
                    max_age_ms: self.max_age_ms,
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
        let was_ready = core::mem::replace(&mut self.ready, false);
        was_ready.then_some(Signal::TransportUnavailable)
    }

    /// Envelopes waiting.
    pub fn depth(&self) -> u32 {
        // The queue never exceeds `max_pending`, a u32.
        self.queue.len() as u32
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

/// One envelope's retry history: the sends made and the wait last chosen
/// before jitter. Starts empty at the first send.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Attempts {
    made: u32,
    last_backoff_ms: Option<i64>,
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
        self.made += 1;
        let Some(policy) = policy else {
            return Ok(AfterFailure::GiveUp(Signal::SendFailed { transport_error }));
        };
        if retry_exhausted(self.made, policy.max_retries, retryable)? {
            return Ok(AfterFailure::GiveUp(Signal::DeliveryExhausted {
                attempts: self.made,
                transport_error,
            }));
        }
        let backoff = match self.last_backoff_ms {
            None => policy.initial_backoff_ms,
            Some(prev) => {
                retry_next_backoff(prev, policy.backoff_multiplier, policy.max_backoff_ms)?
            }
        };
        self.last_backoff_ms = Some(backoff);
        let wait_ms = retry_jittered(backoff, policy.jitter_pct, draw)?;
        Ok(AfterFailure::RetryIn { wait_ms })
    }

    /// Sends made so far.
    pub fn made(&self) -> u32 {
        self.made
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

    #[test]
    fn a_ready_target_with_nothing_waiting_sends_at_once() {
        let mut out = Outbound::new(4, 0);
        out.mark_ready(0).unwrap();
        assert_eq!(out.admit(bytes(1), 0), Admitted::Send(bytes(1)));
    }

    #[test]
    fn a_target_not_yet_ready_queues_and_releases_in_order() {
        let mut out = Outbound::new(4, 0);
        assert_eq!(out.admit(bytes(1), 0), Admitted::Queued);
        assert_eq!(out.admit(bytes(2), 0), Admitted::Queued);
        let drained = out.mark_ready(10).unwrap();
        assert_eq!(drained.send, vec![bytes(1), bytes(2)]);
        assert!(drained.signals.is_empty());
        assert_eq!(out.depth(), 0);
    }

    #[test]
    fn a_full_queue_drops_the_newest_and_says_how_deep_it_was() {
        let mut out = Outbound::new(2, 0);
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
        let mut out = Outbound::new(4, 50);
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

    #[test]
    fn losing_readiness_signals_once_and_only_after_being_ready() {
        let mut out = Outbound::new(4, 0);
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
