# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML 5.10 + 6.2: a <send>'s payload rides on its own event — Python AOT.

Fixture: ``integration_resources/a_payload_rides_on_its_own_event/a_payload_rides_on_its_own_event.scxml``.

Regeneration (after fixture or template edit):
  ``scripts/regen_a_payload_rides_on_its_own_event_python.sh`` (local)
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import a_payload_rides_on_its_own_event_sm as _sm  # noqa: E402 — path inserted above

_State = _sm.APayloadRidesOnItsOwnEventState
_Event = _sm.APayloadRidesOnItsOwnEventEvent


def test_each_payload_arrives_on_its_own_event() -> None:
    engine = _sm.create_engine()
    engine.initialize()
    engine.send_event(_Event.FINISH)

    p = engine.policy
    seen = f"got={p.got()} stolen={p.stolen()} plains={p.plains()} (wanted 4 / 0 / 4)"
    assert engine.terminal_state == _State.DONE, f"`finish` must carry the run to `done`. {seen}"
    assert p.got() == 4, f"each payload event arrives carrying its own payload. {seen}"
    assert p.stolen() == 0, f"no data-less event arrives carrying a payload. {seen}"
    assert p.plains() == 4, f"every data-less event arrives, and arrives empty. {seen}"
