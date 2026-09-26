# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML Appendix D exitInterpreter: a run ends by exiting every state — Python AOT.

Reached two ways, and both are driven here: the run enters a top-level
``<final>`` after a step, or the host stops it. Either way every remaining
state's ``<onexit>`` runs innermost first and the configuration ends empty;
where the run ended is ``terminal_state``, and the datamodel the handlers wrote
stays readable. Measured 2026-09-26, this channel ran the final's ``<onexit>``
in the middle of the microstep that entered it, kept the final in the
configuration afterwards, and ran no ``<onexit>`` on ``stop()``.

Fixture: ``integration_resources/the_run_ends_by_exiting_every_state/the_run_ends_by_exiting_every_state.scxml``.

Regeneration (after fixture or template edit):
  ``scripts/regen_the_run_ends_by_exiting_every_state_python.sh`` (local)
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import the_run_ends_by_exiting_every_state_sm as _sm  # noqa: E402 — path inserted above

_State = _sm.TheRunEndsByExitingEveryStateState
_Event = _sm.TheRunEndsByExitingEveryStateEvent


def _started():
    engine = _sm.create_engine()
    engine.initialize()
    entry = engine.active_configuration()
    assert _State.INNER in entry, f"the run has to start inside `inner`; it came up as {entry}"
    return engine


def _describe(engine) -> str:
    """What the run left behind (W3C SCXML 5.3 readers), so a failure says what
    happened and not only which clause broke."""
    p = engine.policy
    return (
        f"active: {list(engine.active_configuration())}, ended in {engine.terminal_state}, "
        f"running={engine.is_running}, order={p.order()} finalExits={p.final_exits()} "
        f"selfInFinal={p.self_in_final()}"
    )


def test_a_run_that_reaches_its_final_exits_the_final() -> None:
    engine = _started()
    engine.send_event(_Event.FINISH)

    seen = _describe(engine)
    p = engine.policy
    assert engine.terminal_state == _State.DONE, seen
    assert not engine.is_running, seen
    assert not list(engine.active_configuration()), (
        f"exitInterpreter deletes every state it exits, the final included. {seen}"
    )
    assert p.final_exits() == 1, f"the final's own <onexit> must run exactly once as the run ends. {seen}"
    assert p.self_in_final() == 1, f"`done` must still be in the configuration during its own <onexit>. {seen}"
    assert p.order() == 12, f"`finish` exits `inner` then `outer`. {seen}"


def test_a_stopped_run_exits_every_state_innermost_first() -> None:
    engine = _started()
    engine.stop()

    seen = _describe(engine)
    p = engine.policy
    assert engine.terminal_state is None, f"a stopped run did not end in a final. {seen}"
    assert not engine.is_running, seen
    assert not list(engine.active_configuration()), seen
    assert p.order() == 12, (
        "stop() must run `inner`'s <onexit> and then `outer`'s: 0 is a stop that exited nothing, "
        f"21 one that exited in document order instead of exit order. {seen}"
    )
    assert p.final_exits() == 0, f"the run never entered `done`. {seen}"
