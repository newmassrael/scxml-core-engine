# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""A Python machine sends a composed value by its name (docs/adr/0086, section
5.2), run with no script engine, as the Rust and Kotlin machines do
(``backends/rust/tests/tests/a_composed_value_is_sent_by_its_name.rs``,
``AComposedValueIsSentByItsNameTest.kt``). The machine is generated from
``sce-build/tests/fixtures/static_datamodel/static_composed_send.scxml`` by
``scripts/regen_static_datamodel_python.sh``, into this directory and out of
version control.

The first delivery of ``batch.sent`` is forwarded to the machine itself: four
lists and an optional list named as the parts of the payload they are, and a
list variable the machine took the payload's ``late`` into. The delivery that
comes back is lifted as any delivery is (section 4), so it is refused whole if
anything written is outside its type; it is not, and every value read from it
at depths 1 to 3 is what the first delivery held.
"""
from __future__ import annotations

import importlib
import json
import sys
from pathlib import Path

import pytest

_HERE = Path(__file__).resolve().parent
# The tests directory, so this package is importable by the name it has there.
sys.path.insert(0, str(_HERE.parents[1]))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))
sys.path.insert(0, str(_HERE.parents[2] / "forge-runtime"))

from sce_runtime import EventMetadata  # noqa: E402 -- path inserted above


def _item(id_: int, tags: int) -> dict:
    """One item of the batch: ``tags`` holds ``tags`` texts."""
    return {
        "id": id_,
        "amount": "9007199254740993",
        "label": "abcdefgh",
        "level": "high",
        "due": None,
        "blob": "wxyz",
        "tags": [f"tag{n}" for n in range(tags)],
    }


def _batch() -> str:
    """A delivery of ``batch.sent`` whose every list is full, an element of
    ``maybe`` absent and ``spare`` present."""
    return json.dumps({
        "items": [_item(1, 4), _item(2, 3), _item(3, 4)],
        "grid": [[1, 2], [3, 4]],
        "levels": ["low", "critical"],
        "maybe": [7, None],
        "spare": [1, 2],
        "note": "abcd",
        "late": [5, 6],
    })


def test_a_delivery_forwarded_by_its_names_comes_back_whole() -> None:
    module = importlib.import_module("integration.static_datamodel.static_composed_send_sm")
    engine = module.create_engine()
    engine.initialize()
    policy = engine.policy
    engine.send_event(policy.resolve_event_by_name("batch.sent"), EventMetadata(data=_batch()))
    assert policy.refusals() == 1, (
        "the delivery the machine sent itself fits its schema, and the one refusal is the "
        "list sent short of its field's lower count"
    )
    # A list variable sent to a field that states a lower count (docs/adr/0086,
    # section 15.6): sent empty, refused whole and never delivered; sent with
    # its one element, delivered, and forwarded once more with its scalar
    # `hops`, a scalar field of a payload that holds a list.
    assert policy.counted_deliveries() == 2
    assert policy.counted_sum() == 7
    assert policy.counted_hops() == 3
    assert policy.deliveries() == 2, "the forwarded delivery came back on the internal queue"
    # Depth 1: the lengths of a list of records and a list of enum values.
    assert policy.item_count() == 3
    assert policy.level_count() == 2
    # Depth 2: each item's list, and a field of an element read by its index.
    assert policy.tag_count() == 11
    assert policy.id_sum() == 6
    # A list of lists, walked at both depths.
    assert policy.cell_sum() == 10
    # An element that may be absent, present.
    assert policy.first_maybe() == 7
    # The list variable, sent by its name.
    assert policy.late_sum() == 11
    assert policy.late() == [5, 6]


if __name__ == "__main__":
    sys.exit(pytest.main([__file__]))
