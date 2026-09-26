# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML 5.7: a bad donedata pair is ignored, the done events arrive — Python AOT.

Measured 2026-09-27, this channel already raised both done events, but sent
done.state.<parent> with no data at all once an empty `location` was met,
dropping the pairs that had evaluated.

Fixture: ``integration_resources/a_bad_donedata_pair_is_ignored/a_bad_donedata_pair_is_ignored.scxml``.

Regeneration (after fixture or template edit):
  ``scripts/regen_a_bad_donedata_pair_is_ignored_python.sh`` (local)
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import a_bad_donedata_pair_is_ignored_sm as _sm  # noqa: E402 — path inserted above

_State = _sm.ABadDonedataPairIsIgnoredState


def test_the_bad_pairs_are_dropped_and_the_done_events_still_arrive() -> None:
    engine = _sm.create_engine()
    # The run needs nothing from the host.
    engine.initialize()

    p = engine.policy
    seen = f"errors={p.errors()} shape={p.shape()} (wanted 2 / 1)"
    assert engine.terminal_state == _State.DONE, (
        f"done.state.p must still arrive and carry the run to `done`. {seen}"
    )
    assert p.errors() == 2, f"each ignored pair raises its own error.execution. {seen}"
    assert p.shape() == 1, (
        f"done.state.r1 must carry the surviving pair and neither ignored one. {seen}"
    )
