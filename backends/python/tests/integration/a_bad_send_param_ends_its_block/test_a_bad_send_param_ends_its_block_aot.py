# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML 5.7.1 + 4.9: a bad <send> param is dropped and ends its block — Python AOT.

Measured 2026-09-27, this channel let the rest of the block run and never read
a `location`: a valid one arrived as null.

Fixture: ``integration_resources/a_bad_send_param_ends_its_block/a_bad_send_param_ends_its_block.scxml``.

Regeneration (after fixture or template edit):
  ``scripts/regen_a_bad_send_param_ends_its_block_python.sh`` (local)
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import a_bad_send_param_ends_its_block_sm as _sm  # noqa: E402 — path inserted above

_State = _sm.ABadSendParamEndsItsBlockState
_Event = _sm.ABadSendParamEndsItsBlockEvent


def test_the_message_goes_and_the_block_stops() -> None:
    engine = _sm.create_engine()
    engine.initialize()
    engine.send_event(_Event.FINISH)

    p = engine.policy
    seen = (
        f"errors={p.errors()} partials={p.partials()} bares={p.bares()} "
        f"after={p.after()} carried={p.carried()} (wanted 3 / 2 / 1 / 0 / 1)"
    )
    assert engine.terminal_state == _State.DONE, f"`finish` must carry the run to `done`. {seen}"
    assert p.errors() == 3, f"each unreadable <param> raises one error.execution. {seen}"
    assert p.partials() == 2, (
        f"both internal sends go, carrying the good pair without the bad one. {seen}"
    )
    assert p.bares() == 1, f"the external send goes with its empty pair left out. {seen}"
    assert p.after() == 0, (
        f"the <param> error ends the block, so nothing after the <send> runs. {seen}"
    )
    assert p.carried() == 1, f"a valid location param is sent with its value. {seen}"
