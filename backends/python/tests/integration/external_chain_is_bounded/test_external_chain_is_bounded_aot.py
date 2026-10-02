# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# A machine that answers an event by sending itself the next one never lets the
# external queue empty, and every macrostep of it ends, so the microstep ceiling
# never applies. The engine takes at most `max_external_events_per_call` events
# in one main-event-loop invocation, leaves the rest queued and counts the
# refusal (`truncated_event_chains`, `last_truncated_event`), as ARCHITECTURE.md
# "External-Event Budget" says. Python AOT path, the first engine to hold it.
#
# Measured 2026-10-02 before the budget existed: such a design did not return
# from `send_event`; the authoring driver's processor-time limit stopped it
# after 25.5 s and called it the machine's doing.
#
# Fixture: tests/integration/external_chain_is_bounded.scxml
#
# Regeneration (after fixture or template edit):
#   scripts/regen_external_chain_is_bounded_python.sh

from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import external_chain_is_bounded_sm as _sm  # noqa: E402 — path inserted above
from sce_runtime.datamodel_read import read_int  # noqa: E402
from sce_runtime.scripting import LuaScriptEngine  # noqa: E402

_Event = _sm.ExternalChainIsBoundedEvent

# The budget the engine applies when a host does not choose one, spelled here
# rather than read back from it. A test that asked the engine for its own limit
# would agree with any limit, including one an edit moved by three orders of
# magnitude.
DEFAULT_BUDGET = 10_000


def _started():
    script_engine = LuaScriptEngine()
    script_engine.initialize()
    engine = _sm.create_engine(script_engine=script_engine)
    engine.initialize()
    return engine, script_engine


def _counter(engine, script_engine, name: str) -> int:
    """The fixture's `<assign>`s are the only witness of how far a chain got:
    every outcome leaves the machine in a state the configuration alone cannot
    tell apart from the others."""
    value = read_int(script_engine, engine.policy._session_id, name)
    assert value is not None, f"the fixture declares `{name}` in its datamodel"
    return value


def test_the_default_budget_is_the_documented_one() -> None:
    engine, _ = _started()
    assert engine.max_external_events_per_call() == DEFAULT_BUDGET
    assert engine.truncated_event_chains() == 0
    assert engine.last_truncated_event() is None


def test_a_chain_that_cannot_end_is_cut_at_the_budget_and_the_call_returns() -> None:
    """This test returning at all is half the assertion: before the budget the
    call did not."""
    engine, se = _started()

    engine.send_event(_Event.SPIN)

    assert engine.truncated_event_chains() == 1, (
        "the call handed control back with an event still queued, and said so; "
        "without the count the host sees a machine that is running and has "
        "returned, with no sign that anything went wrong"
    )
    # The host's own event is the first of the call, so the budget buys the
    # host's event and then `budget - 1` links.
    assert _counter(engine, se, "links") == DEFAULT_BUDGET - 1, (
        "the chain must run exactly as far as the budget allows: fewer means "
        "the call was cut early, more means the budget moved"
    )
    assert str(engine.last_truncated_event()) is not None
    assert engine.policy.get_event_name(engine.last_truncated_event()) == "link", (
        "the count says a call did not reach quiet; this says what it was still "
        "taking"
    )
    assert engine.is_running, (
        "the chain was cut, not the machine: the document is legal, and refusing "
        "to run it forever is the engine's decision to report, not a reason to "
        "stop a machine whose other states still work"
    )


def test_the_budget_is_exact_for_a_chain_that_ends_by_itself() -> None:
    """The half that makes the count mean something: a chain that ends on its
    own is not refused, however close to the budget it comes. `bounded` is the
    host's event and five `lap`s, six in all."""
    exactly, se = _started()
    exactly.set_max_external_events_per_call(6)
    exactly.send_event(_Event.BOUNDED)
    assert _counter(exactly, se, "laps") == 5
    assert exactly.truncated_event_chains() == 0, (
        "a call that takes exactly the budget and empties the queue refused "
        "nothing: a long chain is not a runaway"
    )
    assert exactly.last_truncated_event() is None

    one_short, se = _started()
    one_short.set_max_external_events_per_call(5)
    one_short.send_event(_Event.BOUNDED)
    assert _counter(one_short, se, "laps") == 4, "one `lap` was left queued"
    assert one_short.truncated_event_chains() == 1
    assert one_short.policy.get_event_name(one_short.last_truncated_event()) == "lap"


def test_a_refused_call_leaves_the_queue_so_the_next_call_finishes_the_chain() -> None:
    """What the refusal did with the events it would not take: it left them
    queued. An engine that dropped the queue stops short and never finishes; one
    that ran the chain anyway finishes it in the first call."""
    engine, se = _started()
    engine.set_max_external_events_per_call(20)

    engine.send_event(_Event.RESUME)
    assert engine.truncated_event_chains() == 1
    assert _counter(engine, se, "beats") == 19, "the host's event and nineteen beats"

    engine.send_event(_Event.POKE)
    assert _counter(engine, se, "beats") == 30, (
        "the second call took the beats the first left on the queue, each in a "
        "budget of its own, and finished"
    )
    assert _counter(engine, se, "pokes") == 1, "and the host's second event was heard"
    assert engine.truncated_event_chains() == 1, (
        "the second call ended the way the clause says: nothing more is counted"
    )


def test_a_chain_through_a_delay_of_zero_does_not_keep_the_call_from_returning() -> None:
    """`delay="0ms"` is due at the instant being processed. Whether an engine
    delivers it straight to the queue or through its scheduler is its own, and
    what is held is that the host's call returns and says it was cut. How many
    blinks it handled is not asserted."""
    engine, se = _started()
    engine.set_max_external_events_per_call(50)

    engine.send_event(_Event.ZERO)

    assert engine.truncated_event_chains() >= 1
    assert _counter(engine, se, "blinks") >= 1
    assert engine.is_running


def test_a_chain_through_a_delay_expression_that_is_zero_does_not_keep_the_call_returning() -> None:
    """The same chain through `delayexpr="'0ms'"`, which has no static value an
    engine could read as undelayed. This runtime sends a delay of zero straight
    to the external queue whichever way it was written, so the invocation bound
    holds it; an engine that schedules it has to hold it with the same-instant
    bound (ARCHITECTURE.md "External-Event Budget", rule 5). What is held is the
    same: the host's call returns, and the cut is counted."""
    engine, se = _started()
    engine.set_max_external_events_per_call(50)

    engine.send_event(_Event.ZERO_EXPR)

    assert engine.truncated_event_chains() >= 1
    assert _counter(engine, se, "exprs") >= 1
    assert engine.is_running


def test_a_chain_that_is_finite_because_the_clock_is_is_not_refused() -> None:
    """Eight pulses, each due one millisecond after the last. They are due at
    later instants, so a legitimate time-driven workload is not a runaway
    however small the budget: three here, against eight events."""
    engine, se = _started()
    engine.set_max_external_events_per_call(3)

    engine.send_event(_Event.TIMED)
    for _ in range(8):
        engine.advance_time(1)

    assert _counter(engine, se, "pulses") == 8
    assert engine.truncated_event_chains() == 0, (
        "each pulse came in a call of its own, at an instant of its own"
    )

    jumped, js = _started()
    jumped.set_max_external_events_per_call(3)
    jumped.send_event(_Event.TIMED)
    jumped.advance_time(8)
    assert _counter(jumped, js, "pulses") == 8
    assert jumped.truncated_event_chains() == 0, (
        "eight due entries in one host call are eight loop invocations of one "
        "event each: the budget is per invocation of the drain, and a clock that "
        "moved a long way is not a chain that does not end"
    )


def test_a_host_chooses_the_budget_and_a_budget_that_takes_no_event_is_refused() -> None:
    engine, _ = _started()
    engine.set_max_external_events_per_call(7)
    assert engine.max_external_events_per_call() == 7
    for refused in (0, -1, 2.5, True, "5"):
        try:
            engine.set_max_external_events_per_call(refused)  # type: ignore[arg-type]
        except ValueError as error:
            assert "at least one" in str(error)
        else:  # pragma: no cover - the failure being asserted against
            raise AssertionError(f"a budget of {refused!r} was accepted")
    assert engine.max_external_events_per_call() == 7, "a refused budget changes nothing"
