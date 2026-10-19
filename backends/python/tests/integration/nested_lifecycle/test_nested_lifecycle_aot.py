# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""A statechart declared as a transition function answers as its machine does
(docs/SCE_TRANSITION_FUNCTION.md) — Python AOT, the nested lifecycle.

The document declares ``sce:transition-function="emit"`` and reaches every rule
the function is computed by: transitions held by an ancestor, a compound state
entered by its ``initial`` attribute, by an ``<initial>`` element and by its
first child, an internal and an external transition, a prefix and a wildcard
descriptor, a targetless transition and a top-level final. This drives the
machine to every state by the function's shortest path, sends every event
there, and holds what the engine does to what the function says, pair by pair.

Fixture: integration_resources/nested_lifecycle/nested_lifecycle.scxml
(canonical, shared with the C++ / Rust / Go / Kotlin / C11 channels).

Regeneration (after fixture or template edit):
  ``scripts/regen_nested_lifecycle_python.sh`` (local)
  ``sce-codegen generate-integration -l python --stem nested_lifecycle`` (CI)
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parent))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import nested_lifecycle_sm as _sm  # noqa: E402 — path inserted above
import nested_lifecycle_transition_function as _tf  # noqa: E402 — path inserted above
from transition_function_equivalence import expect_every_pair_agrees  # noqa: E402 — path inserted above


def test_nested_lifecycle_aot() -> None:
    pairs = expect_every_pair_agrees(_tf, _sm.create_engine)
    assert pairs == len(_tf.ALL_STATES) * len(_tf.ALL_EVENTS), "the drive did not check every pair"
