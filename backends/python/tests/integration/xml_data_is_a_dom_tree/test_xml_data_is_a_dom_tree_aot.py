# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML B.2: a ``<data>`` element's XML content is a DOM tree — Python AOT.

The appendix obliges the Processor to create "the corresponding DOM
structure". Measured 2026-08-18, every backend created an object carrying
three methods — ``getElementsByTagName``, ``getAttribute`` and a non-standard
``getTagName``, which are the two names the W3C IRP suite reads plus one — so
``doc.tagName``, ``doc.firstChild`` and ``doc.childNodes.length`` answered nil
on all seven channels with the whole W3C suite green.

What this adds to ``tests/ecmascript/test_dom_read_surface.py``, which measures
the same surface against the same shared table, is the SEAM: the ``<data>``
initializer the code generator emits, and the guards it lowered. A binding
being right does not say a document reaches it.

Fixture: ``integration_resources/xml_data_is_a_dom_tree/xml_data_is_a_dom_tree.scxml``.

Regeneration (after fixture or template edit):
  ``scripts/regen_xml_data_is_a_dom_tree_python.sh`` (local)
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import xml_data_is_a_dom_tree_sm as _sm  # noqa: E402 — path inserted above

_State = _sm.XmlDataIsADomTreeState


def test_xml_data_is_a_dom_tree_aot() -> None:
    engine = _sm.create_engine()
    # Every transition is eventless, so the verdict is reached inside
    # `initialize`, which drives the macrostep loop until stable — no event is
    # needed to ask the question.
    engine.initialize()

    active = engine.active_configuration()
    assert _State.NOT_ADOCUMENT not in active, (
        f"the variable did not hold a document: nodeType === 9, "
        f"nodeName === '#document', documentElement.tagName === 'books' or "
        f"hasAttribute('count') did not hold (active: {active})"
    )
    assert _State.WRONG_TREE not in active, (
        f"the document element's children are not the two <book> elements in "
        f"document order — the whitespace between them may have become nodes, "
        f"or a sibling/parent link is missing (active: {active})"
    )
    assert _State.NO_TEXT not in active, (
        f"character data did not report itself as a text node, or textContent "
        f"did not read the text below the element (active: {active})"
    )
    assert _State.SETTLED in active, (
        f"the machine reached none of its four verdicts, so the guards did not "
        f"evaluate at all (active: {active})"
    )
