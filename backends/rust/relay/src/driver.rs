// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The one loop that carries an exchange with a relay over a link: the request the exchange wants
//! put, carried, and the answer or the link's failure given back, until the exchange asks nothing
//! more, its deadline passes or it is cancelled.
//!
//! An exchange ([`Exchange`]) does no I/O: a courier's round ([`RoundMachine`]) and the peer core
//! (`sce-rust-peer`'s `Peer`, which composes rounds, the group's device and what the relay is owed)
//! are the two. A link ([`Link`]) carries; every [`Transport`](sce_rust_relay_transport::Transport)
//! is one. [`drive`] joins them, written once, so that a [`Courier`](crate::Courier) over its link,
//! a Rust host's peer core over a transport, and a host in another language that runs its core over
//! a transport through the C ABI go through the same loop, and a deadline and a cancellation mean
//! the same thing in each. A host that carries the texts itself (its operating system's own network
//! stack, a test) may call the exchange by hand instead; the exchange is the same.
//!
//! **A relay's `rate_limited` is flow control, not an outcome.** A relay that refuses a request for
//! the rate says how long until it may be asked again (`retry_after_ms`, the delay of RFC 9110's
//! `Retry-After`), and its WebSocket endpoint refusing a connection for the rate (429) is read as the
//! same refusal ([`crate::Link`]). The driver waits that long and puts the same request again,
//! for as long as the wait ends before the deadline, so that a host's round is not ended for a wait
//! of milliseconds that the host would have to schedule a new round for. The wait is the clock's
//! ([`Clock`]): the system's ([`SystemClock`], which [`drive`] runs on), or the clock of whoever
//! carries the texts, a host whose time is not the system's (a simulation) among them
//! ([`drive_with`]).
//!
//! The same request may be put again because a relay answers `rate_limited` before it does
//! anything of the request, on every path:
//!
//! - a session's request: the device's allowance is the first thing a request is held to after a
//!   relay that takes nothing at all, before its role, its form or what it carries are read
//!   (`Relay::answer_session`, `src/relay.rs`, the `allowances.take` before the role);
//! - a connection with no token: its source's connections a minute are spent at the upgrade, before
//!   any request (`Doorway::open`, `src/login.rs`, answered 429 by `src/serve.rs`'s `upgrade`);
//!   challenges, logins, joins and creations a minute, before the challenge is drawn or the login,
//!   the join or the creation is read (`Doorway::ask`, `src/login.rs`);
//! - a login: the used challenges the relay keeps and the node's logins of the hour are asked before
//!   the list's side appends the entries the login brought (`Doorway::admissible`, `src/login.rs`,
//!   called first by `Relay::anonymous` in `src/relay.rs` and `Shards::handle_anonymous` in
//!   `src/hub.rs`), and asked again, and spent, only once the signature held (`Doorway::redeem`);
//! - a join and a creation: the used challenges the relay keeps, and the relay's creations of the
//!   hour, before the nonce is used and before the list's side judges or keeps anything
//!   (`Doorway::spend_join`, `Doorway::spend_create`, `src/login.rs`).
//!
//! An allowance refused spends nothing of it (`limits::spend` leaves a bucket that cannot pay as it
//! was, refilled to the time asked). So a request asked again after its wait is a request asked for
//! the first time.

use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::time::{Duration, Instant};

use sce_rust_relay_transport::{Signal, TransportError};

use crate::client::{Link, LinkError, RoundMachine, Step};
use crate::protocol::{Code, Request, Response};

/// An exchange with a relay that does no I/O: it says which request it wants put to the relay next
/// and is given the answer, or why the link did not carry it.
///
/// An exchange ends at a failure of its link, or asks only finitely many requests after one: the
/// driver fails every request it is given once the exchange is cancelled or past its deadline, and
/// returns once the exchange asks nothing more.
pub trait Exchange {
    /// The request to put to the relay next; `None` once the exchange is over and its outcome waits
    /// in it. Asked again before it is answered, it is the same request.
    fn request(&mut self) -> Option<Request>;

    /// The answer to the request [`Self::request`] gave, or why the link did not carry it.
    fn answer(&mut self, answer: Result<Response, LinkError>);
}

impl Exchange for RoundMachine {
    fn request(&mut self) -> Option<Request> {
        match self.poll() {
            Step::Request(request) => Some(request),
            Step::Done(_) => None,
        }
    }

    fn answer(&mut self, answer: Result<Response, LinkError>) {
        self.respond(answer);
    }
}

/// How a drive ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Driven {
    /// The exchange asked nothing more: it came to its end, and its outcome waits in it.
    Ended,
    /// The cancellation was raised: the request that was out, or the next one, was failed as the
    /// link's failure, and the exchange ended on it as it ends on any failure of its link. Its
    /// outcome waits in it, and what it took in is handed over by the next round.
    Cancelled,
    /// The deadline passed, with the same end as a cancellation.
    DeadlinePassed,
}

/// The time a drive reads its deadline by, and how it waits out a relay's refusal for the rate.
pub trait Clock {
    /// Now, on the clock a deadline is given in.
    fn now(&self) -> Instant;

    /// Wait `wait`, or less: return early once `cancel` is raised, and may return early for a
    /// reason of the clock's own. The driver reads the cancellation and the deadline again when it
    /// returns, and puts the request again unless one of them stops it, so that a wait cut short
    /// costs one more request, which the relay refuses again with the wait that is left.
    fn wait(&self, wait: Duration, cancel: &Signal);
}

/// The system's clock: [`Instant`], and a wait that the cancellation wakes, whichever thread raises
/// it ([`Signal::watch`]).
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }

    fn wait(&self, wait: Duration, cancel: &Signal) {
        let Some(until) = Instant::now().checked_add(wait) else {
            return;
        };
        let woken = Arc::new((Mutex::new(false), Condvar::new()));
        let _watch = {
            let woken = Arc::clone(&woken);
            cancel.watch(move || {
                let (raised, wake) = &*woken;
                *raised.lock().unwrap_or_else(PoisonError::into_inner) = true;
                wake.notify_all();
            })
        };
        let (raised, wake) = &*woken;
        let mut held = raised.lock().unwrap_or_else(PoisonError::into_inner);
        // Read after the watch began, so that a raise before it is not missed.
        if cancel.is_raised() {
            return;
        }
        while !*held {
            let now = Instant::now();
            if now >= until {
                return;
            }
            held = wake
                .wait_timeout(held, until - now)
                .unwrap_or_else(PoisonError::into_inner)
                .0;
        }
    }
}

/// Why the drive stops asking, when it does, and the failure the exchange is given for it: the
/// words a transport gives for the same stop. The cancellation is read before the deadline.
fn interrupted<C: Clock + ?Sized>(
    deadline: Option<Instant>,
    cancel: &Signal,
    clock: &C,
) -> Option<(Driven, TransportError)> {
    if cancel.is_raised() {
        Some((Driven::Cancelled, TransportError::Cancelled))
    } else if deadline.is_some_and(|deadline| clock.now() >= deadline) {
        Some((Driven::DeadlinePassed, TransportError::Deadline))
    } else {
        None
    }
}

/// The relay's word that a request may be asked again only after `retry_after_ms`.
fn rate_limited(answer: &Result<Response, LinkError>) -> Option<u64> {
    match answer {
        Ok(Response::Refused {
            code: Code::RateLimited { retry_after_ms },
            ..
        }) => Some(*retry_after_ms),
        _ => None,
    }
}

/// How long to wait before the request is put again, when the relay asked for `retry_after_ms` at
/// `now`: the wait, when it ends before `deadline` (or there is none). `None` when it ends at the
/// deadline or after (a wait to the deadline would end in the deadline, and the relay's answer is
/// worth more to the exchange than the deadline's), when it is `u64::MAX` (the relay's word for
/// never: an allowance that refills nothing), and when its end is past what the clock can count.
fn fits(now: Instant, retry_after_ms: u64, deadline: Option<Instant>) -> Option<Duration> {
    if retry_after_ms == u64::MAX {
        return None;
    }
    let wait = Duration::from_millis(retry_after_ms);
    let ends = now.checked_add(wait)?;
    match deadline {
        Some(deadline) if ends >= deadline => None,
        _ => Some(wait),
    }
}

/// Carry `exchange`'s requests over `link` until it asks nothing more, by `deadline` when there is
/// one, and stopping once `cancel` is raised; how it ended is returned, and the exchange's outcome
/// waits in it. The system's clock reads the deadline and waits out a refusal for the rate
/// ([`drive_with`]).
pub fn drive<E, L>(
    exchange: &mut E,
    link: &mut L,
    deadline: Option<Instant>,
    cancel: &Signal,
) -> Driven
where
    E: Exchange + ?Sized,
    L: Link + ?Sized,
{
    drive_with(exchange, link, deadline, cancel, &SystemClock)
}

/// [`drive`] on `clock`: the deadline is read by it, and a refusal for the rate is waited out on it.
///
/// Before each request the cancellation and the deadline are read, and a request asked after either
/// is not carried: the exchange is given the link's failure for it. A request that is out when
/// either comes is given up by the link ([`Link::call_within`]) and failed the same way. An answer
/// that is the relay's `rate_limited` (or its endpoint's refusal of the connection for the rate,
/// which the link gives as that) is not given to the exchange when its wait ends before the deadline
/// or there is none: the clock waits it out, woken by the cancellation, and the same request is put
/// again, as many times as the relay asks. A wait that would end at the deadline or after it, or
/// never, gives the exchange the answer, as any refusal is given; a cancellation raised during a
/// wait, or a deadline passed by the time it ends, fails the request as the link's failure, as
/// before any request. What the exchange makes of a failure of its link is its own: a round of a
/// courier ends, and leaves what it took in for the next round ([`crate::CourierState`]).
///
/// Each of these is a record of the `log` facade (`crate::logs`): the request answered (`debug`),
/// each wait for the rate with the request's kind, how long and how many times it was refused in a
/// row, a refusal for the rate given to the exchange because its wait does not fit, a link that
/// failed, and a drive stopped by its cancellation or deadline.
pub fn drive_with<E, L, C>(
    exchange: &mut E,
    link: &mut L,
    deadline: Option<Instant>,
    cancel: &Signal,
    clock: &C,
) -> Driven
where
    E: Exchange + ?Sized,
    L: Link + ?Sized,
    C: Clock + ?Sized,
{
    let mut driven = Driven::Ended;
    while let Some(request) = exchange.request() {
        let op = request.op();
        // How many times in a row the relay refused this request for the rate.
        let mut limited: u32 = 0;
        let answer = loop {
            if let Some((stop, error)) = interrupted(deadline, cancel, clock) {
                driven = stop;
                log::info!(op = op, reason = error.to_string().as_str(), limited = limited; "drive_stopped");
                break Err(LinkError(error.to_string()));
            }
            let answer = link.call_within(&request, deadline, cancel);
            if let Some(retry_after_ms) = rate_limited(&answer) {
                limited = limited.saturating_add(1);
                if let Some(wait) = fits(clock.now(), retry_after_ms, deadline) {
                    log::info!(
                        op = op,
                        wait_ms = retry_after_ms,
                        limited = limited,
                        deadline = deadline.is_some();
                        "rate_limited_wait"
                    );
                    clock.wait(wait, cancel);
                    continue;
                }
                let left_ms = deadline.map_or(u64::MAX, |deadline| {
                    u64::try_from(deadline.saturating_duration_since(clock.now()).as_millis())
                        .unwrap_or(u64::MAX)
                });
                log::info!(
                    op = op,
                    retry_after_ms = retry_after_ms,
                    left_ms = left_ms,
                    limited = limited;
                    "rate_limited_given"
                );
                break answer;
            }
            match &answer {
                Ok(response) => {
                    log::debug!(op = op, answer = response.tag(), limited = limited; "drive_answered");
                }
                Err(LinkError(reason)) => {
                    if let Some((stop, _)) = interrupted(deadline, cancel, clock) {
                        driven = stop;
                    }
                    log::info!(op = op, reason = reason.as_str(), limited = limited; "link_failed");
                }
            }
            break answer;
        };
        exchange.answer(answer);
    }
    driven
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, Ordering};

    use super::*;

    /// The system's clock waits as long as it is asked when nothing wakes it, returns at once for
    /// a cancellation raised before, and is woken by one raised on another thread during the wait.
    #[test]
    fn the_systems_wait_ends_at_its_time_or_at_the_cancellation() {
        let clock = SystemClock;
        let began = Instant::now();
        clock.wait(Duration::from_millis(30), &Signal::new());
        assert!(
            began.elapsed() >= Duration::from_millis(30),
            "it waited its time"
        );

        let raised = Signal::new();
        raised.raise();
        let began = Instant::now();
        clock.wait(Duration::from_secs(30), &raised);
        assert!(
            began.elapsed() < Duration::from_secs(5),
            "a cancellation raised before"
        );

        let cancel = Signal::new();
        let woke = Arc::new(AtomicBool::new(false));
        let raiser = {
            let (cancel, woke) = (cancel.clone(), Arc::clone(&woke));
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(50));
                woke.store(true, Ordering::SeqCst);
                cancel.raise();
            })
        };
        let began = Instant::now();
        clock.wait(Duration::from_secs(30), &cancel);
        assert!(woke.load(Ordering::SeqCst), "it returned after the raise");
        assert!(
            began.elapsed() < Duration::from_secs(5),
            "woken, not timed out"
        );
        raiser.join().unwrap();
    }

    /// The wait fits when it ends before the deadline, and not at it, after it, never, or past what
    /// the clock counts.
    #[test]
    fn a_wait_fits_when_it_ends_before_the_deadline() {
        let now = Instant::now();
        let deadline = now + Duration::from_millis(100);
        assert_eq!(fits(now, 0, None), Some(Duration::ZERO));
        assert_eq!(
            fits(now, 99, Some(deadline)),
            Some(Duration::from_millis(99))
        );
        assert_eq!(
            fits(now, 100, Some(deadline)),
            None,
            "exactly the time left"
        );
        assert_eq!(fits(now, 101, Some(deadline)), None);
        assert_eq!(
            fits(now, 1_000_000, None),
            Some(Duration::from_millis(1_000_000))
        );
        assert_eq!(fits(now, u64::MAX, None), None, "never");
        assert_eq!(
            fits(now, u64::MAX - 1, None).is_some(),
            now.checked_add(Duration::from_millis(u64::MAX - 1))
                .is_some()
        );
    }
}
