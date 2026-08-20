# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# W3C SCXML 3.13 says a macrostep is a chain of microsteps ending in a
# configuration where nothing is enabled by NULL. Appendix D's Principles and
# Constraints then say the chain need not exist: "A microstep always
# terminates. A macrostep may not. ... This is currently allowed." Python AOT
# path.
#
# This engine is where that allowance was measured, on 2026-08-20: it was the
# only one of the seven with no ceiling here at all, and `initialize()` on a
# two-state cyclic document did not return. That is the conformant reading of
# the clause and it is also the one an unattended host cannot act on — the
# other six stopped the chain instead and said nothing a program could read.
#
# `error_cascade_is_bounded` owns the chain built from errors; this one owns
# the chain built from transitions that need no event at all. The fixture
# separates a chain that stops on its own — a HUNDRED microsteps, exactly the
# ceiling, which is where an off-by-one lands — from one that cannot stop.
#
# Fixture: integration_resources/eventless_macrostep_is_bounded/eventless_macrostep_is_bounded.scxml
#
# Regeneration (after fixture or template edit):
#   scripts/regen_eventless_macrostep_is_bounded_python.sh

from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import eventless_macrostep_is_bounded_sm as _sm  # noqa: E402 — path inserted above
from sce_runtime.datamodel_read import read_int  # noqa: E402
from sce_runtime.scripting import LuaScriptEngine  # noqa: E402

_Event = _sm.EventlessMacrostepIsBoundedEvent

# The ceiling the engine applies, spelled here rather than read back from it. A
# test that asked the engine for its own limit would agree with any limit,
# including one an edit moved by three orders of magnitude.
MAX_MICROSTEPS = 1000

# One lap of either chain is two microsteps (`_a` to `_b`, then back), and only
# the `_a` edge counts, so a chain run to the ceiling records half.
LAPS_AT_CEILING = MAX_MICROSTEPS // 2


def _started():
    script_engine = LuaScriptEngine()
    script_engine.initialize()
    engine = _sm.create_engine(script_engine=script_engine)
    engine.initialize()
    return engine, script_engine


def _counter(engine, script_engine, name: str) -> int:
    """The fixture's `<assign>`s are the only witness of how far a chain got —
    the configuration alone cannot tell a chain that stopped from one that was
    stopped."""
    value = read_int(script_engine, engine.policy._session_id, name)
    assert value is not None, f"the fixture declares `{name}` in its datamodel"
    return value


def test_a_macrostep_that_cannot_end_is_stopped() -> None:
    """The axis: a macrostep whose eventless chain cannot end is stopped, and
    the host is told that it was.

    This test returning at all is half the assertion. Before the ceiling
    existed it did not — this exact engine ran until it was killed."""
    engine, se = _started()
    assert engine.truncated_macrosteps() == 0, (
        "nothing has been refused before the machine has done anything"
    )

    engine.send_event(_Event.SPIN)

    assert _counter(engine, se, "spins") == LAPS_AT_CEILING, (
        "the chain must run exactly as far as the engine allows — fewer means "
        "the document was cut off early, more means the ceiling moved"
    )
    assert engine.truncated_macrosteps() == 1, (
        "the microstep past the budget was enabled and was not taken. "
        "Without this count the host sees a machine that is running, in a "
        "state the document names, having returned at once — and no way to "
        "learn that the configuration it is reading is not a stable one"
    )
    assert str(engine.last_truncated_macrostep_state()) == "spin_a", (
        "an eventless cycle is a closed walk through the state graph, and the "
        "count alone does not say which walk. This names a state on it, which "
        "is where an author looks first; it is "
        f"{engine.last_truncated_macrostep_state()!s}"
    )
    assert engine.is_running, (
        "the chain was cut, not the machine. §scxml-D allows the document; "
        "refusing to run it forever is the engine's decision to report, not a "
        "reason to stop a machine whose other states still work"
    )


def test_a_chain_that_ends_at_the_ceiling_is_not_refused() -> None:
    """The other half, and the one that makes the count mean something: a
    chain that ends on its own is not refused, however long it is.

    The fixture's bounded chain is exactly `MAX_MICROSTEPS` microsteps for
    this reason. A ceiling that counted loop turns rather than microsteps
    taken, or that tested `>=` where it meant `>`, reports this ordinary
    document as a runaway."""
    engine, se = _started()

    engine.send_event(_Event.BOUNDED)

    assert _counter(engine, se, "laps") == LAPS_AT_CEILING, (
        "the guard `laps < 500` closes after five hundred laps, so the chain "
        "is a thousand microsteps long and then stops by itself"
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
    assert str(engine.current_state) == "bounded_a", (
        f"the chain rests where its guard closed; it is in {engine.current_state!s}"
    )


def test_a_second_truncated_macrostep_counts_again() -> None:
    """A count, not a flag: a second unbounded macrostep is refused the same
    way the first was."""
    engine, se = _started()

    engine.send_event(_Event.SPIN)
    assert engine.truncated_macrosteps() == 1, (
        "precondition: this test is about the SECOND refusal"
    )

    # `reset` is the fixture's way back out of the cycle, and it moves the
    # machine on purpose: the two C++ engines complete a macrostep only after a
    # transition that does.
    engine.send_event(_Event.RESET)
    assert str(engine.current_state) == "idle"

    engine.send_event(_Event.SPIN)

    assert engine.truncated_macrosteps() == 2, (
        "the second macrostep hit the same ceiling and was counted again"
    )
    assert _counter(engine, se, "spins") == 2 * LAPS_AT_CEILING, (
        "and it really bought the document a full budget again rather than "
        "refusing on sight — the ceiling bounds a macrostep, it does not "
        "condemn a machine"
    )


def test_an_ordinary_macrostep_is_not_counted() -> None:
    """The control: an ordinary document is untouched by any of this. Without
    it, an engine that refused every macrostep would pass the assertions above
    and fail nothing."""
    engine, se = _started()

    engine.send_event(_Event.POKE)

    assert _counter(engine, se, "pokes") == 1, "the run fired"
    assert engine.truncated_macrosteps() == 0, (
        "a macrostep of one microstep ends the way the clause says it does"
    )
    assert engine.last_truncated_macrostep_state() is None
    assert str(engine.current_state) == "idle"
