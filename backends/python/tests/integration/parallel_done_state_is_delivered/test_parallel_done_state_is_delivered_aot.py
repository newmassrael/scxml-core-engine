# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML 3.4 + 3.7: ``done.state.<parallel>`` is delivered, not merely declared — Python AOT.

The sibling fixture ``parallel_completion_raises_done_state`` carries no
listener, deliberately: a transition's ``event`` attribute is itself a
registration site, so a listener there would register the completion event no
matter what the ``<final>`` walk does and leave that fixture unable to fail for
the defect it exists to catch. What it proves is that the event is DECLARED.

Declared is not delivered. A backend that names the event and never raises it —
or raises it where nothing selects from — passes there. This document listens,
and ``settled`` is a top-level ``<final>`` no other route reaches.

Fixture: ``integration_resources/parallel_done_state_is_delivered/parallel_done_state_is_delivered.scxml``.

Regeneration (after fixture or template edit):
  ``scripts/regen_parallel_done_state_is_delivered_python.sh`` (local)
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import parallel_done_state_is_delivered_sm as _sm  # noqa: E402 — path inserted above

_State = _sm.ParallelDoneStateIsDeliveredState
_Event = _sm.ParallelDoneStateIsDeliveredEvent


def test_parallel_done_state_is_delivered_aot() -> None:
    engine = _sm.create_engine()
    engine.initialize()

    entry = engine.active_configuration()
    assert _State.A1 in entry and _State.B1 in entry, (
        f"fixture came up as {entry}; it is supposed to start with both regions "
        "inside the <parallel>, so nothing below is testing what it claims"
    )

    engine.send_event(_Event.GO)

    # One assertion, with the configuration in its message, because the two
    # ways this can fail are not separately observable: completion is selected
    # within the SAME macrostep as the regions' finals, so once the event
    # returns the parallel has been exited and A2/B2 are gone. Measured —
    # checking them as a precondition failed against engines that had already
    # done the right thing.
    #
    # The remaining states tell the two apart: A1/B1 means `go` moved nothing;
    # A2/B2 means the parallel completed and the event went nowhere.
    after = engine.active_configuration()
    assert _State.SETTLED in after, (
        "every region reaching its <final> completes the parallel, so done.state.run "
        f"had to be raised AND selected — `settled` is reachable by nothing else (active: {after})"
    )
