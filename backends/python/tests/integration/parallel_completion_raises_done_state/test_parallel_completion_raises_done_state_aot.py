# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML 3.4 + 3.7: a ``<parallel>`` completing raises ``done.state.<id>`` — Python AOT.

A ``<parallel>`` owns no ``<final>`` of its own; its finals sit one level down,
inside the regions. A rule that registers the completion event by walking from
a ``<final>`` to its direct parent therefore never reaches the parallel, while
an emitter that raises it from the grandparent does — which is how the C++ and
C11 channels ended up naming an enumerator nothing had declared.

This channel is asked the behavioural half of the same question: both regions
reaching their ``<final>`` on one event, in one microstep.

Fixture: ``integration_resources/parallel_completion_raises_done_state/parallel_completion_raises_done_state.scxml``.

Regeneration (after fixture or template edit):
  ``scripts/regen_parallel_completion_raises_done_state_python.sh`` (local)
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import parallel_completion_raises_done_state_sm as _sm  # noqa: E402 — path inserted above

_State = _sm.ParallelCompletionRaisesDoneStateState
_Event = _sm.ParallelCompletionRaisesDoneStateEvent


def test_parallel_completion_raises_done_state_aot() -> None:
    engine = _sm.create_engine()
    engine.initialize()

    entry = engine.active_configuration()
    assert _State.A1 in entry and _State.B1 in entry, (
        f"fixture came up as {entry}; it is supposed to start with both regions "
        "inside the <parallel>, so nothing below is testing what it claims"
    )

    engine.send_event(_Event.GO)

    after = engine.active_configuration()
    assert _State.A2 in after, f"region `a` did not reach its <final> on `go` (active: {after})"
    assert _State.B2 in after, f"region `b` did not reach its <final> on `go` (active: {after})"
