# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML B.2: the reading of inline ``<content>`` text is the data model's,
on the Python AOT path.

Under ``datamodel="ecmascript"`` a ``<send>``'s ``21`` is the number 21,
``{"k": 7}`` an object, and text that is no expression the space-normalized
string, whether or not the sending machine needs a script engine for anything
else; a ``<donedata>``'s XML is a document. The child that sends them needs no
engine.

Fixture: ``integration_resources/inline_content_is_read_by_its_data_model/inline_content_is_read_by_its_data_model.scxml``.

Regeneration (after fixture or template edit):
  ``scripts/regen_inline_content_is_read_by_its_data_model_python.sh`` (local)
  ``sce-codegen generate-integration -l python --stem inline_content_is_read_by_its_data_model`` (CI)
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import inline_content_is_read_by_its_data_model_sm as _sm  # noqa: E402 — path inserted above

_WHY = {
    "failNumber": (
        "`number` arrived with `_event.data` unequal to 21: under the ECMAScript data "
        "model <content>21</content> is the number 21, finished at build time, whether "
        "or not the sending machine needs a script engine."
    ),
    "failText": (
        "`text` arrived with `_event.data` other than 'two words': text that is no "
        "expression is the space-normalized string it spells."
    ),
    "failObject": (
        "`object` arrived without `_event.data.k === 7`: <content>{\"k\": 7}</content> "
        "is an object literal, the object it denotes."
    ),
    "failDocument": (
        "`done.invoke.reporter` arrived without a document whose `book` carries "
        "title=\"x\": a <donedata>'s XML <content> travels as its text and the receiver "
        "builds the DOM; it is not an expression to evaluate."
    ),
}


def test_inline_content_is_read_by_its_data_model_aot() -> None:
    engine = _sm.create_engine()
    engine.initialize()

    elapsed = 0
    while not engine.reached_final and elapsed < 100:
        engine.advance_time(10)
        elapsed += 10

    assert engine.reached_final, (
        "inline_content_is_read_by_its_data_model did not reach a top-level <final> "
        f"within 100 ms; last leaf={engine.current_state} — the child's events never "
        "reached the parent"
    )
    actual = str(engine.terminal_state)
    assert actual == "pass", (
        f"inline_content_is_read_by_its_data_model reached <final id={actual!r}>; "
        "expected 'pass'. " + _WHY.get(actual, "That is not a verdict state.")
    )
