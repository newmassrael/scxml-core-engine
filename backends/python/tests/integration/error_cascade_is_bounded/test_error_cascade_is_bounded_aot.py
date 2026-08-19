# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# W3C SCXML 3.12.2 says an error event nothing matches is ignored. It says
# nothing about an error event something DOES match, answered by a handler that
# fails the same way every time: the failure raises `error.execution`, the same
# transition answers it, and the drain never empties. Python AOT path.
#
# This engine is where the cost was measured, on 2026-08-19, with a probe on a
# two-line document: `initialize()` never returned, 37,000 links a second, the
# configuration never moved, `is_running` stayed true. An unattended supervisor
# reads a healthy idle machine and a pinned core.
#
# `unhandled_error_is_observable` owns the error nobody answered; this one owns
# the error answered by a handler that cannot handle it. The fixture separates
# a chain that STOPS by itself (`settle`, three links, then its guard stops
# matching) from one that cannot (`spin`) — both are runs of errors, and only
# the second is a defect.
#
# Fixture: integration_resources/error_cascade_is_bounded/error_cascade_is_bounded.scxml
#
# Regeneration (after fixture or template edit):
#   scripts/regen_error_cascade_is_bounded_python.sh

from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import error_cascade_is_bounded_sm as _sm  # noqa: E402 — path inserted above
from sce_runtime.datamodel_read import read_int  # noqa: E402
from sce_runtime.scripting import LuaScriptEngine  # noqa: E402

_Event = _sm.ErrorCascadeIsBoundedEvent

# The ceiling the engine applies, spelled here rather than read back from it. A
# test that asked the engine for its own limit would agree with any limit,
# including one an edit moved by three orders of magnitude.
MAX_LINKS = 100


def _started():
    script_engine = LuaScriptEngine()
    script_engine.initialize()
    engine = _sm.create_engine(script_engine=script_engine)
    engine.initialize()
    return engine, script_engine


def _counter(engine, script_engine, name: str) -> int:
    """The fixture's `<assign>`s are the only witness that a handler ran — every
    outcome here leaves the configuration where it was."""
    value = read_int(script_engine, engine.policy._session_id, name)
    assert value is not None, f"the fixture declares `{name}` in its datamodel"
    return value


def test_a_handler_that_cannot_handle_its_error_is_stopped() -> None:
    """The axis: a handler answering its own failure with the same failure is
    stopped, and the host is told.

    This test returning at all is half the assertion. Before the ceiling
    existed it did not — this exact engine ran until it was killed."""
    engine, se = _started()
    assert engine.error_cascade_events() == 0, (
        "nothing has been refused before the machine has done anything"
    )

    engine.send_event(_Event.SPIN)

    assert _counter(engine, se, "runs") == MAX_LINKS, (
        "`runaway`'s handler must run exactly as many times as the engine "
        "allows links in a chain — fewer means the document was cut off "
        "early, more means the ceiling moved"
    )
    assert engine.error_cascade_events() == 1, (
        "the handler's <assign> failed again on the last allowed link, and the "
        "error it raised is the one the engine refused to queue. Without that "
        "count the host sees a machine that is running, in a plausible state, "
        "with nothing to say about the core it is burning"
    )
    assert engine.last_error_cascade_event() == _Event.ERROR_EXECUTION, (
        "a count alone does not name the repair: error.execution is a handler "
        "whose own content fails, error.communication one that answers an "
        "unreachable target by talking to it again"
    )
    assert engine.is_running, (
        "the chain was cut, not the machine — refusing to feed a broken "
        "handler is not a reason to stop running a document whose other "
        "states still work"
    )
    assert str(engine.current_state) == "runaway", (
        "the handler is targetless, so nothing here may move the machine; it "
        f"is in {engine.current_state!s}"
    )


def test_a_chain_that_ends_on_its_own_is_not_refused() -> None:
    """The other half, and the one that makes the count mean something."""
    engine, se = _started()

    engine.send_event(_Event.SETTLE)

    assert _counter(engine, se, "repairs") == 3, (
        "`settling`'s handler repairs three times and then its `repairs < 3` "
        "guard stops matching. Three links is what a real repair strategy "
        "looks like, and the engine must not have interrupted it"
    )
    assert engine.error_cascade_events() == 0, (
        "nothing was refused: the chain ended on the document's own terms. A "
        "ceiling that fired here would report every document that fails often "
        "as one that cannot stop failing"
    )
    assert engine.last_error_cascade_event() is None, (
        "nothing was refused, so there is no last one to name"
    )
    assert engine.unhandled_error_events() == 1, (
        "the fourth error found no matching transition once the guard closed, "
        "which is the ordinary clause — the two counts answer different "
        "questions and this document produces exactly one of each"
    )


def test_one_error_nobody_answered_is_not_a_chain() -> None:
    """The chain is measured handler-to-handler, not failure-to-failure."""
    engine, _se = _started()

    for _ in range(5):
        engine.send_event(_Event.BOOM)

    assert engine.unhandled_error_events() == 5, (
        "five failures, none of them answered — the clause's own case"
    )
    assert engine.error_cascade_events() == 0, (
        "no handler ran, so no handler raised anything: a count keyed off how "
        "OFTEN a document fails would already be at five here"
    )


def test_the_machine_still_answers_after_its_chain_is_cut() -> None:
    """Cutting the chain must not cost the document the states that work."""
    engine, se = _started()

    engine.send_event(_Event.SPIN)
    assert engine.error_cascade_events() == 1, (
        "precondition: this test is about what happens AFTER a refusal"
    )

    engine.send_event(_Event.POKE)

    assert _counter(engine, se, "pokes") == 1, (
        "`runaway` answers `poke` with a targetless transition, and it ran — "
        "an engine that stopped the machine to end the chain would leave the "
        "host with a dead document instead of a bounded one"
    )
    assert engine.error_cascade_events() == 1, (
        "`poke` raises nothing, so the count that was already there is all "
        "there is: the refusal is a fact about the past, not a mode"
    )


def test_a_second_chain_starts_from_zero() -> None:
    """The depth is a property of the chain, not of the machine's whole life."""
    engine, se = _started()

    engine.send_event(_Event.SPIN)
    engine.send_event(_Event.RESET)
    assert str(engine.current_state) == "idle", (
        "`reset` is the fixture's way back out of the chain"
    )

    engine.send_event(_Event.SPIN)

    assert _counter(engine, se, "runs") == 2 * MAX_LINKS, (
        "the second entry into `runaway` must buy the document a full chain "
        "again. A depth carried across the drains would stop this one at its "
        f"first link and leave the counter at {MAX_LINKS}"
    )
    assert engine.error_cascade_events() == 2, (
        "two chains, two refusals — a count that saturates at one would read "
        "as a machine that recovered"
    )
