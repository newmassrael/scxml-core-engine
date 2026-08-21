# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""§scxml-6.4.3: an ``<invoke>`` ``<param>`` seeds a declared ``<data>`` of
the invoked session with the INVOKING session's value — Python AOT channel.

The clause has two halves and the fixture gives each one a ``<final>``: a
matching name takes the param's value (and the child's own ``<data>``
expression is ignored), and a name matching no top-level ``<data>`` is not
added to the child's data model at all. A fourth phase carries the same
rule through the sibling ``namelist`` syntax of §scxml-6.4.1.

The W3C IRP param surface (226, 240, 241, 243, 244, 245, 276) passes
literals only, so it cannot separate "the parent evaluated this" from "the
child evaluated this text" — ``1`` means ``1`` in either data model. This
fixture makes the two answers differ.

Fixture: ``integration_resources/invoke_param_seeds_declared_child_data/invoke_param_seeds_declared_child_data.scxml``.

Regeneration (after fixture or template edit):
  ``scripts/regen_invoke_param_seeds_declared_child_data_python.sh`` (local)
  ``sce-codegen generate-integration -l python --stem invoke_param_seeds_declared_child_data`` (CI)
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import invoke_param_seeds_declared_child_data_sm as _sm  # noqa: E402 — path inserted above

_WHY = {
    "failChildEvaluatedTheExpression": (
        "the child evaluated the author's `<param expr>` text in its own data "
        "model and found its own `token` — §scxml-6.4.3 says the value of the "
        "param element, which only the invoking session can produce"
    ),
    "failParentOnlyExprLost": (
        "the expression named a variable only the parent has and nothing "
        "arrived — the same defect where the child has no shadow to find"
    ),
    "failUnmatchedParamEnteredTheChild": (
        "a `<param>` naming no top-level `<data>` of the child became a "
        "variable there anyway; the clause forbids adding it"
    ),
    "failNamelistValueLost": (
        "the namelist value did not arrive — a rendered string forwarded as "
        "an expression becomes an identifier lookup in the child"
    ),
    "failShadowSeedLost": (
        "the child saw neither the parent's value nor a shadow, so its own "
        "`<data>` default stood — nothing was seeded at all"
    ),
    "failDeclaredParamLost": (
        "the declared param did not arrive, so the child's own `<data>` "
        "default stood"
    ),
}


def test_invoke_param_seeds_declared_child_data_aot() -> None:
    engine = _sm.create_engine()
    engine.initialize()

    elapsed = 0
    while not engine.reached_final and elapsed < 200:
        engine.advance_time(10)
        elapsed += 10

    assert engine.reached_final, (
        "invoke_param_seeds_declared_child_data did not reach a top-level "
        f"<final> within 200 ms; last leaf={engine.current_state}"
    )
    actual = str(engine.current_state)
    assert actual == "pass", (
        f"reached <final id={actual!r}>: {_WHY.get(actual, 'unknown verdict state')}"
    )
