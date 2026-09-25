# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML 5.3: a typed ``<data>`` whose id is a Python keyword still gets a
reader, and it reads its own variable — Python AOT.

``<data id="pass">`` used to generate ``def pass(self)``, a syntax error; and a
reader that met one of the policy's own methods silently replaced it, since the
later ``def`` wins. ``sce-build/src/reader_names.rs`` now spells the keyword
``pass_`` (PEP 8) and gives no reader to an id no backend can spell
(``auto``, ``self``, ``new``, ``start``, ``t``, and ``a-b`` beside ``a_b``).
The generated module importing at all is what shows no keyword was emitted.

Fixture: ``integration_resources/typed_reader_names/typed_reader_names.scxml``.

Regeneration (after fixture or template edit):
  ``scripts/regen_typed_reader_names_python.sh``
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import typed_reader_names_sm as _sm  # noqa: E402 — path inserted above

_Event = _sm.TypedReaderNamesEvent


def _started():
    engine = _sm.create_engine()
    engine.initialize()
    return engine


def test_a_reader_reads_its_own_variable() -> None:
    """Each reader reads the value its own variable was declared with."""
    p = _started().policy
    assert (p.box(), p.object(), p.pass_(), p.screen_rules(), p.a_b()) == (1, 2, 3, 4, 10)


def test_a_reader_reads_the_live_value() -> None:
    """``bump`` adds 10 to four of them, and leaves ``screen-rules`` — which no
    ECMAScript expression can name — alone."""
    engine = _started()
    engine.send_event(_Event.BUMP)
    p = engine.policy
    assert (p.box(), p.object(), p.pass_(), p.screen_rules(), p.a_b()) == (11, 12, 13, 4, 20)


def test_no_reader_was_emitted_for_a_refused_name() -> None:
    """Python would not have refused these — a later ``def`` quietly wins — so
    the absence is asserted rather than left to a compiler that is not there.
    Refused in some other backend, they are refused here too: a reader's
    existence does not depend on the backend."""
    policy = type(_started().policy)
    for refused in ("auto", "self", "new", "start", "t"):
        assert not hasattr(policy, refused), refused
