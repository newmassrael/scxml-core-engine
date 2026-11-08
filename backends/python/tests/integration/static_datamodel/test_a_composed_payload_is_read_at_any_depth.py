# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""A Python machine reads a composed payload at any depth (docs/adr/0086,
section 6), run with no script engine, as the Rust and Kotlin machines do
(``backends/rust/tests/tests/a_composed_payload_is_read_at_any_depth.rs``,
``AComposedPayloadIsReadAtAnyDepthTest.kt``). The machine is generated from
``sce-build/tests/fixtures/static_datamodel/static_composed_payload.scxml`` by
``scripts/regen_static_datamodel_python.sh``, into this directory and out of
version control.

A delivery whose every list is full is lifted into the machine's own types, and
each read at depths 1 to 3 is what a host reads back. A delivery one past a
bound at depth 2 is refused whole (section 4.2): none of the content runs, and
``error.execution`` is raised once. A value a host injects through the seam is
held to every bound where it is made (section 5.4): within them it is read as
any delivery, and one past a bound at any depth raises ``BoundError`` from the
``__post_init__`` of the class that holds it, before anything is sent. A list
taken whole into a variable is the machine's own: what the machine writes into
it does not reach the list the host handed in.
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
from sce_runtime.event_payload import BoundError  # noqa: E402 -- path inserted above

_MACHINE = "integration.static_datamodel.static_composed_payload_sm"


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


def _batch(second_tags: int) -> str:
    """A delivery of ``batch.sent`` whose every list is full, the second item's
    ``tags`` holding ``second_tags`` texts."""
    return json.dumps({
        "items": [_item(1, 4), _item(2, second_tags), _item(3, 4)],
        "grid": [[1, 2], [3, 4]],
        "levels": ["low", "critical"],
        "maybe": [7, None],
        "spare": [1, 2],
        "note": "abcd",
        "late": [5, 6],
    })


def _machine():
    """The generated machine's module, and an engine of it, started."""
    module = importlib.import_module(_MACHINE)
    engine = module.create_engine()
    engine.initialize()
    return module, engine


def _delivered(data: str):
    """The policy of a machine that took one delivery of ``batch.sent`` carrying
    ``data``."""
    _, engine = _machine()
    policy = engine.policy
    engine.send_event(policy.resolve_event_by_name("batch.sent"), EventMetadata(data=data))
    return policy


def test_a_full_delivery_is_read_at_every_depth() -> None:
    policy = _delivered(_batch(4))
    assert policy.refusals() == 0, "a delivery of its type is no error"
    # Depth 1: the length of a list of the payload.
    assert policy.item_count() == 3
    # Depth 2: a loop over each item's list, and a field of an element read
    # through its index.
    assert policy.tag_count() == 12
    assert policy.word_count() == 12
    assert policy.id_sum() == 6
    # A list of lists, walked at both depths and read by two indices.
    assert policy.cell_sum() == 10
    assert policy.cell() == 3
    # Depth 3: the length of the first item's `tags`.
    assert policy.first_tags() == 4
    # An element that may be absent, present.
    assert policy.first_maybe() == 7
    # A list taken whole into a list of a larger bound.
    assert policy.late() == [5, 6]


def test_a_list_past_its_bound_at_depth_two_is_refused_whole() -> None:
    policy = _delivered(_batch(5))
    assert policy.refusals() == 1, "error.execution reached the document once"
    assert policy.item_count() == 0, "none of the content ran"
    assert policy.late() == [], "nothing of the delivery was kept"


def _past(change) -> str:
    """A delivery of ``batch.sent`` whose every list is full, changed by
    ``change`` in place to put one value past its bound."""
    batch = json.loads(_batch(4))
    change(batch)
    return json.dumps(batch)


@pytest.mark.parametrize("delivery", [
    # A text and a byte string inside an element.
    _past(lambda b: b["items"][0].update(label="abcdefghi")),
    _past(lambda b: b["items"][0].update(blob="vwxyz")),
    # The top-level text.
    _past(lambda b: b.update(note="abcde")),
    # The count of the top-level list.
    _past(lambda b: b["items"].insert(0, _item(0, 1))),
])
def test_every_bound_of_the_schema_is_held_where_the_delivery_is_read(delivery: str) -> None:
    """Every bound of the schema is held where the delivery is read
    (docs/adr/0086, section 4.4): each value one past its bound in a delivery
    otherwise full refuses the delivery whole."""
    assert delivery != _batch(4), "the case changes the delivery"
    policy = _delivered(delivery)
    assert policy.refusals() == 1, "error.execution reached the document once"
    assert policy.item_count() == 0, "none of the content ran"
    assert policy.late() == [], "nothing of the delivery was kept"


def _host_item(module, id_: int, tags: int):
    """An item a host builds, its ``tags`` holding ``tags`` texts."""
    level = next(iter(module.StaticComposedPayloadSignalLevelEnum))
    return module.StaticComposedPayloadItemRecord(
        id_, 2, "abc", level, None, b"wx", [f"t{n}" for n in range(tags)])


def test_a_value_a_host_injects_within_its_bounds_is_read_as_any_delivery() -> None:
    module, engine = _machine()
    level = next(iter(module.StaticComposedPayloadSignalLevelEnum))
    late = [5, 6]
    module.raise_batch_sent(
        engine, [_host_item(module, 4, 2)], [[1, 2]], [level], [9, None], None, "ab", late)
    policy = engine.policy
    assert policy.refusals() == 0
    assert policy.item_count() == 1
    assert policy.tag_count() == 2
    assert policy.id_sum() == 4
    assert policy.cell_sum() == 3
    assert policy.first_maybe() == 9
    assert policy.late() == [5, 6]
    # The variable holds a copy of its own: the list the host handed in is not
    # the one the machine keeps.
    assert policy.v_late is not late


def test_a_list_a_host_injects_one_past_its_bound_is_refused_where_it_is_made() -> None:
    # `late` is a list<uint8>[2]: three elements are no value of its type, and
    # the seam's payload refuses them before anything is sent.
    module, engine = _machine()
    with pytest.raises(BoundError, match=r"^'late' holds 3 elements, past the 2 its type declares$"):
        module.raise_batch_sent(engine, [_host_item(module, 4, 2)], [], [], [], None, "ab", [5, 6, 7])
    assert engine.policy.refusals() == 0, "nothing was sent, so nothing was refused"
    assert engine.policy.late() == []


def test_a_record_a_host_builds_one_past_an_inner_bound_is_refused_where_it_is_made() -> None:
    # `tags` is a list<string[8]>[4] inside each item: an item of five is
    # refused where the host builds it, a tag of nine bytes too, and an `id` past
    # the width of a uint32, which a Python `int` holds no width of.
    module = importlib.import_module(_MACHINE)
    with pytest.raises(BoundError, match=r"^'tags' holds 5 elements, past the 4 its type declares$"):
        _host_item(module, 1, 5)
    level = next(iter(module.StaticComposedPayloadSignalLevelEnum))
    with pytest.raises(BoundError, match=r"^'tags/0' is 9 bytes, past the 8 its type declares$"):
        module.StaticComposedPayloadItemRecord(1, 2, "abc", level, None, b"wx", ["ninebytes"])
    with pytest.raises(BoundError, match=r"^'id' does not fit the width its schema declares"):
        module.StaticComposedPayloadItemRecord(1 << 32, 2, "abc", level, None, b"wx", [])


if __name__ == "__main__":
    sys.exit(pytest.main([__file__]))
