# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML 5.5 + 6.3.1 donedata surfacing on the Python AOT
local-invoke path.

Closes the W3C IRP coverage gap on the AOT engine: no public IRP
test exercises ``<donedata>`` on the invoked child's top-level
``<final>`` combined with ``done.invoke.<id>._event.data`` readback
on the parent. The pybind11 channel test at
``sce-python/tests/test_donedata_local_invoke.py`` covers the same
contract via the C++ Interpreter; both production channels are
verified independently.

Fixture: ``integration_resources/donedata_local_invoke/donedata_local_invoke.scxml``.

Regeneration (after fixture or template edit):
  ``scripts/regen_donedata_local_invoke_python.sh`` (local)
  ``sce-codegen generate-integration -l python --stem donedata_local_invoke`` (CI)
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "sce-python-runtime"))

import donedata_local_invoke_sm as _sm  # noqa: E402 — path inserted above


def test_donedata_local_invoke_aot() -> None:
    engine = _sm.create_engine()
    engine.initialize()

    elapsed = 0
    while not engine.reached_final and elapsed < 100:
        engine.advance_time(10)
        elapsed += 10

    assert engine.reached_final, (
        "donedata_local_invoke did not reach a top-level <final> within "
        f"100 ms; last leaf={engine.current_state}"
    )
    actual = str(engine.current_state)
    assert actual == "pass", (
        f"donedata_local_invoke reached <final id={actual!r}>; "
        "expected 'pass' — donedata envelope round-trip regressed on "
        "the Python AOT engine"
    )
