// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What the edge's limiters decide besides the rule itself (docs/adr/0109, D2): the arrival time a
//! key is judged by, what a request is answered and whether it spends a permit, how many keys a
//! limiter tracks and where a key goes in its table, which of two refusals an exchange is told,
//! the whole seconds a `Retry-After` asks, and which address of a `Forwarded` header (RFC 7239)
//! is the client's. `Limiter`, `Edge` and `client_address` hold the tables, the clock and the
//! addresses, and decide none of this. These rules are here by themselves, over numbers alone, and
//! proved by Verus (docs/adr/0115, D13): a limiter's table never tracks more keys than its bound; a
//! refused request spends nothing, and one the rule could not judge is refused; a fresh key that
//! is dropped is judged after as if it had been kept; an exchange is refused while either failure
//! limiter refuses it, with the longer wait; a `Retry-After` asks no less than the wait and no
//! more than a second over; and the client is the rightmost address that is no trusted proxy.
//!
//! The decision of a limiter is `sce:std/rate/gcra_admit`'s, generated into this crate: here its
//! answer is an argument, and a proof below takes of the rule what its document says it reads. Its
//! own contract is judged where it is generated (docs/adr/0115, D10).
//!
//! The contracts and the proofs are read by Verus and by nothing else: without `cfg(verus_only)`
//! this file is the plain Rust it compiles as, and needs no crate. What is proved is of this code,
//! with no second rendering of it. `docs/formal/proofs.json` declares the proof, and the changes
//! of this code that it must refuse.

#[cfg(verus_only)]
use vstd::prelude::*;

/// What a limiter answers a request with: within the rate, or past it, with the wait in
/// milliseconds after which the same request would be within it.
#[cfg_attr(verus_only, verus_verify)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Answer {
    Admitted,
    Refused { wait_ms: u64 },
}

/// Where a key whose arrival time moves goes in a limiter's table.
#[cfg_attr(verus_only, verus_verify)]
pub(crate) enum Room<T> {
    /// The table tracks the key, at the arrival time it holds, which the new one replaces.
    Held(T),
    /// The table does not track the key, and has room for it.
    Free,
    /// The table does not track the key and is at its bound: the key of the earliest arrival goes
    /// first.
    Full,
}

/// One address a `Forwarded` header names, as the walk from the right reaches it.
#[cfg_attr(verus_only, verus_verify)]
pub(crate) enum Hop<A> {
    /// A name that is no address (`unknown`, or obfuscated): the walk ends, and the client is the
    /// trusted hop that named it.
    Ends,
    /// An address of a trusted proxy: the client so far, and the walk goes on.
    Passes(A),
    /// An address that is no trusted proxy: the client.
    Client(A),
}

/// The arrival time a limiter judges a key by: the one it keeps of the key, or 0 for a key it does
/// not track, a fresh limiter, at or before any reading of a clock that starts at 0.
#[cfg_attr(verus_only, verus_verify)]
#[cfg_attr(verus_only, verus_spec(judged_by =>
    ensures
        kept matches Some(at) ==> judged_by == at,
        kept is None ==> judged_by == 0,
))]
pub(crate) fn arrival(kept: Option<i64>) -> i64 {
    kept.unwrap_or(0)
}

/// The rule's interval for a rate of `interval_ms`: as it is, or the longest the rule's 64 bits
/// hold, never a negative one.
#[cfg_attr(verus_only, verus_verify)]
#[cfg_attr(verus_only, verus_spec(interval =>
    ensures
        interval as int == (if interval_ms as int <= i64::MAX as int { interval_ms as int } else { i64::MAX as int }),
))]
pub(crate) fn interval(interval_ms: u64) -> i64 {
    if interval_ms > i64::MAX as u64 {
        i64::MAX
    } else {
        interval_ms as i64
    }
}

/// What a request is answered, and the arrival time to keep for its key: `decided` is the rule's
/// decision (whether it admitted, the arrival time it moved to, the wait it asks), `None` when the
/// rule refused its arguments. Only an admitted request keeps an arrival time, so a refused one
/// spends nothing. The build bounds a limiter's span far below what the rule's 64-bit arithmetic
/// holds, so the rule refuses nothing it is given; were it to, the request is refused for one
/// interval, `interval_ms`, rather than admitted unjudged.
#[cfg_attr(verus_only, verus_verify)]
#[cfg_attr(verus_only, verus_spec(judged =>
    ensures
        decided is None ==> judged.0 == Answer::Refused { wait_ms: interval_ms } && judged.1 is None,
        decided matches Some((admits, moved_to, asks)) ==> (if admits {
            judged.0 == Answer::Admitted && judged.1 == Some(moved_to)
        } else {
            judged.0 == Answer::Refused { wait_ms: if asks < 0 { u64::MAX } else { asks as u64 } }
                && judged.1 is None
        }),
))]
pub(crate) fn answer(decided: Option<(bool, i64, i64)>, interval_ms: u64) -> (Answer, Option<i64>) {
    match decided {
        None => (
            Answer::Refused {
                wait_ms: interval_ms,
            },
            None,
        ),
        Some((admitted, arrival, wait)) => {
            if admitted {
                (Answer::Admitted, Some(arrival))
            } else if wait < 0 {
                (Answer::Refused { wait_ms: u64::MAX }, None)
            } else {
                (
                    Answer::Refused {
                        wait_ms: wait as u64,
                    },
                    None,
                )
            }
        }
    }
}

/// How many keys a limiter of `max_tracked` tracks at most: at least one, since a table that may
/// hold none still keeps the key it was just asked about.
#[cfg_attr(verus_only, verus_verify)]
#[cfg_attr(verus_only, verus_spec(most =>
    ensures
        most as int == (if max_tracked >= 1 { max_tracked as int } else { 1 }),
))]
pub(crate) fn bound(max_tracked: u32) -> usize {
    if max_tracked == 0 {
        1
    } else {
        max_tracked as usize
    }
}

/// Where a key goes in a table of `tracked` keys bounded by `bound`, its arrival time there
/// `held`, `None` when the table does not track it.
#[cfg_attr(verus_only, verus_verify)]
#[cfg_attr(verus_only, verus_spec(place =>
    ensures
        held matches Some(kept) ==> place == Room::Held(kept),
        held is None ==> ((place is Full) == (bound <= tracked)),
        held is None ==> ((place is Free) == (tracked < bound)),
))]
pub(crate) fn room<T>(tracked: usize, bound: usize, held: Option<T>) -> Room<T> {
    match held {
        Some(held) => Room::Held(held),
        None => {
            if tracked >= bound {
                Room::Full
            } else {
                Room::Free
            }
        }
    }
}

/// Of two answers, the one that asks the longer wait: refused when either is.
#[cfg_attr(verus_only, verus_verify)]
#[cfg_attr(verus_only, verus_spec(longest =>
    ensures
        wait(longest) == (if wait(a) < wait(b) { wait(b) } else { wait(a) }),
))]
pub(crate) fn longer(a: Answer, b: Answer) -> Answer {
    match (a, b) {
        (Answer::Refused { wait_ms: x }, Answer::Refused { wait_ms: y }) => Answer::Refused {
            wait_ms: core::cmp::max(x, y),
        },
        (Answer::Refused { .. }, Answer::Admitted) => a,
        (Answer::Admitted, _) => b,
    }
}

/// The seconds a `Retry-After` header asks for a wait of `wait_ms`: rounded up, at least one,
/// and the most a `u64` holds when the wait is longer than that.
#[cfg_attr(verus_only, verus_verify)]
#[cfg_attr(verus_only, verus_spec(seconds =>
    ensures
        seconds >= 1,
        seconds == u64::MAX || wait_ms as int <= seconds as int * 1000,
        seconds == 1 || (seconds as int - 1) * 1000 < wait_ms as int,
))]
pub(crate) fn seconds(wait_ms: u128) -> u64 {
    let whole = wait_ms / 1000;
    let rounded = if wait_ms.is_multiple_of(1000) {
        whole
    } else {
        whole + 1
    };
    if rounded == 0 {
        1
    } else if rounded > u64::MAX as u128 {
        u64::MAX
    } else {
        rounded as u64
    }
}

/// One address of a `Forwarded` header, `None` for a name that is no address, and otherwise the
/// address with whether it is of a trusted proxy: where the walk from the right goes.
#[cfg_attr(verus_only, verus_verify)]
#[cfg_attr(verus_only, verus_spec(step =>
    ensures
        named is None ==> step is Ends,
        named matches Some((address, trusted)) ==> step == (if trusted { Hop::Passes(address) } else { Hop::Client(address) }),
))]
pub(crate) fn hop<A>(named: Option<(A, bool)>) -> Hop<A> {
    match named {
        None => Hop::Ends,
        Some((address, trusted)) => {
            if trusted {
                Hop::Passes(address)
            } else {
                Hop::Client(address)
            }
        }
    }
}

#[cfg(verus_only)]
verus! {

/// How long an answer asks to wait, in milliseconds: -1, below every wait, when it asks none.
spec fn wait(answer: Answer) -> int {
    match answer {
        Answer::Admitted => -1,
        Answer::Refused { wait_ms } => wait_ms as int,
    }
}

/// What `sce:std/rate/gcra_admit` reads of a key's arrival time at `now`: the later of the two,
/// its `start`.
spec fn start(arrival: int, now: int) -> int {
    if arrival > now { arrival } else { now }
}

/// The bound docs/adr/0109, D2 sets a table of `max_tracked`: at least one key.
spec fn at_most(max_tracked: u32) -> int {
    if max_tracked == 0 { 1 } else { max_tracked as int }
}

/// Whether one hop of the walk is an address of a trusted proxy.
spec fn passes(named: Option<(u128, bool)>) -> bool {
    match named {
        Some((_, trusted)) => trusted,
        None => false,
    }
}

/// A request is answered as the rule decided, and spends a permit of its key only when admitted,
/// as `Limiter::spend` spends it: a refusal keeps no arrival time, and a request the rule could
/// not judge is refused, never admitted unjudged.
fn a_request_spends_only_when_admitted(
    decided: Option<(bool, i64, i64)>, interval_ms: u64,
) -> (judged: (Answer, Option<i64>))
    ensures
        (judged.1 is Some) == (judged.0 is Admitted),
        judged.0 is Admitted ==> (decided matches Some((admits, _, _)) && admits),
        decided is None ==> judged.0 is Refused,
{
    answer(decided, interval_ms)
}

/// A key the table dropped while it was fresh, its arrival time at or before the moment it was
/// dropped, is judged after as it would have been had the table kept it, as `Limiter::judge` judges
/// a key it does not track: the rule reads of an arrival time only whether it is later than now,
/// and a clock that starts at 0 and never runs back reads no earlier than the 0 an untracked key is
/// judged by. So a table that drops the key of the earliest arrival, a fresh one whenever it holds
/// one (docs/adr/0109, D2), changes no answer by it.
///
/// Taken as given of the rule: it reads a key's arrival time only through `start`. Of the clock: it
/// starts at 0 and never runs back.
fn a_fresh_key_dropped_is_judged_as_if_kept(kept: i64, dropped_at: i64, now: i64) -> (judged_by: (i64, i64))
    requires
        kept <= dropped_at,
        0 <= dropped_at <= now,
    ensures
        judged_by.1 == kept,
        start(judged_by.0 as int, now as int) == start(judged_by.1 as int, now as int),
{
    (arrival(None), arrival(Some(kept)))
}

/// A spend that keeps an arrival time leaves the table within its bound, as `Limiter::set` keeps
/// it: a key the table tracks is replaced; a new one takes a free place; and at the bound the key
/// of the earliest arrival is dropped first, which a table at a bound of at least one holds.
fn a_table_stays_within_its_bound(max_tracked: u32, tracked: usize, held: Option<i64>) -> (after: Ghost<int>)
    requires
        tracked as int <= at_most(max_tracked),
    ensures
        after@ <= at_most(max_tracked),
        held is Some ==> after@ == tracked as int,
        held is None ==> after@ == tracked as int + 1 || (after@ == tracked as int && after@ == at_most(max_tracked)),
{
    let most = bound(max_tracked);
    let place = room(tracked, most, held);
    let ghost after = match place {
        Room::Held(_) => tracked as int,
        Room::Free => tracked as int + 1,
        Room::Full => tracked as int,
    };
    Ghost(after)
}

/// An exchange reaches the authenticator only while neither failure limiter refuses it, as
/// `Edge::may_authenticate` asks them: refused when the failures of its name or of its address are
/// past their rate, with the longer of the two waits, so that `Retry-After` asks for no less than
/// either.
fn an_exchange_is_refused_while_either_failure_limiter_refuses(
    by_name: Answer, by_address: Answer,
) -> (told: Answer)
    ensures
        (told is Refused) == (by_name is Refused || by_address is Refused),
        wait(by_name) <= wait(told),
        wait(by_address) <= wait(told),
        told is Refused ==> wait(told) == wait(by_name) || wait(told) == wait(by_address),
{
    longer(by_name, by_address)
}

/// The address a connection from a trusted proxy is taken to come from, as `client_address` walks
/// the addresses its `Forwarded` headers name (RFC 7239), `peer` being the proxy's own: `hops`
/// holds them in the order they arrived, each with whether it is a trusted proxy, `None` for a name
/// that is no address. From the right, the first address that is no trusted proxy is the client;
/// the walk ends at a name that is no address, at the trusted hop that named it; and when every
/// address is a trusted proxy, the leftmost is the client. Returned with the hop that names it,
/// `hops.len()` for the proxy itself.
fn the_client_is_the_rightmost_address_not_a_trusted_proxy(
    peer: u128, hops: &[Option<(u128, bool)>],
) -> (client: (u128, Ghost<int>))
    ensures
        0 <= client.1@ <= hops@.len(),
        forall|i: int| client.1@ < i < hops@.len() ==> passes(#[trigger] hops@[i]),
        client.1@ == hops@.len() ==> client.0 == peer && (hops@.len() == 0 || hops@.last() is None),
        client.1@ < hops@.len() ==> hops@[client.1@] == Some((client.0, false))
            || (hops@[client.1@] == Some((client.0, true)) && (client.1@ == 0 || hops@[client.1@ - 1] is None)),
{
    let mut client = peer;
    let mut at = hops.len();
    loop
        invariant
            at <= hops@.len(),
            forall|i: int| at < i < hops@.len() ==> passes(#[trigger] hops@[i]),
            at == hops@.len() ==> client == peer,
            at < hops@.len() ==> hops@[at as int] == Some((client, true)),
        decreases at,
    {
        if at == 0 {
            return (client, Ghost(0int));
        }
        let named = hops[at - 1];
        match hop(named) {
            Hop::Ends => {
                return (client, Ghost(at as int));
            }
            Hop::Passes(address) => {
                proof {
                    if at < hops@.len() {
                        assert(passes(hops@[at as int]));
                    }
                }
                client = address;
                at = at - 1;
            }
            Hop::Client(address) => {
                proof {
                    if at < hops@.len() {
                        assert(passes(hops@[at as int]));
                    }
                }
                return (address, Ghost(at as int - 1));
            }
        }
    }
}

}
