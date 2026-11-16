# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""SCE Accepted Subset §2.15, "Child sessions", and §scxml-6.4 (docs/adr/0098)
on the Python channel: a ``datamodel="sce-static"`` child declares its
variables with the values its invoke handed, each in place of its own initial
value, so a variable declared after a handed one reads the handed value.

``worker`` is handed ``start = 7`` and declares ``copied = start`` after it; it
ends the moment ``copied`` is 7. A child that wrote the handed value over
``start`` after declaring both holds ``copied = 0``, never ends, and leaves the
parent in ``working``.

Fixture: ``tests/integration/a_static_child_declares_its_data_with_a_handed_value.scxml``.
The Rust and Go halves are ``a_static_child_declares_its_data_with_a_handed_value.rs``
and ``a_static_child_declares_its_data_with_a_handed_value_test.go``.
"""
from __future__ import annotations

import importlib
import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE.parents[1]))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))
sys.path.insert(0, str(_HERE.parents[2] / "forge-runtime"))

_HANDED = importlib.import_module(
    "integration.static_datamodel.a_static_child_declares_its_data_with_a_handed_value_sm"
)


def test_a_variable_declared_after_a_handed_one_reads_the_handed_value() -> None:
    engine = _HANDED.create_engine()
    engine.initialize()
    # Let the child start, run and end, and its parent take `done.invoke`.
    for _ in range(5):
        engine.advance_time(0)
    assert engine.reached_final and str(engine.terminal_state) == "passed", (
        "`worker` declares `copied = start` after `start`, which its invoke "
        "handed 7, so it ends; the parent stands in "
        f"{engine.current_state}"
    )
