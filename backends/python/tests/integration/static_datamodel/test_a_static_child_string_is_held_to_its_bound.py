# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""SCE Accepted Subset §2.15, "Child sessions": a string an `<invoke
type="scxml">` hands its child is held to the bound the child declared for that
variable, in UTF-8 bytes, as an `<assign>` to it would be -- Python.

A value past it is the evaluation that failed (W3C SCXML 5.7.1):
`error.execution` is raised, that one value is left out, and the child still
starts, holding the one its `<data>` gave it.

``static_invoke_string.scxml`` invokes three children whose `title` holds four
bytes: `fits` is handed 'wxyz' and ends on it; `over` is handed eight bytes and
`wide` two characters of five bytes, both past the bound, so each starts with the
'ab' its `<data>` gave it, which it ends on. Python counts a string in
characters, so `wide` is the case that tells the bound is in bytes.

The Rust, Kotlin and Go halves are `a_static_child_string_is_held_to_its_bound.rs`,
`AStaticChildStringIsHeldToItsBoundTest.kt` and
`a_static_child_string_is_held_to_its_bound_test.go`.
"""
from __future__ import annotations

import importlib
import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
# The tests directory, so this package is importable by the name it has there.
sys.path.insert(0, str(_HERE.parents[1]))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))
sys.path.insert(0, str(_HERE.parents[2] / "forge-runtime"))


def test_a_string_past_the_childs_bound_is_left_out_and_reported() -> None:
    module = importlib.import_module("integration.static_datamodel.static_invoke_string_sm")
    engine = module.create_engine()
    engine.initialize()
    # 1 (`fits`) + 10 (`over`) + 100 (`wide`): all three ended, so none held a
    # value past its bound but the one that fits.
    assert engine.policy.completed() == 111
    # The two values past the bound were reported, once each.
    assert engine.policy.errors() == 2
