// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What the owner of a session decides when it drains the session's inbox (docs/adr/0084, D2;
//! docs/adr/0147, D4): whether a drain begins, one at a time; how many inputs it takes from the
//! inbox, which is what the inbox held when the drain reached it; and whether an event of the
//! owner's own waits for a drain or joins the queue in the place of the input being applied. The
//! engine moves the inputs (the channel, the ring, the calls that apply them) and decides none of
//! this. These rules are here by themselves, over counts alone, and proved by Verus to refine the
//! owner's actions of `EngineInbox` (docs/formal/EngineInbox.tla): `Own`, with the drain before
//! an event of the owner's own and what that drain applies taken out of the inbox, `Boundary`,
//! `End` and `Late`.
//!
//! Taken as given, and judged elsewhere: an inbox is a queue of many producers and one consumer,
//! so what its owner takes is its oldest input and taking it removes it (the channel's and the
//! ring's own, held to that by the inbox history of docs/adr/0147, D3, and the ring's atomics by
//! GenMC, D5); and where a drain reads the count it is handed is where the posts behind that count
//! linearize after it.
//!
//! A session has one inbox. Its engine holds the `std` profile's own or one a deployment attached,
//! never both (`crate::inbox::SessionInbox`): two inboxes drained one after the other would apply
//! a post to the second ahead of an earlier post to the first, though the two posts did not overlap
//! in time, and `InboxService` promises the order the posts linearized in
//! (docs/formal/InboxService.tla; docs/adr/0147, D1). So a drain reaches one inbox: reached again before it is
//! over, it keeps the count of the inbox it reached first, and takes from no second one.
//!
//! The contracts and the proofs are read by Verus and by nothing else: without `cfg(verus_only)`
//! this file is the plain Rust it compiles as, and needs no crate. What is proved is of this code,
//! with no second rendering of it. `docs/formal/proofs.json` declares the proof, and the changes
//! of this code that it must refuse.

#[cfg(verus_only)]
use vstd::prelude::*;

/// Where the owner of a session stands in draining its inbox.
#[cfg_attr(verus_only, verus_verify)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Drain {
    /// The stage of the drain under way, or the rest between two.
    stage: Stage,
}

/// The stages of one drain, and the rest between two. Read only through a [`Drain`], whose
/// field holding it no code outside this module reaches.
#[cfg_attr(verus_only, verus_verify)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Stage {
    /// No drain is under way.
    Idle,
    /// A drain is under way and has not reached the session's inbox yet.
    Begun,
    /// The drain under way reached the session's inbox, and may still take this many inputs
    /// from it.
    Taking(usize),
}

/// What becomes of an event of the owner's own that is about to join its external queue.
#[cfg_attr(verus_only, verus_verify)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Own {
    /// No drain is under way, and one begins: what the inbox holds is applied first, and the
    /// event joins the queue behind it, as it arrived after it (docs/adr/0084, D2 as amended at
    /// step 9).
    AfterDrain,
    /// A drain is under way, and the event is one that applying an input it took queues: it joins
    /// the queue where it stands, in that input's place, and no drain runs inside the drain.
    InPlace,
}

#[cfg_attr(verus_only, verus_verify)]
impl Drain {
    /// The owner of a session before its first drain: none is under way.
    #[cfg_attr(verus_only, verus_spec(made =>
        ensures
            made.stage is Idle,
    ))]
    pub(crate) const fn idle() -> Drain {
        Drain { stage: Stage::Idle }
    }

    /// A drain is asked for, at a macrostep boundary or where the run ends: whether it begins.
    /// One drain at a time: one asked for while one is under way is that one, which takes what it
    /// reached in order, so this one is nothing.
    #[cfg_attr(verus_only, verus_spec(began =>
        ensures
            began == (old(self).stage is Idle),
            began ==> final(self).stage is Begun,
            !began ==> *final(self) == *old(self),
    ))]
    pub(crate) fn begin(&mut self) -> bool {
        if let Stage::Idle = self.stage {
            self.stage = Stage::Begun;
            return true;
        }
        false
    }

    /// The owner is about to put an event of its own on its external queue: whether a drain
    /// begins first, which the event waits for, or the event is the one an input being applied
    /// queues.
    #[cfg_attr(verus_only, verus_spec(own =>
        ensures
            (own is AfterDrain) == (old(self).stage is Idle),
            own is AfterDrain ==> final(self).stage is Begun,
            own is InPlace ==> *final(self) == *old(self),
    ))]
    pub(crate) fn own(&mut self) -> Own {
        if self.begin() {
            Own::AfterDrain
        } else {
            Own::InPlace
        }
    }

    /// The drain under way reaches the session's inbox, which gives `held` as the count of what it
    /// holds: the drain takes at most that many from it, so a post taken while it applies them
    /// waits for the next drain, and posters that never stop cannot hold the owner here. Nothing,
    /// when no drain is under way, and nothing when the drain reached the inbox already: a session
    /// has one inbox, and a drain that took from a second after the first would apply its posts
    /// out of the order they linearized in against the first's.
    #[cfg_attr(verus_only, verus_spec(
        ensures
            final(self).stage == (if old(self).stage is Begun { Stage::Taking(held) } else { old(self).stage }),
    ))]
    pub(crate) fn reach(&mut self, held: usize) {
        if let Stage::Begun = self.stage {
            self.stage = Stage::Taking(held);
        }
    }

    /// Whether the drain takes one more input from the inbox it reached, counted when it does.
    #[cfg_attr(verus_only, verus_spec(take =>
        ensures
            take == (old(self).stage matches Stage::Taking(due) && due > 0),
            take ==> (old(self).stage matches Stage::Taking(due) && final(self).stage == Stage::Taking((due - 1) as usize)),
            !take ==> *final(self) == *old(self),
    ))]
    pub(crate) fn take_another(&mut self) -> bool {
        match self.stage {
            Stage::Taking(due) => {
                if due == 0 {
                    return false;
                }
                self.stage = Stage::Taking(due - 1);
                true
            }
            _ => false,
        }
    }

    /// The inbox the drain reached gave no input where one was counted (a bounded ring is reached
    /// with its capacity, not with what it holds): the drain takes no more from it.
    #[cfg_attr(verus_only, verus_spec(
        ensures
            final(self).stage == (if old(self).stage is Taking { Stage::Taking(0usize) } else { old(self).stage }),
    ))]
    pub(crate) fn ran_dry(&mut self) {
        if let Stage::Taking(_) = self.stage {
            self.stage = Stage::Taking(0);
        }
    }

    /// The drain under way is over: the next one asked for begins.
    #[cfg_attr(verus_only, verus_spec(
        ensures
            final(self).stage is Idle,
    ))]
    pub(crate) fn finish(&mut self) {
        self.stage = Stage::Idle;
    }
}

#[cfg(verus_only)]
verus! {

/// One session of `EngineInbox` as its owner sees it: what its inbox holds, what its external queue
/// took, the order its posts and its own events linearized in (`lin`, a history variable no
/// runtime keeps), and whether its run ended. An input or an event is a number, each distinct.
pub(crate) ghost struct Door {
    inbox: Seq<int>,
    extq: Seq<int>,
    lin: Seq<int>,
    ended: bool,
}

/// What the inbox holds, applied to the external queue in the order the inbox took it, and taken
/// out of the inbox.
spec fn applied(door: Door) -> Door {
    Door { inbox: Seq::empty(), extq: door.extq + door.inbox, lin: door.lin, ended: door.ended }
}

/// `Boundary`, of this session: what its inbox holds is applied while its run goes on.
spec fn step_boundary(door: Door) -> Door {
    if door.ended { door } else { applied(door) }
}

/// `Own(e)`, with the drain before an event of the owner's own (`DrainBeforeOwn`) and what it
/// applies taken out of the inbox (`TakeOut`): what the inbox holds, and then `e`.
spec fn step_own(door: Door, e: int) -> Door {
    Door {
        inbox: Seq::empty(),
        extq: (door.extq + door.inbox).push(e),
        lin: door.lin.push(e),
        ended: door.ended,
    }
}

/// `End`, with what the inbox holds applied (`ApplyAtEnd`), as a direct call made then would be.
spec fn step_end(door: Door) -> Door {
    Door { inbox: Seq::empty(), extq: door.extq + door.inbox, lin: door.lin, ended: true }
}

/// `Late`: after the end, what the inbox took since is applied where the owner reaches the end of
/// its loop again. When it took nothing, this changes nothing.
spec fn step_late(door: Door) -> Door {
    applied(door)
}

/// The posts `later`, taken after a step of the owner: each joins the inbox and the order the
/// session's inputs linearized in (`Post`).
spec fn posted(door: Door, later: Seq<int>) -> Door {
    Door { inbox: door.inbox + later, extq: door.extq, lin: door.lin + later, ended: door.ended }
}

spec fn least(a: int, b: int) -> int {
    if a <= b { a } else { b }
}

/// What an inbox holds is what a drain takes, `taken`, and behind it what it took after, `later`.
proof fn taken_then_later(taken: Seq<int>, later: Seq<int>)
    ensures
        (taken + later).take(taken.len() as int) == taken,
        (taken + later).skip(taken.len() as int) == later,
        Seq::<int>::empty() + later == later,
{
    assert((taken + later).take(taken.len() as int) =~= taken);
    assert((taken + later).skip(taken.len() as int) =~= later);
    assert(Seq::<int>::empty() + later =~= later);
}

/// The session's inbox drained by the drain under way, as the engine drains it: reached with
/// `held`, the count it gave, it gives its oldest input while it holds one (`holds` of them,
/// `fifo` in the order it took them), and each input the drain takes is applied to the external
/// queue `extq` in the order it was taken. The drain takes what the inbox held when it reached it,
/// or all it has when it holds less, and leaves the rest.
fn a_drain_takes_what_it_reached(
    drain: &mut Drain,
    held: usize,
    holds: usize,
    fifo: Ghost<Seq<int>>,
    extq: Ghost<Seq<int>>,
) -> (after: Ghost<(Seq<int>, Seq<int>)>)
    requires
        old(drain).stage is Begun,
        fifo@.len() == holds,
    ensures
        final(drain).stage is Taking,
        after@.0 == extq@ + fifo@.take(least(held as int, holds as int)),
        after@.1 == fifo@.skip(least(held as int, holds as int)),
{
    drain.reach(held);
    let mut left = holds;
    let ghost mut queue = extq@;
    let ghost mut inbox = fifo@;
    proof {
        assert(fifo@.skip(0) =~= fifo@);
        assert(extq@ + fifo@.take(0) =~= extq@);
    }
    loop
        invariant_except_break
            drain.stage == Stage::Taking((held - (holds - left)) as usize),
        invariant
            fifo@.len() == holds,
            drain.stage is Taking,
            left <= holds,
            holds - left <= held,
            inbox == fifo@.skip(holds - left),
            queue == extq@ + fifo@.take(holds - left),
        ensures
            holds - left == least(held as int, holds as int),
        decreases left,
    {
        if !drain.take_another() {
            break;
        }
        if left == 0 {
            drain.ran_dry();
            break;
        }
        proof {
            let k = holds - left;
            assert(fifo@.take(k + 1) =~= fifo@.take(k).push(fifo@[k]));
            assert(fifo@.skip(k + 1) =~= fifo@.skip(k).skip(1));
            assert(inbox[0] == fifo@[k]);
            queue = queue.push(inbox[0]);
            inbox = inbox.skip(1);
            assert(queue =~= extq@ + fifo@.take(k + 1));
        }
        left = left - 1;
    }
    Ghost((queue, inbox))
}

/// A drain reaches one inbox, the session's one: a drain that took all the inbox it reached held,
/// reached again with any count before it is over, takes nothing more. So it takes from no second
/// inbox after the first, which would apply that inbox's posts out of the order they linearized in
/// against the first's (docs/adr/0147, D1).
fn a_drain_reaches_one_inbox(drain: &mut Drain, again: usize) -> (took: bool)
    requires
        old(drain).stage == Stage::Taking(0usize),
    ensures
        !took,
        final(drain).stage == Stage::Taking(0usize),
{
    drain.reach(again);
    drain.take_another()
}

/// A drain asked for where none is under way, of the session's inbox, as
/// `EngineState::drain_inbox` makes it: what it reached is applied in order and taken out, the
/// posts the inbox took behind it stay there, and the drain is over.
///
/// `door` is the session as the step finds it, its inbox what the drain takes; `later` is what the
/// inbox took behind that, the posts that linearized after the step (the module says why).
fn a_drain_applies_what_it_reached(
    drain: &mut Drain,
    door: Ghost<Door>,
    later: Ghost<Seq<int>>,
    held: usize,
    holds: usize,
) -> (after: Ghost<Door>)
    requires
        old(drain).stage is Idle,
        (door@.inbox + later@).len() == holds,
        door@.inbox.len() == least(held as int, holds as int),
    ensures
        final(drain).stage is Idle,
        after@ == posted(applied(door@), later@),
{
    let ghost mut stepped = door@;
    if drain.begin() {
        let got = a_drain_takes_what_it_reached(
            drain, held, holds, Ghost(door@.inbox + later@), Ghost(door@.extq));
        drain.finish();
        proof {
            taken_then_later(door@.inbox, later@);
            stepped = Door { inbox: got@.1, extq: got@.0, lin: door@.lin + later@, ended: door@.ended };
        }
    }
    Ghost(stepped)
}

/// A macrostep boundary of a session whose run goes on is a step of `Boundary`, then the posts
/// that came while it applied.
fn a_boundary_is_a_step_of_boundary(
    drain: &mut Drain,
    door: Ghost<Door>,
    later: Ghost<Seq<int>>,
    held: usize,
    holds: usize,
) -> (after: Ghost<Door>)
    requires
        old(drain).stage is Idle,
        !door@.ended,
        (door@.inbox + later@).len() == holds,
        door@.inbox.len() == least(held as int, holds as int),
    ensures
        final(drain).stage is Idle,
        after@ == posted(step_boundary(door@), later@),
{
    a_drain_applies_what_it_reached(drain, door, later, held, holds)
}

/// An event `e` of the owner's own, put on the queue of a session whose run goes on, as
/// `EngineState::enqueue_external` puts it there, is a step of `Own(e)`: what the inbox held is
/// applied and taken out first, and `e` joins the queue behind it; then the posts that came
/// meanwhile.
fn an_own_event_is_a_step_of_own(
    drain: &mut Drain,
    door: Ghost<Door>,
    later: Ghost<Seq<int>>,
    held: usize,
    holds: usize,
    e: Ghost<int>,
) -> (after: Ghost<Door>)
    requires
        old(drain).stage is Idle,
        !door@.ended,
        (door@.inbox + later@).len() == holds,
        door@.inbox.len() == least(held as int, holds as int),
    ensures
        final(drain).stage is Idle,
        after@ == posted(step_own(door@, e@), later@),
{
    let ghost mut stepped = door@;
    if let Own::AfterDrain = drain.own() {
        let got = a_drain_takes_what_it_reached(
            drain, held, holds, Ghost(door@.inbox + later@), Ghost(door@.extq));
        drain.finish();
        proof {
            taken_then_later(door@.inbox, later@);
            stepped = Door {
                inbox: got@.1,
                extq: got@.0.push(e@),
                lin: door@.lin.push(e@) + later@,
                ended: door@.ended,
            };
        }
    }
    Ghost(stepped)
}

/// The end of a session's run, where the engine's loop stops and `EngineState::end_run` drains, is
/// a step of `End`: what the inbox holds is applied, not left. That the run is over is the
/// engine's to say (its loop goes on no more); what the step asks of the inbox is this drain.
fn the_end_is_a_step_of_end(
    drain: &mut Drain,
    door: Ghost<Door>,
    later: Ghost<Seq<int>>,
    held: usize,
    holds: usize,
) -> (after: Ghost<Door>)
    requires
        old(drain).stage is Idle,
        !door@.ended,
        (door@.inbox + later@).len() == holds,
        door@.inbox.len() == least(held as int, holds as int),
    ensures
        final(drain).stage is Idle,
        after@ == posted(step_end(door@), later@),
{
    let got = a_drain_applies_what_it_reached(drain, door, later, held, holds);
    Ghost(Door { inbox: got@.inbox, extq: got@.extq, lin: got@.lin, ended: true })
}

/// Each later end of the owner's loop over a session whose run is over drains again: a step of
/// `Late`, which applies what the inbox took since the end, or none when it took nothing.
fn a_later_end_is_a_step_of_late(
    drain: &mut Drain,
    door: Ghost<Door>,
    later: Ghost<Seq<int>>,
    held: usize,
    holds: usize,
) -> (after: Ghost<Door>)
    requires
        old(drain).stage is Idle,
        door@.ended,
        (door@.inbox + later@).len() == holds,
        door@.inbox.len() == least(held as int, holds as int),
    ensures
        final(drain).stage is Idle,
        after@ == posted(step_late(door@), later@),
{
    a_drain_applies_what_it_reached(drain, door, later, held, holds)
}

}

#[cfg(test)]
mod tests {
    use super::{Drain, Own};

    /// A drain under way that reached an inbox which gave `held` as its count.
    fn reached(held: usize) -> Drain {
        let mut drain = Drain::idle();
        assert!(drain.begin(), "no drain was under way, so one begins");
        drain.reach(held);
        drain
    }

    #[test]
    fn a_drain_takes_what_its_inbox_held_when_reached_and_no_more() {
        let mut drain = reached(2);
        assert!(drain.take_another());
        assert!(drain.take_another());
        assert!(
            !drain.take_another(),
            "the drain took past what the inbox held when it reached it"
        );
    }

    #[test]
    fn a_drain_reached_again_keeps_the_count_of_the_inbox_it_reached_first() {
        let mut drain = reached(1);
        drain.reach(5);
        assert!(drain.take_another());
        assert!(
            !drain.take_another(),
            "a second reach in one drain handed it a second inbox's count"
        );

        let mut emptied = reached(1);
        assert!(emptied.take_another());
        emptied.reach(3);
        assert!(
            !emptied.take_another(),
            "a drain that emptied the inbox it reached took from a second one"
        );
    }

    #[test]
    fn one_drain_at_a_time_and_an_own_event_waits_only_outside_one() {
        let mut drain = Drain::idle();
        assert_eq!(drain.own(), Own::AfterDrain);
        assert!(!drain.begin(), "a drain began inside a drain");
        assert_eq!(drain.own(), Own::InPlace);
        drain.finish();
        assert!(
            drain.begin(),
            "a drain asked for after one was over did not begin"
        );
    }

    #[test]
    fn a_reach_outside_a_drain_and_a_dry_inbox_take_nothing() {
        let mut idle = Drain::idle();
        idle.reach(4);
        assert!(
            !idle.take_another(),
            "a drain that never began took an input"
        );

        let mut dry = reached(4);
        dry.ran_dry();
        assert!(
            !dry.take_another(),
            "a drain took from an inbox that ran dry"
        );
    }
}
