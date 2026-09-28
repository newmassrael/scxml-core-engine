# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML 5.6.2 + 6.2: a <send>'s <content expr> is the payload — Python AOT.

The expression is evaluated when the send is and its value is the event's
data; one that fails raises error.execution, the event still arrives carrying
the empty string, and the rest of the block does not run.

Fixture: ``integration_resources/a_send_content_expr_is_the_payload/a_send_content_expr_is_the_payload.scxml``.

Regeneration (after fixture or template edit):
  ``scripts/regen_a_send_content_expr_is_the_payload_python.sh`` (local)
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import a_send_content_expr_is_the_payload_sm as _sm  # noqa: E402 — path inserted above

_State = _sm.ASendContentExprIsThePayloadState


def test_a_send_content_expr_is_the_payload() -> None:
    engine = _sm.create_engine()
    engine.initialize()
    elapsed = 0
    while not engine.reached_final and elapsed < 300:
        engine.advance_time(10)
        elapsed += 10

    p = engine.policy
    assert engine.terminal_state == _State.DONE, "the run must end in `done`"
    observed = {
        "numberOk": (p.number_ok(), 1),
        "objectOk": (p.object_ok(), 1),
        "textOk": (p.text_ok(), 1),
        "errors": (p.errors(), 1),
        "badArrived": (p.bad_arrived(), 1),
        "badEmpty": (p.bad_empty(), 1),
        "afterBad": (p.after_bad(), 0),
    }
    wrong = {name: got for name, (got, want) in observed.items() if got != want}
    assert not wrong, f"observed {wrong}, want { {n: w for n, (_, w) in observed.items()} }"
