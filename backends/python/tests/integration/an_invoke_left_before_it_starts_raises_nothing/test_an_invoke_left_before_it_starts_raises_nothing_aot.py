# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML 6.4: an invoke its state left is never attempted — Python AOT.

Measured 2026-09-26, this channel already raised nothing; the fixture pins it.

Fixture: ``integration_resources/an_invoke_left_before_it_starts_raises_nothing/an_invoke_left_before_it_starts_raises_nothing.scxml``.

Regeneration (after fixture or template edit):
  ``scripts/regen_an_invoke_left_before_it_starts_raises_nothing_python.sh`` (local)
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import an_invoke_left_before_it_starts_raises_nothing_sm as _sm  # noqa: E402 — path inserted above

_State = _sm.AnInvokeLeftBeforeItStartsRaisesNothingState
_Event = _sm.AnInvokeLeftBeforeItStartsRaisesNothingEvent


def test_the_left_states_invoke_is_never_attempted() -> None:
    engine = _sm.create_engine()
    engine.initialize()
    assert _State.S1 in engine.active_configuration(), (
        "`s0` leaves on an eventless transition within the first macrostep"
    )

    engine.send_event(_Event.FINISH)

    assert engine.terminal_state == _State.DONE, "`finish` must carry the run to `done`"
    assert engine.policy.errors() == 0, (
        "`s0` left before its macrostep ended, yet its invoke was attempted and raised error.execution "
        f"(errors={engine.policy.errors()}); W3C SCXML 6.4 cancels an invoke whose state has left"
    )
