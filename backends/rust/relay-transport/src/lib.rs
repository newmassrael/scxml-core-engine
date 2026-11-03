// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What carries the protocol of a shared list's relay between a device and the relay (ADR 0079
//! section 5).
//!
//! The relay's protocol (`sce-rust-relay`, `docs/SCE_SHARED_LIST_RELAY.md` section 3) is texts: a
//! request text, the response text to it, and what the relay pushes. A [`Transport`] carries them
//! over one medium and says what became of its connection; it knows nothing of what the texts mean.
//! What they mean is the peer core's (`sce-rust-peer`), which does no I/O: a driver
//! (`sce_rust_relay::drive`) takes each request the core gives, carries it over a transport and
//! gives the core the response or the transport's failure, until the core's exchange ends, its
//! deadline passes or it is cancelled. One driver serves every transport.
//!
//! A binding implements [`Transport`] and nothing else: the relay's WebSocket binding
//! (`sce_rust_relay::wss::WssLink`, the reference), a transport a host carries itself through the
//! peer core's C ABI, and the deployment transport's adapter, which lives in that transport's own
//! repository and depends on this crate alone of this repository's relay. The contract is the
//! documentation of [`Transport`] and of what it is given ([`Signal`]) and gives back
//! ([`TransportError`], [`Notice`]).
//!
//! A transport does the framing, the authentication of the device (the token its login earned,
//! `sce_rust_relay::token`, carried as the medium carries credentials) and the reconnection, so that
//! none of the three is written again in a host's language: a host gives only the network's
//! reachability ([`Transport::network`]) and the permission to run (a deadline, a cancellation).

#![forbid(unsafe_code)]

use std::collections::VecDeque;
use std::fmt;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Instant;

/// A medium that carries the relay protocol's texts between one device and one relay.
///
/// **One exchange at a time, in order.** [`Self::exchange`] is called by one thread at a time, and
/// the next exchange begins only after the last returned. The response it returns is the response
/// to the request it was given: a medium that could deliver a late response to an earlier request
/// drops what would carry it (the connection) rather than give it to a later one.
///
/// **What it may block on, and for how long.** An exchange may block while it connects (dialling,
/// the medium's handshake, the device's authentication), while it sends, and while it waits for the
/// response. It never blocks past `deadline`, and it returns promptly once `cancel` is raised,
/// whichever thread raises it: it watches the signal ([`Signal::watch`]) for the length of the
/// exchange, or reads it at least as often as it can be stopped.
///
/// **Reconnection is the transport's.** A transport connects when an exchange needs a connection
/// and has none, and after a failure, a deadline, a cancellation or a change of the network the
/// next exchange connects again. It does not send a request twice: when the connection breaks in
/// the middle of an exchange, the exchange fails, and whether to ask again is the core's, which
/// knows what the request means (the relay's requests are made to be asked again, and the core's
/// next round asks what it still needs).
///
/// **What an error means.** An `Ok` is a text the relay sent: its refusal of a request is a response
/// (`{"refused": ...}`), never an error of the transport. An error says the relay's answer did not
/// come back, and why ([`TransportError`]); the core then ends its exchange as a failure of the link,
/// and what the exchange took in is handed over by the next round. Two errors are the relay's
/// endpoint refusing before it carried the request, which the core reads as the relay's own
/// refusals: the device's token ([`TransportError::Unauthorized`]) and the rate
/// ([`TransportError::RateLimited`], which the driver waits out as it waits out a `rate_limited`
/// response).
///
/// **Moved, not shared.** A transport is moved to the thread that runs the rounds (`Send`); what
/// another thread may do to it while an exchange blocks is raise the two signals: the exchange's
/// cancellation, and the network's ([`Self::network`]).
pub trait Transport: Send {
    /// Carry `request` (one request text of the relay protocol) to the relay and return its response
    /// text, by `deadline` when there is one, and giving up once `cancel` is raised. The transport
    /// reads neither text: a response that is not one of the protocol is the core's to refuse.
    fn exchange(
        &mut self,
        request: &str,
        deadline: Option<Instant>,
        cancel: &Signal,
    ) -> Result<String, TransportError>;

    /// What became of the connection and what the relay pushed since this was last asked, oldest
    /// first. It never blocks. A transport keeps at most [`NOTICES_KEPT`] of them and drops the
    /// oldest past that ([`Notices`]).
    fn take_notices(&mut self) -> Vec<Notice>;

    /// The signal the host raises when the device's network changed: an interface came or went, or
    /// the operating system said the path to the network is another. Any thread raises it, at any
    /// time. A transport that holds a connection gives it up and connects again on its next
    /// exchange, and an exchange blocked when it is raised fails ([`TransportError::Link`]) rather
    /// than wait on a path that may be gone. A transport whose medium follows the network itself
    /// may give a signal it does not watch.
    fn network(&self) -> Signal;

    /// Present `token` from the next connection on, the empty text being no token at all (a device
    /// that logs in, ADR 0080): a connection open now that was made with another is given up, and
    /// the next exchange connects again. Whether the transport carries the relay's token; one whose
    /// credentials its host carries itself (a host's own transport) carries none, says `false` and
    /// keeps nothing, which is what a transport that does not override this does.
    fn set_token(&mut self, token: &str) -> bool {
        let _ = token;
        false
    }
}

/// Why an exchange did not bring the relay's response back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportError {
    /// The medium did not carry the request or the response: the relay was not reached, the
    /// connection closed, the network changed. The request may or may not have reached the relay,
    /// and the core assumes neither.
    Link(String),
    /// The relay's endpoint refused the device's authentication before it carried the request: a
    /// token the relay did not issue, or one that expired. The host asks its application for a new
    /// token and makes a transport with it. A relay that refuses a token on a connection that stays
    /// open answers with its refusal instead, which is a response.
    Unauthorized(String),
    /// The relay's endpoint refused the connection for the rate before it carried the request
    /// (the WebSocket binding's upgrade answered 429 with `Retry-After`), and says how long until it
    /// may be asked again, in milliseconds, and why. Nothing of the request was done, so asking the
    /// same request again once the wait is over is safe; the core reads it as the relay's refusal
    /// for the rate (`rate_limited`), the answer a relay gives on a connection it took, so that the
    /// driver waits out both alike.
    RateLimited { retry_after_ms: u64, reason: String },
    /// The deadline passed before the response came.
    Deadline,
    /// The exchange's cancellation was raised before the response came.
    Cancelled,
}

impl fmt::Display for TransportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransportError::Link(reason) => f.write_str(reason),
            TransportError::Unauthorized(reason) => {
                write!(f, "the relay refused the device's token: {reason}")
            }
            TransportError::RateLimited {
                retry_after_ms,
                reason,
            } => write!(
                f,
                "the relay refused the connection for the rate, and may be asked again in \
                 {retry_after_ms} ms: {reason}"
            ),
            TransportError::Deadline => {
                f.write_str("the deadline passed before the relay answered")
            }
            TransportError::Cancelled => f.write_str("the exchange was cancelled"),
        }
    }
}

impl std::error::Error for TransportError {}

/// What a transport tells its host outside an exchange ([`Transport::take_notices`]). The core is
/// told none of it: it reads what the relay holds by its own cursor when a round asks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Notice {
    /// A connection to the relay was made.
    Connected,
    /// The connection was given up, and why: it failed, the relay closed it, the network changed,
    /// or an exchange on it was cancelled or ran out of time.
    Lost(String),
    /// A text the relay pushed, as it came. For a host it is a reason to begin a round, never an
    /// input of the core. Version 1 of the relay protocol pushes nothing, so a binding of it, the
    /// WebSocket binding among them, gives none; a medium that delivers a relay's pushes (a
    /// subscription) gives each here.
    Pushed(String),
}

/// How many notices a transport keeps for its host before it drops the oldest.
pub const NOTICES_KEPT: usize = 64;

/// The notices a transport keeps for its host: at most [`NOTICES_KEPT`], the oldest dropped first,
/// so that a host that never asks costs the transport no more than that.
#[derive(Debug, Default)]
pub struct Notices {
    kept: VecDeque<Notice>,
}

impl Notices {
    /// Keep `notice`, dropping the oldest when [`NOTICES_KEPT`] are kept already.
    pub fn push(&mut self, notice: Notice) {
        if self.kept.len() == NOTICES_KEPT {
            self.kept.pop_front();
        }
        self.kept.push_back(notice);
    }

    /// Every notice kept, oldest first, and none kept after.
    pub fn take(&mut self) -> Vec<Notice> {
        self.kept.drain(..).collect()
    }
}

/// What a watcher of a [`Signal`] is woken with.
type Wake = Arc<dyn Fn() + Send + Sync>;

#[derive(Default)]
struct Shared {
    state: Mutex<State>,
}

#[derive(Default)]
struct State {
    raised: u64,
    next: u64,
    watchers: Vec<(u64, Wake)>,
}

impl Shared {
    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// A notice another thread gives a running exchange: that it is cancelled, or that the network
/// changed. Any thread raises it, through any clone, at any time; a transport reads how many times it
/// was raised ([`Self::raised`]) or is woken when it is ([`Self::watch`]).
///
/// A cancellation is raised once and stays raised: a host makes a new one for each run. The
/// network's is raised at each change, and a transport compares the count with the one its
/// connection was made at.
#[derive(Clone, Default)]
pub struct Signal {
    shared: Arc<Shared>,
}

impl fmt::Debug for Signal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Signal")
            .field("raised", &self.raised())
            .finish()
    }
}

impl Signal {
    /// A signal not raised yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// Raise it: the count goes up by one, and every watcher is woken, on this thread, after the
    /// count moved.
    pub fn raise(&self) {
        // The watchers are woken with the lock let go, so that one may drop its watch.
        let watchers: Vec<Wake> = {
            let mut state = self.shared.state();
            state.raised += 1;
            state
                .watchers
                .iter()
                .map(|(_, wake)| Arc::clone(wake))
                .collect()
        };
        for wake in watchers {
            wake();
        }
    }

    /// How many times it was raised.
    pub fn raised(&self) -> u64 {
        self.shared.state().raised
    }

    /// Whether it was raised at all.
    pub fn is_raised(&self) -> bool {
        self.raised() > 0
    }

    /// Call `wake` each time the signal is raised from now on, until the [`Watch`] is dropped. A
    /// raise before the watch was made does not wake it, so a transport reads [`Self::raised`] after
    /// it begins to watch. `wake` runs on the raising thread and must not block.
    pub fn watch(&self, wake: impl Fn() + Send + Sync + 'static) -> Watch {
        let mut state = self.shared.state();
        let id = state.next;
        state.next += 1;
        state.watchers.push((id, Arc::new(wake)));
        Watch {
            shared: Arc::clone(&self.shared),
            id,
        }
    }
}

/// A watcher of a [`Signal`]: woken while it lives, and taken off when it is dropped.
#[must_use = "a watch dropped at once wakes no one"]
pub struct Watch {
    shared: Arc<Shared>,
    id: u64,
}

impl Drop for Watch {
    fn drop(&mut self) {
        let id = self.id;
        self.shared
            .state()
            .watchers
            .retain(|(watcher, _)| *watcher != id);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use super::{Notice, Notices, Signal, NOTICES_KEPT};

    #[test]
    fn a_signal_counts_its_raises_and_wakes_its_watchers_while_they_live() {
        let signal = Signal::new();
        assert!(!signal.is_raised());
        let woken = Arc::new(AtomicUsize::new(0));
        let watch = {
            let woken = Arc::clone(&woken);
            signal.watch(move || {
                woken.fetch_add(1, Ordering::SeqCst);
            })
        };
        let clone = signal.clone();
        let raiser = std::thread::spawn(move || clone.raise());
        raiser.join().unwrap();
        assert_eq!(signal.raised(), 1);
        assert!(signal.is_raised());
        assert_eq!(
            woken.load(Ordering::SeqCst),
            1,
            "woken by a raise on another thread"
        );
        signal.raise();
        assert_eq!(woken.load(Ordering::SeqCst), 2);
        drop(watch);
        signal.raise();
        assert_eq!(signal.raised(), 3);
        assert_eq!(
            woken.load(Ordering::SeqCst),
            2,
            "a dropped watch is woken no more"
        );
    }

    #[test]
    fn a_watch_made_after_a_raise_is_not_woken_by_it() {
        let signal = Signal::new();
        signal.raise();
        let woken = Arc::new(AtomicUsize::new(0));
        let _watch = {
            let woken = Arc::clone(&woken);
            signal.watch(move || {
                woken.fetch_add(1, Ordering::SeqCst);
            })
        };
        assert_eq!(woken.load(Ordering::SeqCst), 0);
        assert!(
            signal.is_raised(),
            "which is why a transport reads the count after it watches"
        );
    }

    #[test]
    fn notices_keep_the_newest_past_their_bound_and_are_taken_once() {
        let mut notices = Notices::default();
        for n in 0..NOTICES_KEPT + 2 {
            notices.push(Notice::Pushed(n.to_string()));
        }
        let taken = notices.take();
        assert_eq!(taken.len(), NOTICES_KEPT);
        assert_eq!(taken[0], Notice::Pushed("2".to_string()));
        assert_eq!(
            taken[NOTICES_KEPT - 1],
            Notice::Pushed((NOTICES_KEPT + 1).to_string())
        );
        assert!(notices.take().is_empty());
    }
}
