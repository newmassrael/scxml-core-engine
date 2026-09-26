# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML Appendix D exitStates: onexit runs before the state leaves — Python AOT.

The procedure is onexit, then cancelInvoke, then configuration.delete(s), for
each state in exitOrder. So ``In(s)`` inside s's own ``<onexit>`` is true, the
parent is still active inside the child's ``<onexit>``, and the child is already
gone inside the parent's. The configuration after the microstep is the same
whatever order an engine used, so the handlers record what they saw and those
records are the verdict.

Fixture: ``integration_resources/onexit_runs_before_the_state_leaves/onexit_runs_before_the_state_leaves.scxml``.

Regeneration (after fixture or template edit):
  ``scripts/regen_onexit_runs_before_the_state_leaves_python.sh`` (local)
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import onexit_runs_before_the_state_leaves_sm as _sm  # noqa: E402 — path inserted above

_State = _sm.OnexitRunsBeforeTheStateLeavesState
_Event = _sm.OnexitRunsBeforeTheStateLeavesEvent


def test_onexit_runs_before_the_state_leaves_aot() -> None:
    engine = _sm.create_engine()
    engine.initialize()

    entry = engine.active_configuration()
    assert _State.INNER in entry, f"the run has to start inside `inner`; it came up as {entry}"

    engine.send_event(_Event.LEAVE)

    settled = engine.terminal_state
    # What the handlers recorded (W3C SCXML 5.3 readers): the final says which
    # clause broke, these say what the handler actually saw.
    p = engine.policy
    records = (
        f"selfInInner={p.self_in_inner()} parentInInner={p.parent_in_inner()} "
        f"selfInOuter={p.self_in_outer()} childInOuter={p.child_in_outer()} "
        f"exits={p.exits()}; wanted 1 / 1 / 1 / 0 / 2"
    )
    assert settled == _State.SETTLED, (
        f"`leave` did not carry the machine to `settled` (ended in: {settled}; {records}). The document "
        "checks its clauses in document order and lands each in a `<final>` of its own: "
        "`failExits` (a handler did not run), `failSelfInInner` / `failSelfInOuter` (a state "
        "was already out of the configuration during its own `<onexit>`), `failParentInInner` "
        "(the parent left before its child's `<onexit>`), `failChildInOuter` (the child was "
        "still active during its parent's `<onexit>`)"
    )
