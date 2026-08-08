# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML 6.4.1: an unsupported ``<invoke type>`` raises ``error.execution`` — Python AOT path.

The spec defines the case ("the processor MUST place error.execution in the
internal event queue"), so the document is valid SCXML with one observable:
that raise. No child session starts and ``done.invoke.<id>`` never fires.

Both engines were silent here in different ways before this landed — the
Interpreter substituted an SCXML handler for the unknown type, and AOT dropped
the ``<invoke>`` from the model entirely. A backend that renders this fixture
without the raise reproduces the AOT form, and the machine then rests in
``probe`` instead of reaching ``pass``.

Fixture: ``integration_resources/invoke_unsupported_type/invoke_unsupported_type.scxml``.

Regeneration (after fixture or template edit):
  ``scripts/regen_invoke_unsupported_type_python.sh`` (local)
  ``sce-codegen generate-integration -l python --stem invoke_unsupported_type`` (CI)
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import invoke_unsupported_type_sm as _sm  # noqa: E402 — path inserted above


def test_invoke_unsupported_type_aot() -> None:
    engine = _sm.create_engine()
    engine.initialize()

    elapsed = 0
    while not engine.reached_final and elapsed < 100:
        engine.advance_time(10)
        elapsed += 10

    assert engine.reached_final, (
        "the machine never completed; last leaf="
        f"{engine.current_state!s}. W3C SCXML 6.4.1 requires an <invoke> whose "
        "`type` names no supported processor to place error.execution on the "
        "internal queue; parking in `probe` means the <invoke> was dropped "
        "rather than lowered"
    )
    assert str(engine.current_state) == "pass", (
        "the machine completed somewhere other than the error.execution target; "
        f"it reached {engine.current_state!s}"
    )
