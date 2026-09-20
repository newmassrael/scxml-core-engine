# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML 6.4.3 on the Python AOT path — the expression is evaluated.

An ``<invoke>`` naming its target through an expression evaluates that
expression at invoke-fire time, and a failure to evaluate raises
``error.execution``. The clause puts two obligations on the Processor and this
fixture is about the second only: the evaluated string does not select the
child on this path, because codegen fixes the child at build time and writes
an immediate-``<final>`` stub for it (docs/SCE_ACCEPTED_SUBSET.md §2.13).

This channel is why the fixture exists. Measured 2026-09-20: the Python
template had no evaluation step at all, so a document whose expression could
not be evaluated started its child here as though nothing were wrong, while
four other backends raised. Nothing in the tree could see the difference —
every other invoke fixture uses an expression that evaluates.

Fixture: ``integration_resources/invoke_expression_failure_is_reported/invoke_expression_failure_is_reported.scxml``.

Regeneration:
  ``scripts/regen_invoke_expression_failure_is_reported_python.sh`` (local)
  ``sce-codegen generate-integration -l python --stem invoke_expression_failure_is_reported`` (CI)
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import invoke_expression_failure_is_reported_sm as _sm  # noqa: E402 — path inserted above


def test_invoke_expression_failure_is_reported_aot() -> None:
    engine = _sm.create_engine()
    engine.initialize()

    elapsed = 0
    while not engine.reached_final and elapsed < 100:
        engine.advance_time(10)
        elapsed += 10

    assert engine.reached_final, (
        "the machine never completed; last leaf="
        f"{engine.current_state!s}. W3C SCXML 6.4.3 requires the expression to be "
        "evaluated when the <invoke> fires; parking means neither the raise nor "
        "the child arrived"
    )
    assert str(engine.current_state) == "pass", (
        "reaching `fail` means the child started on an expression that cannot be "
        f"evaluated, so nothing evaluated it; it reached {engine.current_state!s}"
    )
