# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML 3.4: a self-transitioned region answers the next event — Python AOT.

Its sibling ``parallel_regions_take_own_transitions`` owns the microstep axis.
This one owns what that axis cannot reach: a region can take its transition and
run its content exactly as required and still be left holding no leaf, and the
only thing that tells you so is a later event it fails to answer.

Measured 2026-08-14 on the C++ AOT channel, where the defect lived: with the
mutation ``parallel_microstep_owns_exit_and_entry.cases`` restoring it, the
sibling fixture's driver stayed green and this fixture's went red.

Fixture: ``integration_resources/parallel_self_transition_keeps_its_leaf/parallel_self_transition_keeps_its_leaf.scxml``.

Regeneration (after fixture or template edit):
  ``scripts/regen_parallel_self_transition_keeps_its_leaf_python.sh`` (local)
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import parallel_self_transition_keeps_its_leaf_sm as _sm  # noqa: E402 — path inserted above

_State = _sm.ParallelSelfTransitionKeepsItsLeafState
_Event = _sm.ParallelSelfTransitionKeepsItsLeafEvent


def test_parallel_self_transition_keeps_its_leaf_aot() -> None:
    engine = _sm.create_engine()
    engine.initialize()

    entry = engine.active_configuration()
    assert _State.WITHIN in entry and _State.WORKING in entry, (
        f"fixture came up as {entry}; it is supposed to start with the self-transitioning "
        "region in `within` and the deeper one in `working`, so nothing below is testing "
        "what it claims"
    )

    engine.send_event(_Event.E)

    # The symptom, named where it happens. A region holding no atomic state is
    # still "in" the parallel by every ancestor test.
    after = engine.active_configuration()
    assert _State.WITHIN in after, (
        f"the self-transitioning region lost its leaf on the first event (active: {after}). "
        "`within` is both the source and the target of its own external self-transition, so "
        "the microstep exits and re-enters it; anything that exits it a second time takes it "
        "back out and does not put it back"
    )
    assert _State.JUDGING in after, (
        f"the deeper region did not take its own transition on `e` (active: {after})"
    )

    # The second event is the one this fixture exists for: `judging` has no `e`
    # transition, so nothing but the self-transitioning region can answer, and
    # it can only answer from a leaf.
    engine.send_event(_Event.E)

    twice = engine.active_configuration()
    assert _State.WITHIN in twice, (
        f"the self-transitioning region is not in `within` after the second `e` (active: {twice})"
    )

    engine.send_event(_Event.CHECK)

    settled = engine.active_configuration()
    assert _State.SETTLED in settled, (
        f"`check` did not carry the machine to `settled` (active: {settled}), which the "
        "document guards on `n == 1 && m == 2`. `m` reaches 2 only if the self-transitioning "
        "region still had a leaf to transition from when the second `e` arrived"
    )
