# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML 5.7.1 + 6.4: what each argument of an <invoke> costs — Python AOT.

Fixture: ``integration_resources/a_bad_invoke_argument_is_reported_once/a_bad_invoke_argument_is_reported_once.scxml``.

Regeneration (after fixture or template edit):
  ``scripts/regen_a_bad_invoke_argument_is_reported_once_python.sh`` (local)
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import a_bad_invoke_argument_is_reported_once_sm as _sm  # noqa: E402 — path inserted above

_State = _sm.ABadInvokeArgumentIsReportedOnceState
_Event = _sm.ABadInvokeArgumentIsReportedOnceEvent


def test_each_argument_costs_what_its_clause_says() -> None:
    engine = _sm.create_engine()
    engine.initialize()
    for event in (_Event.GO, _Event.FINISH):
        engine.send_event(event)

    p = engine.policy
    assert engine.terminal_state == _State.DONE, "`finish` must carry the run to `done`"
    observed = {
        "errors": (p.errors(), 5),
        "started": (p.started(), 1),
        "fromLocOk": (p.from_loc_ok(), 1),
        "emptyLocLeftOut": (p.empty_loc_left_out(), 1),
        "brokenLeftOut": (p.broken_left_out(), 1),
    }
    wrong = {name: got for name, (got, want) in observed.items() if got != want}
    assert not wrong, f"observed {wrong}, want { {n: w for n, (_, w) in observed.items()} }"
