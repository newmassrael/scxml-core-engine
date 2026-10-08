# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""§scxml-C-2: the `<param>`s of a BasicHTTP `<send>` of a `datamodel="sce-static"`
machine are read from its own fields when the send runs, and cross as the text a
form carries (docs/adr/0005, decision 4) -- Python. The request is observed where
the engine hands it to its transport, so no listener is involved.

Fixture: ``sce-build/tests/fixtures/static_datamodel/static_send_http.scxml``.
The machine is the one ``scripts/regen_static_datamodel_python.sh`` generates.
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

from sce_runtime import EventMetadata  # noqa: E402 -- path inserted above


def _machine():
    """A machine whose transport keeps what it was handed."""
    module = importlib.import_module("integration.static_datamodel.static_send_http_sm")
    engine = module.create_engine()
    posted: list = []
    engine.set_http_send_callback(lambda request: posted.append(request))
    engine.initialize()
    return engine, posted


def _raise(engine, name: str) -> None:
    event = engine.policy.resolve_event_by_name(name)
    assert event is not None, f"the machine's events do not match {name!r}"
    engine.send_event(event, EventMetadata(data=""))


def _pairs(**texts: str) -> dict:
    return {name: [text] for name, text in texts.items()}


def test_a_send_over_http_carries_the_text_the_fields_hold_when_it_runs() -> None:
    engine, posted = _machine()
    _raise(engine, "bump")
    _raise(engine, "go")

    assert len(posted) == 1, "one request is handed to the transport"
    request = posted[0]
    assert request.target == "http://example.invalid/hook"
    assert request.event_name == "note"
    assert request.content == "", "no <content>, so the body is the pairs"
    assert request.params == _pairs(
        count="4", ready="true", label="busy", twice="8", delta="-5", ratio="1.5"
    ), (
        "each value is the text it spells: an integer's digits, `true`, the string, "
        "a negative number, a real's String()"
    )


def test_the_pairs_are_the_fields_as_they_stand_and_not_a_copy_from_start_up() -> None:
    engine, posted = _machine()
    _raise(engine, "go")

    assert len(posted) == 1
    assert posted[0].params == _pairs(
        count="3", ready="false", label="idle", twice="6", delta="-5", ratio="1.5"
    ), (
        "without `bump` the fields hold their initial values, and the request carries "
        "those: it is read when the send runs"
    )


def test_a_param_that_cannot_be_read_is_left_out_and_the_request_still_goes() -> None:
    engine, posted = _machine()
    _raise(engine, "bump")
    _raise(engine, "boom")

    assert len(posted) == 1, "the request goes with the pair that could be read"
    assert posted[0].params == _pairs(count="4"), (
        "`big` is `count * 2000000000`, which a 32-bit field cannot hold: its pair is "
        "left out, not carried as a zero"
    )
    assert engine.policy.errors() == 1, (
        "§scxml-5.7.1: the failed pair is reported as error.execution, once"
    )
