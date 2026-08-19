# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# W3C SCXML 3.12.2: the processor MUST signal its own failures by raising
# `error.*` events into the internal queue, and the same paragraph says they
# "are ignored if no transition is found that matches them". Being ignored is
# the clause. Being unable to say it happened is not. Python AOT path.
#
# `discarded_event_is_observable` asked this for the EXTERNAL queue and stopped
# at its edge on the stated ground that an unmatched internal event is the
# document's own business with both ends inside the document. That is exactly
# right for an author's `<raise>` and exactly wrong for an error event, whose
# sender is the ENGINE. The host never wrote the document, cannot see the
# failure in the configuration, and is the only party able to act on it.
#
# Four outcomes the fixture separates, all four leaving the configuration on the
# same state:
#
#   poke              handled, no error            control: proves a run fired
#   whisper           author's <raise>, unmatched  NOT counted
#   boom in idle      error, unmatched             COUNTED — the silent failure
#   boom in guarded   error, HANDLED               not counted
#
# `boom` is one event name routed to two outcomes by state, so a count cannot be
# keyed off the event or the action — only off what the configuration did with
# the error the engine raised.
#
# Fixture: integration_resources/unhandled_error_is_observable/unhandled_error_is_observable.scxml
#
# Regeneration (after fixture or template edit):
#   scripts/regen_unhandled_error_is_observable_python.sh

from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import unhandled_error_is_observable_sm as _sm  # noqa: E402 — path inserted above
from sce_runtime.datamodel_read import read_int  # noqa: E402
from sce_runtime.scripting import LuaScriptEngine  # noqa: E402

_Event = _sm.UnhandledErrorIsObservableEvent


def _started():
    """Hold the script engine so the fixture's counters can be read back.

    The `<assign>`s are the only witness that a transition ran at all — every
    outcome this fixture separates leaves the configuration on the same state.
    """
    script_engine = LuaScriptEngine()
    script_engine.initialize()
    engine = _sm.create_engine(script_engine=script_engine)
    engine.initialize()
    return engine, script_engine


def _counter(engine, script_engine, name: str) -> int:
    value = read_int(script_engine, engine.policy._session_id, name)
    assert value is not None, f"the fixture declares `{name}` in its datamodel"
    return value


def test_an_error_no_transition_answered_is_counted() -> None:
    """The axis: an error the engine raised that no active state answers."""
    engine, se = _started()
    assert engine.unhandled_error_events() == 0, (
        "no error has gone unhandled before the first event"
    )

    engine.send_event(_Event.BOOM)

    assert _counter(engine, se, "booms") == 1, (
        "`boom`'s transition did not run, so nothing below is measuring an "
        "error raised inside a transition that fired"
    )
    assert engine.unhandled_error_events() == 1, (
        "`boom`'s second <assign> has W3C 5.3's invalid empty location, so the "
        "engine raised error.execution — and `idle` declares no transition for "
        "it. The host driving this machine has no other way to learn its "
        "<assign> failed"
    )
    assert str(engine.current_state) == "idle", (
        "the error must not move the machine on its own; it is in "
        f"{engine.current_state!s}"
    )


def test_an_error_the_document_handled_is_not_counted() -> None:
    """The other half: an error the DOCUMENT answered must not be counted."""
    engine, se = _started()

    engine.send_event(_Event.GO)
    assert str(engine.current_state) == "guarded", (
        "`go` should have moved the machine to the state that answers errors"
    )

    engine.send_event(_Event.BOOM)

    assert _counter(engine, se, "caught") == 1, (
        "`guarded`'s error.execution transition did not run, so this test is "
        "not measuring a HANDLED error"
    )
    assert engine.unhandled_error_events() == 0, (
        "the same <assign> failed in `guarded`, where the document does declare "
        "a transition for error.execution. The document dealt with it, and its "
        "handling is already visible in the configuration — counting it would "
        "report the author's own error handling as a silent failure"
    )
    assert engine.last_unhandled_error() is None, (
        "nothing went unhandled, so there is no last one to name"
    )


def test_an_authors_unmatched_raise_is_not_an_unhandled_error() -> None:
    """The boundary: an author's own unmatched `<raise>` is not an error."""
    engine, se = _started()

    engine.send_event(_Event.WHISPER)

    assert engine.unhandled_error_events() == 0, (
        "`whisper` raises `unheard` and `retry.error.execution`, neither of "
        "which any state answers. Both are discarded exactly as an unmatched "
        "error is, and neither is one: the author wrote the raises and the "
        "absent handlers. `retry.error.execution` is the sharper half — it "
        "CONTAINS `error.` without starting with it, and W3C 3.12.2 reserves "
        "the prefix, not the substring"
    )
    assert _counter(engine, se, "heards") == 1, (
        "`whisper`'s third raise, `heard`, does match — and the transition it "
        "matches did not run. The count above is a byproduct of the internal "
        "drain, never its job: an implementation that only selects transitions "
        "for error events stops running the document for everything else"
    )
    assert engine.discarded_external_events() == 0, (
        "`whisper` itself was handled, so the external-queue count stays put — "
        "the internal events it raised are not on that queue at all"
    )


def test_the_unhandled_error_is_not_derivable_from_any_other_accessor() -> None:
    """Every pre-existing accessor answers the same for both runs."""
    engine, _se = _started()

    engine.send_event(_Event.POKE)
    clean = (
        str(engine.current_state),
        list(engine.active_leaves),
        engine.is_running,
        engine.reached_final,
        engine.discarded_external_events(),
        engine.last_discarded_event(),
    )

    engine.send_event(_Event.BOOM)
    failed = (
        str(engine.current_state),
        list(engine.active_leaves),
        engine.is_running,
        engine.reached_final,
        engine.discarded_external_events(),
        engine.last_discarded_event(),
    )

    assert clean == failed, (
        "this fixture exists because these two are indistinguishable through "
        "every accessor a host had — including layer three's discard count, "
        "which never sees the internal queue. If they ever differ, the fixture "
        "stopped measuring what it claims"
    )
    assert engine.unhandled_error_events() == 1, (
        "the two are indistinguishable through every other accessor, so this "
        "count is the only thing that separates a silent failure from a clean run"
    )


def test_the_engine_names_the_error_it_dropped() -> None:
    """A count says something failed; a repair needs the class of error."""
    engine, _se = _started()
    assert engine.last_unhandled_error() is None, "nothing has gone unhandled yet"

    engine.send_event(_Event.BOOM)

    assert engine.last_unhandled_error() == _Event.ERROR_EXECUTION, (
        "`error.execution` is the document's own executable content failing; "
        "`error.communication` would be a <send> that could not reach its "
        "target. Two different repairs, and a bare count separates neither"
    )


def test_a_machine_failing_every_round_is_counted_every_round() -> None:
    """The supervisor's failure mode: every round fails identically."""
    engine, se = _started()

    for round_no in range(1, 4):
        engine.send_event(_Event.BOOM)
        assert engine.unhandled_error_events() == round_no, (
            f"round {round_no} did not add to the count; a supervisor polling "
            "this number is exactly who learns the loop is not making progress"
        )
        assert str(engine.current_state) == "idle", (
            "the machine looks identical on every round, which is the problem"
        )
    assert _counter(engine, se, "booms") == 3, (
        "all three rounds should have run their transition"
    )
