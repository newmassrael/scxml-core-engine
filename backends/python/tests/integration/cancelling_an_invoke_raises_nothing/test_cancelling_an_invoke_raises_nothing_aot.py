# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML 6.4: cancelling an invocation raises no event — Python AOT.

Measured 2026-09-26, this channel already raised nothing; the fixture pins it.

Fixture: ``integration_resources/cancelling_an_invoke_raises_nothing/cancelling_an_invoke_raises_nothing.scxml``.

Regeneration (after fixture or template edit):
  ``scripts/regen_cancelling_an_invoke_raises_nothing_python.sh`` (local)
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import cancelling_an_invoke_raises_nothing_sm as _sm  # noqa: E402 — path inserted above

_State = _sm.CancellingAnInvokeRaisesNothingState
_Event = _sm.CancellingAnInvokeRaisesNothingEvent


def test_leaving_the_invoking_state_raises_no_cancel_event() -> None:
    engine = _sm.create_engine()
    engine.initialize()
    assert _State.P in engine.active_configuration(), "the run has to start in `p`, with its child invoked"

    for event in (_Event.LEAVE, _Event.FINISH):
        engine.send_event(event)

    assert engine.terminal_state == _State.DONE, "`finish` must carry the run to `done`"
    assert engine.policy.spurious() == 0, (
        "leaving `p` cancelled its child, and a `cancel.invoke` event reached the invoking session "
        f"(spurious={engine.policy.spurious()}); W3C SCXML 6.4 defines no such event"
    )
