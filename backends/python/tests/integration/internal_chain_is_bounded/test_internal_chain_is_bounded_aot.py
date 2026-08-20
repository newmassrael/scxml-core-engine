# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# W3C SCXML 3.13 ends a macrostep at a configuration where nothing is enabled
# by NULL AND the internal queue is empty. Appendix D's Principles and
# Constraints then say that end need not exist: "A microstep always terminates.
# A macrostep may not. ... This is currently allowed." Python AOT path.
#
# `eventless_macrostep_is_bounded` owns the half of that clause built from
# transitions that need no event. This one owns the other half: a `<raise>`
# answered by a transition that raises again. Measured 2026-08-20 before the
# ceiling reached this branch, `send_event` on the fixture's `spin` document
# did not return on this engine — the internal drain had no budget at all, and
# `_drain_eventless`'s hundred was spent on the branch that was not running.
#
# Fixture: integration_resources/internal_chain_is_bounded/internal_chain_is_bounded.scxml
#
# Regeneration (after fixture or template edit):
#   scripts/regen_internal_chain_is_bounded_python.sh

from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import internal_chain_is_bounded_sm as _sm  # noqa: E402 — path inserted above
from sce_runtime.datamodel_read import read_int  # noqa: E402
from sce_runtime.scripting import LuaScriptEngine  # noqa: E402

_Event = _sm.InternalChainIsBoundedEvent

# The ceiling the engine applies, spelled here rather than read back from it. A
# test that asked the engine for its own limit would agree with any limit,
# including one an edit moved by three orders of magnitude.
MAX_MICROSTEPS = 1000

# One lap of the alternating chain is two microsteps — one internal event, one
# eventless transition — and only the internal half is counted, so a chain run
# to the shared ceiling records half.
ALTERNATING_LAPS_AT_CEILING = MAX_MICROSTEPS // 2


def _started():
    script_engine = LuaScriptEngine()
    script_engine.initialize()
    engine = _sm.create_engine(script_engine=script_engine)
    engine.initialize()
    return engine, script_engine


def _counter(engine, script_engine, name: str) -> int:
    """The fixture's `<assign>`s are the only witness of how far a chain got —
    every outcome leaves the machine in a state the configuration alone cannot
    tell apart from the others."""
    value = read_int(script_engine, engine.policy._session_id, name)
    assert value is not None, f"the fixture declares `{name}` in its datamodel"
    return value


def test_a_raise_chain_that_cannot_end_is_stopped() -> None:
    """The axis: a macrostep whose `<raise>` chain cannot end is stopped, and
    the host is told that it was.

    This test returning at all is half the assertion. Before the ceiling
    reached this branch it did not."""
    engine, se = _started()
    assert engine.truncated_macrosteps() == 0, (
        "nothing has been refused before the machine has done anything"
    )

    engine.send_event(_Event.SPIN)

    assert _counter(engine, se, "links") == MAX_MICROSTEPS, (
        "the chain must run exactly as far as the engine allows — fewer means "
        "the document was cut off early, more means the ceiling moved"
    )
    assert engine.truncated_macrosteps() == 1, (
        "the microstep past the budget was queued and was not taken. "
        "Without this count the host sees a machine that is running, in a "
        "state the document names, having returned at once — and no way to "
        "learn that the configuration it is reading is not a stable one"
    )
    assert str(engine.last_truncated_macrostep_state()) == "spin", (
        "the count alone says a document somewhere cannot settle; this says "
        f"where to look; it is {engine.last_truncated_macrostep_state()!s}"
    )
    assert engine.is_running, (
        "the chain was cut, not the machine. §scxml-D allows the document; "
        "refusing to run it forever is the engine's decision to report, not a "
        "reason to stop a machine whose other states still work"
    )


def test_a_raise_chain_that_ends_at_the_ceiling_is_not_refused() -> None:
    """The other half, and the one that makes the count mean something: a
    chain that ends on its own is not refused, however long it is.

    The fixture's bounded chain is exactly `MAX_MICROSTEPS` links for this
    reason. A ceiling that counted loop turns rather than microsteps taken, or
    that tested `>=` where it meant `>`, reports this ordinary document as a
    runaway."""
    engine, se = _started()

    engine.send_event(_Event.BOUNDED)

    assert _counter(engine, se, "laps") == MAX_MICROSTEPS, (
        "the guard `laps < 999` stops matching at the thousandth link, which "
        "raises nothing — so the queue empties and the chain stops by itself"
    )
    assert engine.truncated_macrosteps() == 0, (
        "nothing was refused: the macrostep reached the stable configuration "
        "§scxml-3.13 describes, using every microstep it was allowed. A long "
        "chain is not a runaway"
    )
    assert engine.last_truncated_macrostep_state() is None, (
        "and nothing names a state, because nothing was stopped"
    )
    assert engine.is_running, (
        "a document that settles on its own must not be reported dead by an "
        "engine that just finished running it correctly"
    )


def test_an_alternating_chain_spends_one_shared_budget() -> None:
    """The case a per-branch budget lets through: a chain that alternates one
    `<raise>` with one eventless transition.

    Neither branch of §scxml-D's inner loop reaches the ceiling on its own here
    — each takes every other microstep — so an engine that gives each branch a
    counter of its own runs this document forever with both ceilings half
    spent. One of the seven shipped exactly that pair of counters."""
    engine, se = _started()

    engine.send_event(_Event.ALTERNATE)

    assert _counter(engine, se, "alts") == ALTERNATING_LAPS_AT_CEILING, (
        "the two branches share one budget, so a chain that alternates them "
        "gets five hundred laps out of a thousand microsteps. A thousand here "
        "would mean the internal branch had a ceiling of its own"
    )
    assert engine.truncated_macrosteps() == 1, (
        "and the refusal is reported once, whichever branch was holding the "
        "budget when it ran out"
    )
    assert str(engine.last_truncated_macrostep_state()) == "alt", (
        "named the same way as any other chain that could not settle"
    )


def test_a_refused_chain_is_left_queued_for_the_next_macrostep() -> None:
    """What the refusal did with the links it would not run: it left them
    queued.

    The fixture's `resume` chain is half again as long as the ceiling, so the
    first macrostep is refused with five hundred links still to go and the
    second one finishes them. An engine that dropped the queue stops at a
    thousand and never finishes; one that ran the chain anyway finishes it in
    the first macrostep.

    The event driving the second macrostep is `poke`, and what it does is
    deliberately not asserted: §scxml-3.13 gives internal events priority, so
    this engine reaches it only after the chain, while the C++ AOT engine's
    `processEvent` takes the host's event first. That divergence is its own
    debt — the counters below are the same on both."""
    engine, se = _started()

    engine.send_event(_Event.RESUME)
    assert _counter(engine, se, "beats") == MAX_MICROSTEPS, (
        "the first macrostep spends the whole budget on the chain"
    )
    assert engine.truncated_macrosteps() == 1

    engine.send_event(_Event.POKE)

    assert _counter(engine, se, "beats") == MAX_MICROSTEPS + MAX_MICROSTEPS // 2, (
        "the second macrostep picked the chain up where the first was cut and "
        "ran it to its end — the refused links were left on the queue, not "
        "dropped"
    )
    assert engine.truncated_macrosteps() == 1, (
        "and nothing was refused this time: the chain ended on its own inside "
        "the budget, which is an ordinary macrostep however long the document "
        "took to get there"
    )
    assert engine.is_running


def test_an_ordinary_macrostep_is_not_counted() -> None:
    """The control: an ordinary document is untouched by any of this.

    Without it, an engine that refused every macrostep would pass the
    assertions above and fail nothing."""
    engine, se = _started()

    engine.send_event(_Event.POKE)

    assert _counter(engine, se, "pokes") == 1, (
        "the run happened: a counter of zero cannot tell an engine that did "
        "nothing from one that was never asked"
    )
    assert engine.truncated_macrosteps() == 0, (
        "and one transition is not a chain that cannot end"
    )
    assert engine.last_truncated_macrostep_state() is None
    assert str(engine.current_state) == "idle"
