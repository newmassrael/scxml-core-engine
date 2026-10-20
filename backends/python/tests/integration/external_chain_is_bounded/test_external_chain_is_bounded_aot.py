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
# The numbers (the default budget, each outcome's budget and counters) are read
# from tests/external_event_budget/budget_outcomes.json, the one table every
# engine's test of this contract is held to. They are read from that file and
# not from the engine: a test that asked the engine for its own limit would
# agree with any limit, including one an edit moved by three orders of
# magnitude.
#
# Fixture: tests/integration/external_chain_is_bounded.scxml
#
# Regeneration (after fixture or template edit):
#   scripts/regen_external_chain_is_bounded_python.sh

from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any, Dict

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import external_chain_is_bounded_sm as _sm  # noqa: E402 — path inserted above
from sce_runtime.datamodel_read import read_int  # noqa: E402
from sce_runtime.scripting import LuaScriptEngine  # noqa: E402

_Event = _sm.ExternalChainIsBoundedEvent

# The repository root, from this file's own location: the shared table is named
# by the same path every other reader uses.
_REPO_ROOT = _HERE.parents[4]
_TABLE_PATH = _REPO_ROOT / "tests" / "external_event_budget" / "budget_outcomes.json"


def _load_table() -> Dict[str, Any]:
    assert _TABLE_PATH.is_file(), f"cannot read the shared table at {_TABLE_PATH}"
    return json.loads(_TABLE_PATH.read_text(encoding="utf-8"))


_TABLE = _load_table()


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
    assert engine.max_external_events_per_call() == _TABLE["default_budget"]
    assert engine.truncated_event_chains() == 0
    assert engine.last_truncated_event() is None


def test_a_chain_that_cannot_end_is_cut_at_the_budget_and_the_call_returns() -> None:
    """This test returning at all is half the assertion: before the budget the
    call did not."""
    engine, se = _started()

    engine.send_event(_Event.SPIN)

    assert engine.truncated_event_chains() == _TABLE["spin"]["truncated"], (
        "the call handed control back with an event still queued, and said so; "
        "without the count the host sees a machine that is running and has "
        "returned, with no sign that anything went wrong"
    )
    # The host's own event is the first of the call, so the budget buys the
    # host's event and then `budget - 1` links.
    assert _counter(engine, se, "links") == _TABLE["default_budget"] - 1, (
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
    exactly.set_max_external_events_per_call(_TABLE["exact"]["budget"])
    exactly.send_event(_Event.BOUNDED)
    assert _counter(exactly, se, "laps") == _TABLE["exact"]["laps"]
    assert exactly.truncated_event_chains() == _TABLE["exact"]["truncated"], (
        "a call that takes exactly the budget and empties the queue refused "
        "nothing: a long chain is not a runaway"
    )
    assert exactly.last_truncated_event() is None

    one_short, se = _started()
    one_short.set_max_external_events_per_call(_TABLE["one_short"]["budget"])
    one_short.send_event(_Event.BOUNDED)
    assert _counter(one_short, se, "laps") == _TABLE["one_short"]["laps"], (
        "one `lap` was left queued"
    )
    assert one_short.truncated_event_chains() == _TABLE["one_short"]["truncated"]
    assert one_short.policy.get_event_name(one_short.last_truncated_event()) == "lap"


def test_a_refused_call_leaves_the_queue_so_the_next_call_finishes_the_chain() -> None:
    """What the refusal did with the events it would not take: it left them
    queued. An engine that dropped the queue stops short and never finishes; one
    that ran the chain anyway finishes it in the first call."""
    engine, se = _started()
    resume = _TABLE["resume"]
    engine.set_max_external_events_per_call(resume["budget"])

    engine.send_event(_Event.RESUME)
    assert engine.truncated_event_chains() == resume["truncated"]
    assert _counter(engine, se, "beats") == resume["beats_after_first_call"], (
        "the host's event and nineteen beats"
    )

    engine.send_event(_Event.POKE)
    assert _counter(engine, se, "beats") == resume["beats_after_second_call"], (
        "the second call took the beats the first left on the queue, each in a "
        "budget of its own, and finished"
    )
    assert _counter(engine, se, "pokes") == resume["pokes"], (
        "and the host's second event was heard"
    )
    assert engine.truncated_event_chains() == resume["truncated"], (
        "the second call ended the way the clause says: nothing more is counted"
    )


def test_a_chain_through_a_delay_of_zero_does_not_keep_the_call_from_returning() -> None:
    """`delay="0ms"` is due at the instant being processed. Whether an engine
    delivers it straight to the queue or through its scheduler is its own, and
    what is held is that the host's call returns and says it was cut. How many
    blinks it handled is not asserted."""
    engine, se = _started()
    engine.set_max_external_events_per_call(_TABLE["static_zero"]["budget"])

    engine.send_event(_Event.ZERO)

    assert engine.truncated_event_chains() >= _TABLE["static_zero"]["truncated"]
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
    engine.set_max_external_events_per_call(_TABLE["zero_expr"]["budget"])

    engine.send_event(_Event.ZERO_EXPR)

    assert engine.truncated_event_chains() >= 1
    assert _counter(engine, se, "exprs") >= 1
    assert engine.is_running


def test_a_chain_that_is_finite_because_the_clock_is_is_not_refused() -> None:
    """Eight pulses, each due one millisecond after the last. They are due at
    later instants, so a legitimate time-driven workload is not a runaway
    however small the budget: three here, against eight events."""
    timed = _TABLE["timed"]
    engine, se = _started()
    engine.set_max_external_events_per_call(timed["budget"])

    engine.send_event(_Event.TIMED)
    for _ in range(timed["pulses"]):
        engine.advance_time(timed["interval_ms"])

    assert _counter(engine, se, "pulses") == timed["pulses"]
    assert engine.truncated_event_chains() == timed["truncated"], (
        "each pulse came in a call of its own, at an instant of its own"
    )

    jumped, js = _started()
    jumped.set_max_external_events_per_call(timed["budget"])
    jumped.send_event(_Event.TIMED)
    # Each pulse is due the table's interval after the last, so one jump over
    # all of them is that many milliseconds for each pulse.
    jumped.advance_time(timed["pulses"] * timed["interval_ms"])
    assert _counter(jumped, js, "pulses") == timed["pulses"]
    assert jumped.truncated_event_chains() == timed["truncated"], (
        "eight due entries in one host call are eight loop invocations of one "
        "event each: the budget is per invocation of the drain, and a clock that "
        "moved a long way is not a chain that does not end"
    )


def test_a_host_chooses_the_budget_and_a_budget_that_takes_no_event_is_refused() -> None:
    host_budget = _TABLE["host"]["budget"]
    engine, _ = _started()
    engine.set_max_external_events_per_call(host_budget)
    assert engine.max_external_events_per_call() == host_budget
    # The table's refused numbers, then the values that are not a whole number
    # of events at all: a type this driver's language can be handed and Rust's
    # and Go's typed signatures cannot.
    for refused in (*_TABLE["host"]["refused"], 2.5, True, "5"):
        try:
            engine.set_max_external_events_per_call(refused)  # type: ignore[arg-type]
        except ValueError as error:
            assert "at least one" in str(error)
        else:  # pragma: no cover - the failure being asserted against
            raise AssertionError(f"a budget of {refused!r} was accepted")
    assert engine.max_external_events_per_call() == host_budget, (
        "a refused budget changes nothing"
    )
