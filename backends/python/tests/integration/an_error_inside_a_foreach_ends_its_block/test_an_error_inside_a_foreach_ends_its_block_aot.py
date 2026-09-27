# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML 4.9 + 4.6: an error inside a <foreach> ends its block — Python AOT.

Fixture: ``integration_resources/an_error_inside_a_foreach_ends_its_block/an_error_inside_a_foreach_ends_its_block.scxml``.

Regeneration (after fixture or template edit):
  ``scripts/regen_an_error_inside_a_foreach_ends_its_block_python.sh`` (local)
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import an_error_inside_a_foreach_ends_its_block_sm as _sm  # noqa: E402 — path inserted above

_State = _sm.AnErrorInsideAForeachEndsItsBlockState
_Event = _sm.AnErrorInsideAForeachEndsItsBlockEvent


def test_the_error_ends_the_block_and_is_the_only_one() -> None:
    engine = _sm.create_engine()
    engine.initialize()
    for event in (_Event.GO, _Event.T, _Event.FINISH):
        engine.send_event(event)

    p = engine.policy
    assert engine.terminal_state == _State.DONE, "`finish` must carry the run to `done`"
    observed = {
        "iters1": (p.iters1(), 1),
        "after1": (p.after1(), 0),
        "iters2": (p.iters2(), 1),
        "after2": (p.after2(), 0),
        "iters3": (p.iters3(), 1),
        "after3": (p.after3(), 0),
        "errors": (p.errors(), 3),
        "sent": (p.sent(), 2),
    }
    wrong = {name: got for name, (got, want) in observed.items() if got != want}
    assert not wrong, f"observed {wrong}, want { {n: w for n, (_, w) in observed.items()} }"
