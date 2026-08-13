# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML 3.4: every region of a ``<parallel>`` takes its own transition — Python AOT.

The fixture is asymmetric on purpose. One region's transition on the event is
an external self-transition, whose domain Appendix D resolves through
``findLCCA`` over the proper ancestors — candidates that never include the
state itself. Answering with the state left the exit-set walk without a
stopping point, so it ran to the document root, the exit set named the
enclosing ``<parallel>``, and conflict resolution preempted the deeper region's
transition on that same event.

The observable is ``settled``, which the document reaches only when both
regions' assignments have run — a configuration check alone would still pass
for a region that moved without executing its transition content.

Fixture: ``integration_resources/parallel_regions_take_own_transitions/parallel_regions_take_own_transitions.scxml``.

Regeneration (after fixture or template edit):
  ``scripts/regen_parallel_regions_take_own_transitions_python.sh`` (local)
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import parallel_regions_take_own_transitions_sm as _sm  # noqa: E402 — path inserted above

_State = _sm.ParallelRegionsTakeOwnTransitionsState
_Event = _sm.ParallelRegionsTakeOwnTransitionsEvent


def test_parallel_regions_take_own_transitions_aot() -> None:
    engine = _sm.create_engine()
    engine.initialize()

    entry = engine.active_configuration()
    assert _State.WORKING in entry and _State.WITHIN in entry, (
        f"fixture came up as {entry}; it is supposed to start with the deeper region "
        "in `working` and the shallower one in `within`, so nothing below is testing "
        "what it claims"
    )

    engine.send_event(_Event.E)

    after = engine.active_configuration()
    assert _State.JUDGING in after, (
        f"the deeper region lost its leaf (active: {after}). W3C SCXML 3.4 has every "
        "region take its own enabled transition on `e`; the sibling region's external "
        "self-transition must not preempt this one"
    )
    assert _State.WITHIN in after, (
        f"the shallower region left `within`, which is both the source and the target "
        f"of its own external self-transition (active: {after})"
    )

    engine.send_event(_Event.CHECK)

    settled = engine.active_configuration()
    assert _State.SETTLED in settled, (
        f"`check` did not carry the machine to `settled` (active: {settled}), which the "
        "document guards on both regions' assignments having run. Reaching `judging` "
        "without `n == 1 && m == 1` means a region changed state while its transition "
        "content was skipped"
    )
