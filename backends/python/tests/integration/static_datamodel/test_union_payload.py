# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""A union value, in the Python runtime (docs/adr/0092, sections 2 and 4).

A union is read tag first, by the reader of the tag's enum, and then the record
of the case its tag names, from the same object; it is written as one object,
its tag among the fields of its case, every key ascending by its bytes.
"""
from __future__ import annotations

from dataclasses import dataclass
from enum import IntEnum
from typing import Any

import pytest

from sce_runtime.event_payload import (
    CaseOf,
    RecordOf,
    RefusalCode,
    TypedPayloadError,
    UnionOf,
    Whole,
    lift,
    lift_value,
    union_json,
)


class _ShapeKind(IntEnum):
    """The tag's enum, as a `sce-static` machine declares one."""

    DOT = 0
    CIRCLE = 1
    DISC = 2

    @property
    def sce_name(self) -> str:
        return {0: "dot", 1: "circle", 2: "disc"}[int(self)]


@dataclass
class _Circle:
    radius: int


@dataclass
class _Disc:
    radius: int


@dataclass
class _Dot:
    pass


_RADIUS = RecordOf([("radius", Whole(0, (1 << 16) - 1))], make=lambda radius: radius)

# `circle` and `disc` hold one record and stay apart by their tag.
_SHAPE = UnionOf("kind", _ShapeKind, [
    CaseOf(_ShapeKind.DOT, make=_Dot),
    CaseOf(_ShapeKind.CIRCLE, _RADIUS, make=_Circle),
    CaseOf(_ShapeKind.DISC, _RADIUS, make=_Disc),
])


def _read(data: str, declared: Any = _SHAPE) -> Any:
    return lift(data, [("s", declared)])[0]


@pytest.mark.parametrize("data, want", [
    ('{"s":{"kind":"dot"}}', _Dot()),
    ('{"s":{"radius":2,"kind":"disc"}}', _Disc(2)),
    ('{"s":{"kind":"circle","radius":2,"width":null,"text":"x"}}', _Circle(2)),
    ('{"s":{"kind":"dot","radius":null}}', _Dot()),
])
def test_a_union_reads_its_tag_first_and_then_the_record_of_its_case(data: str, want: Any) -> None:
    assert _read(data) == want


@pytest.mark.parametrize("data, code, pointer", [
    ('{"s":[]}', RefusalCode.WRONG_KIND, "/s"),
    ('{"s":null}', RefusalCode.NULL_NOT_OPTIONAL, "/s"),
    ('{"s":{"radius":2}}', RefusalCode.MISSING, "/s/kind"),
    ('{"s":{"kind":null,"radius":null}}', RefusalCode.NULL_NOT_OPTIONAL, "/s/kind"),
    ('{"s":{"kind":3,"radius":"x"}}', RefusalCode.WRONG_KIND, "/s/kind"),
    ('{"s":{"kind":"oval","radius":2}}', RefusalCode.UNKNOWN_VARIANT, "/s/kind"),
    ('{"s":{"kind":"circle","width":2}}', RefusalCode.MISSING, "/s/radius"),
    ('{"s":{"kind":"disc","radius":null}}', RefusalCode.NULL_NOT_OPTIONAL, "/s/radius"),
])
def test_a_union_refuses_at_its_tag_or_at_the_field_of_its_case(
        data: str, code: RefusalCode, pointer: str) -> None:
    with pytest.raises(TypedPayloadError) as refused:
        _read(data)
    assert (refused.value.code, refused.value.pointer) == (code, pointer), str(refused.value)


def test_a_union_with_no_builder_reads_as_its_variant_and_record() -> None:
    plain = UnionOf("kind", _ShapeKind, [
        CaseOf(_ShapeKind.DOT),
        CaseOf(_ShapeKind.CIRCLE, RecordOf([("radius", Whole(0, 255))])),
        CaseOf(_ShapeKind.DISC, RecordOf([("radius", Whole(0, 255))])),
    ])
    assert _read('{"s":{"kind":"dot"}}', plain) == (_ShapeKind.DOT, None)
    assert _read('{"s":{"kind":"circle","radius":4}}', plain) == (_ShapeKind.CIRCLE, {"radius": 4})


def test_a_union_is_written_with_its_tag_among_the_fields_of_its_case() -> None:
    assert union_json("kind", "none", {}) == '{"kind":"none"}'
    assert union_json("kind", "relativeWeeks", {"count": 2}) == '{"count":2,"kind":"relativeWeeks"}'
    assert (union_json("kind", "absolute", {"year": 2027, "month": 3, "day": 3})
            == '{"day":3,"kind":"absolute","month":3,"year":2027}')


def test_a_lift_reads_a_text_into_a_union_whole_or_answers_its_refusal() -> None:
    """A `<sce:lift>` into a union reads the text's object at its root as a
    delivery's field of the union is read, tag first (docs/adr/0092, section
    6.1 item 5), and points a refusal inside it below the root."""
    assert lift_value('{"radius":3,"kind":"disc"}', 1, 256, _SHAPE) == (_Disc(3), None)
    assert lift_value('{"kind":"dot"}', 1, 256, _SHAPE) == (_Dot(), None)
    for text, code, pointer in [
        (" ", RefusalCode.NOT_JSON, ""),
        ("[1]", RefusalCode.NOT_OBJECT, ""),
        ('{"radius":3}', RefusalCode.MISSING, "/kind"),
        ('{"kind":"oval"}', RefusalCode.UNKNOWN_VARIANT, "/kind"),
        ('{"kind":"circle"}', RefusalCode.MISSING, "/radius"),
        ('{"kind":"circle","radius":{"r":1}}', RefusalCode.TOO_DEEP, "/radius"),
    ]:
        value, refusal = lift_value(text, 1, 256, _SHAPE)
        assert value is None and refusal is not None, text
        assert (refusal.code, refusal.pointer) == (code, pointer), text
