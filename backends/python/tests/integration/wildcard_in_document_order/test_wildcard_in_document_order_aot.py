# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""A wildcard keeps its own guard and its own type — Python AOT path.

W3C SCXML 3.12.1 lets a ``*`` descriptor match every event, and that is all it
changes: W3C SCXML 3.13 still asks the transition's ``cond`` whether it is
enabled, and a ``type="internal"`` transition whose target is a proper
descendant of its compound source still does not exit that source.

No W3C document writes a wildcard with a ``cond`` or with ``type="internal"``,
so a generator that moved the wildcard into a hand-written fallback dropped both
without a suite noticing — the Kotlin one did until 2026-09-13.

Fixture: ``integration_resources/wildcard_in_document_order/wildcard_in_document_order.scxml``.

Regeneration (after fixture or template edit):
  ``scripts/regen_wildcard_in_document_order_python.sh`` (local)
  ``sce-codegen generate-integration -l python --stem wildcard_in_document_order`` (CI)
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import wildcard_in_document_order_sm as _sm  # noqa: E402 — path inserted above
from sce_runtime.scripting import LuaScriptEngine  # noqa: E402


def test_wildcard_in_document_order_aot() -> None:
    # The guards and the entry counters need a datamodel, so this is an
    # ECMAScript-datamodel machine.
    script_engine = LuaScriptEngine()
    script_engine.initialize()
    engine = _sm.create_engine(script_engine=script_engine)
    engine.initialize()

    elapsed = 0
    while not engine.reached_final and elapsed < 2000:
        engine.advance_time(10)
        elapsed += 10

    assert engine.reached_final, (
        "the machine never reached a final state (parked in "
        f"{engine.current_state!s}); resting in guardedFrom or sealedFrom means an "
        "internal wildcard was not taken at all"
    )

    # Each failure final names the case, so this one assertion says which
    # property of the wildcard was lost:
    #   failGuardIgnored              a wildcard fired with its guard false
    #   failGuardNeverFired           a wildcard did not fire with its guard true
    #   failGuardedInternalReentered  a guarded internal wildcard exited its source
    #   failSealedInternalReentered   an unguarded internal wildcard exited its source
    assert str(engine.terminal_state) == "pass", (
        f"the machine rested in {engine.terminal_state!s}: a wildcard is enabled only "
        "when its guard is true, and an internal wildcard targeting a descendant of "
        "its compound source must not exit and re-enter that source"
    )
