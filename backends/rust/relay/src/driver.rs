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

use std::time::Instant;

use sce_rust_relay_transport::{Signal, TransportError};

use crate::client::{Link, LinkError, RoundMachine, Step};
use crate::protocol::{Request, Response};

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

/// Why the drive stops asking, when it does, and the failure the exchange is given for it: the
/// words a transport gives for the same stop. The cancellation is read before the deadline.
fn interrupted(deadline: Option<Instant>, cancel: &Signal) -> Option<(Driven, TransportError)> {
    if cancel.is_raised() {
        Some((Driven::Cancelled, TransportError::Cancelled))
    } else if deadline.is_some_and(|deadline| Instant::now() >= deadline) {
        Some((Driven::DeadlinePassed, TransportError::Deadline))
    } else {
        None
    }
}

/// Carry `exchange`'s requests over `link` until it asks nothing more, by `deadline` when there is
/// one, and stopping once `cancel` is raised; how it ended is returned, and the exchange's outcome
/// waits in it.
///
/// Before each request the cancellation and the deadline are read, and a request asked after either
/// is not carried: the exchange is given the link's failure for it. A request that is out when
/// either comes is given up by the link ([`Link::call_within`]) and failed the same way. What the
/// exchange makes of a failure of its link is its own: a round of a courier ends, and leaves what
/// it took in for the next round ([`crate::CourierState`]).
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
    let mut driven = Driven::Ended;
    while let Some(request) = exchange.request() {
        let answer = match interrupted(deadline, cancel) {
            Some((stop, error)) => {
                driven = stop;
                Err(LinkError(error.to_string()))
            }
            None => {
                let answer = link.call_within(&request, deadline, cancel);
                if answer.is_err() {
                    if let Some((stop, _)) = interrupted(deadline, cancel) {
                        driven = stop;
                    }
                }
                answer
            }
        };
        exchange.answer(answer);
    }
    driven
}
