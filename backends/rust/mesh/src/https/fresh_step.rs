// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The freshness step of the HTTPS binding (docs/adr/0117, D4) over numbers alone: which step's
//! value a server takes, which value a client signs, and when a client that was refused signs
//! again at once. These are the rules a request signed ahead serves a thief by, and a client is
//! left without a request taken by, so they are here by themselves and proved by Verus
//! (docs/adr/0115, D13), as `client_mailbox` is. The values themselves, and the key they are
//! made of, are the module `freshness`'s.
//!
//! The contracts and the proofs are read by Verus and by nothing else: without `cfg(verus_only)`
//! this file is the plain Rust it compiles as, and needs no crate. `docs/formal/proofs.json`
//! declares the proof, and the changes of this code that it must refuse.
//!
//! `docs/formal/Freshness.tla` is the protocol these refine: the server's `Good` is [`admitted`],
//! the page's `has` is [`Held`], and its `Exchange` refused and told the present value is
//! [`Held::give`] then [`Held::again_at_once`].

#[cfg(verus_only)]
use vstd::prelude::*;

/// Whether a value made for `step` is taken at the step `present`: the present one, or the one
/// before it. A value of a step that has not come is known to nobody (the server makes it), and
/// one older than the step before is refused.
#[cfg_attr(verus_only, verus_verify)]
#[cfg_attr(verus_only, verus_spec(taken =>
    ensures
        taken == (step == present || (present >= 1 && step == present - 1)),
))]
pub(crate) fn admitted(present: u64, step: u64) -> bool {
    step == present || (present >= 1 && step == present - 1)
}

/// The value a client signs, by what it knows of it: the step it was made for (`None` before the
/// first was given), and how many values other than the one held were given so far, the
/// generation of the one held. A request records the generation it signed.
#[cfg_attr(verus_only, verus_verify)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Held {
    pub(crate) step: Option<u64>,
    pub(crate) generation: u64,
}

#[cfg_attr(verus_only, verus_verify)]
impl Held {
    /// A client that was given no value: its requests sign none.
    #[cfg_attr(verus_only, verus_spec(made =>
        ensures
            made.step is None,
            made.generation == 0,
    ))]
    pub(crate) fn new() -> Held {
        Held {
            step: None,
            generation: 0,
        }
    }

    /// An answer that the client took gave a value made for `step`, `other` when its text is not
    /// the one held: whether the client signs it from now on. It does when it is another value
    /// and no older than the one held, so that an answer that arrives late puts back no value a
    /// later one replaced; a value of the same step that is another is the server's present one
    /// under a key it drew again, and is taken.
    #[cfg_attr(verus_only, verus_spec(replaced =>
        ensures
            replaced == (other && old(self).generation < u64::MAX
                && match old(self).step { Some(held) => held <= step, None => true }),
            *final(self) == if replaced {
                Held { step: Some(step), generation: (old(self).generation + 1) as u64 }
            } else {
                *old(self)
            },
    ))]
    pub(crate) fn give(&mut self, step: u64, other: bool) -> bool {
        if !other || self.generation == u64::MAX {
            return false;
        }
        if let Some(held) = self.step {
            if held > step {
                return false;
            }
        }
        self.step = Some(step);
        self.generation += 1;
        true
    }

    /// A refusal answered a request that signed the value of generation `sent`: whether its lane
    /// signs again at once, which it does when it holds a value other than the one refused. One
    /// that holds nothing newer waits, so that a server that refuses what it gave is not asked
    /// again and again without a pause.
    #[cfg_attr(verus_only, verus_spec(again =>
        ensures
            again == (sent < self.generation),
    ))]
    pub(crate) fn again_at_once(&self, sent: u64) -> bool {
        sent < self.generation
    }
}

#[cfg(verus_only)]
verus! {

/// A value a server takes was made for the present step or the one before: what was signed ahead
/// by one who was inside the page at the step `left`, and so for no step after it, serves them
/// in the step after `left` and no later (`Bounded` of `docs/formal/Freshness.tla`).
fn a_value_taken_serves_one_step_past_its_own(present: u64, step: u64) -> (taken: bool)
    ensures
        taken ==> step <= present && present <= step + 1,
        step == present ==> taken,
{
    admitted(present, step)
}

/// A refusal of a request that signed the value of generation `sent`, which gave a value made
/// for `step`, `other` when it is not the value held. The client holds no older value after it;
/// it signs again at once only a value given after the one refused, so the signings at once of
/// one exchange each sign a later value than the one before, and are no more than the values the
/// server gave; and it waits only when the value it holds is the one refused, never while it
/// holds one it has not tried (`Taken` of `docs/formal/Freshness.tla`).
fn a_refusal_signs_again_only_a_value_given_since(
    held: &mut Held, sent: u64, step: u64, other: bool,
) -> (again: bool)
    requires
        sent <= old(held).generation,
    ensures
        final(held).generation >= old(held).generation,
        again ==> final(held).generation > sent,
        !again ==> final(held).generation == sent,
        (old(held).step matches Some(before) ==> final(held).step matches Some(after) && before <= after),
{
    held.give(step, other);
    held.again_at_once(sent)
}

}
