# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# W3C SCXML Appendix D's main event loop returns to
# `selectEventlessTransitions()` after every microstep, and drains the internal
# queue in the same inner loop. It never asks whether the microstep it just
# took moved the machine — it cannot, because W3C SCXML 3.13 defines a
# transition with no `target` as one that exits and enters nothing and runs its
# content in place. Python AOT path.
#
# Measured 2026-08-20, the two C++ engines end the macrostep at such a
# transition: whatever its content enabled is never walked, and the host is
# handed a configuration the clause says is not stable. This channel is the
# side of that comparison that was already right, and it is here so the
# contract is stated for every backend rather than only for the ones that broke
# it.
#
# `eventless_macrostep_is_bounded` owns how FAR a chain may run; this one owns
# whether the chain is entered at all.
#
# Fixture: integration_resources/targetless_transition_completes_macrostep/targetless_transition_completes_macrostep.scxml
#
# Regeneration (after fixture or template edit):
#   scripts/regen_targetless_transition_completes_macrostep_python.sh

from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import targetless_transition_completes_macrostep_sm as _sm  # noqa: E402 — path inserted above
from sce_runtime.datamodel_read import read_int  # noqa: E402
from sce_runtime.scripting import LuaScriptEngine  # noqa: E402

_Event = _sm.TargetlessTransitionCompletesMacrostepEvent


def _started():
    script_engine = LuaScriptEngine()
    script_engine.initialize()
    engine = _sm.create_engine(script_engine=script_engine)
    engine.initialize()
    return engine, script_engine


def _counter(engine, script_engine, name: str) -> int:
    """The fixture's `<assign>`s are the only witness of how far the macrostep
    got: every outcome here leaves the machine in a state the configuration
    alone cannot tell apart from a macrostep that stopped one microstep
    early."""
    value = read_int(script_engine, engine.policy._session_id, name)
    assert value is not None, f"the fixture declares `{name}` in its datamodel"
    return value


def test_a_targetless_transition_does_not_end_the_macrostep() -> None:
    """The axis: a transition that moves nothing still ends a microstep, so
    the macrostep continues into whatever its content enabled.

    `chained == 1, polished == 0` is the signature of an engine that resumes
    the chain only after a transition that MOVED the machine: it takes the link
    that moves and stops before the link that does not. `chained == 0` is the
    signature of one that never entered the chain at all."""
    engine, se = _started()

    engine.send_event(_Event.ARM)

    assert _counter(engine, se, "armed") == 1, (
        "the targetless transition ran its content — without this the rest "
        "measures a lost event rather than a stopped macrostep"
    )
    assert _counter(engine, se, "chained") == 1, (
        "and the eventless transition that content enabled was taken in the "
        "SAME macrostep, which is the whole of what Appendix D's inner loop "
        "promises a host"
    )
    assert _counter(engine, se, "polished") == 1, (
        "including the chain's last link, which is targetless itself: an "
        "engine that walks the chain only while the machine keeps moving "
        "stops exactly here"
    )
    assert str(engine.current_state) == "settled", (
        "and the host is handed the stable configuration, not the one the "
        f"machine was passing through; it is in {engine.current_state!s}"
    )


def test_a_raise_from_a_targetless_transition_is_answered_in_the_same_macrostep() -> None:
    """The other side of the same inner loop: what a targetless transition
    raises is answered before the host gets control back."""
    engine, se = _started()

    engine.send_event(_Event.PING)

    assert _counter(engine, se, "answered") == 1, (
        "the internal event the targetless transition raised was dequeued and "
        "matched inside this macrostep"
    )
    assert str(engine.current_state) == "idle", (
        "neither transition moves the machine, which is the point: the "
        "macrostep has to continue anyway"
    )


def test_a_targetless_transition_that_enables_nothing_changes_nothing_else() -> None:
    """The control, and the reason a zero above means anything: a targetless
    transition that enables nothing leaves the machine exactly where it was,
    and having run is still observable."""
    engine, se = _started()

    engine.send_event(_Event.QUIET)

    assert _counter(engine, se, "quiet") == 1, "the transition fired"
    assert _counter(engine, se, "chained") == 0, (
        "and nothing else did: the eventless transition's guard is still "
        "closed, so an engine that walked the chain here would be firing a "
        "transition the document did not enable"
    )
    assert _counter(engine, se, "polished") == 0
    assert _counter(engine, se, "answered") == 0
    assert str(engine.current_state) == "idle"
    assert engine.is_running


def test_an_eventless_self_transition_exits_and_re_enters() -> None:
    """The other microstep that ends where it began: a transition whose target
    is its own source.

    It is not targetless — W3C SCXML 3.13 gives it an exit and an entry — but a
    macrostep loop that continues only while the configuration keeps changing
    drops it for the same reason and, in the C++ AOT engine, in the same line
    of code. `entries == 1` is that engine: the transition was selected,
    nothing ran, and the chain ended."""
    engine, se = _started()

    engine.send_event(_Event.RECYCLE)

    assert _counter(engine, se, "entries") == 2, (
        "the state is entered once by `recycle` and once more by the eventless "
        "self transition its entry enabled — a self transition exits and "
        "re-enters, so `<onentry>` runs again"
    )
    assert str(engine.current_state) == "recycled", (
        "and the guard closes behind it, so the machine rests here rather than "
        f"spinning; it is in {engine.current_state!s}"
    )


def test_the_second_targetless_transition_is_followed_too() -> None:
    """A macrostep, not a one-shot: the second targetless transition is
    followed the same way the first was."""
    engine, se = _started()

    engine.send_event(_Event.QUIET)
    engine.send_event(_Event.PING)
    assert _counter(engine, se, "answered") == 1, (
        "precondition: this test is about the SECOND raise"
    )

    engine.send_event(_Event.PING)

    assert _counter(engine, se, "answered") == 2, (
        "the raise in the third macrostep was answered like the one in the "
        "second — the inner loop belongs to every macrostep, not to the first"
    )
    assert _counter(engine, se, "quiet") == 1
    assert str(engine.current_state) == "idle"
