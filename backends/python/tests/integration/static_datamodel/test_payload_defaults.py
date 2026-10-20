# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""A field a later revision of an event schema added, under generated Python.

A delivery written before the field existed lacks it, and the lift reads the
schema's default in its place; a field of the first revision has no default and
is still required (docs/SCE_ACCEPTED_SUBSET.md, "An event schema that grows").
``static_schema_revision`` holds the machine to the same answers the other
engines give; the cases here pin the lift itself.
"""
from __future__ import annotations

import pytest

from sce_runtime.event_payload import TypedPayloadError, Wide, lift

_DEFAULTS = '{"layout":"week","count":"7","urgent":true}'
_FIELDS = (
    ("level", int, 0, 255),
    ("layout", str),
    ("count", Wide, 0, (1 << 64) - 1),
    ("urgent", bool),
)


def test_a_field_the_delivery_lacks_is_read_out_of_the_defaults() -> None:
    assert lift('{"level": 4}', _FIELDS, _DEFAULTS) == [4, "week", 7, True]


def test_a_field_the_delivery_carries_is_read_as_it_came() -> None:
    delivered = {"level": 4, "layout": "day", "urgent": False, "source": {"a": [1]}}
    assert lift(delivered, _FIELDS, _DEFAULTS) == [4, "day", 7, False]
    # The delivery handed in is not written into.
    assert "count" not in delivered
    with pytest.raises(TypedPayloadError, match="is not a truth value"):
        lift({"level": 4, "urgent": 1}, _FIELDS, _DEFAULTS)


def test_a_field_of_the_first_revision_is_still_required() -> None:
    with pytest.raises(TypedPayloadError, match="'level'"):
        lift('{"layout": "day"}', _FIELDS, _DEFAULTS)
    # No data at all is a delivery that names no field.
    for nothing in (None, "", "  "):
        with pytest.raises(TypedPayloadError, match="'level'"):
            lift(nothing, _FIELDS, _DEFAULTS)
    with pytest.raises(TypedPayloadError, match="bare value"):
        lift("5", _FIELDS, _DEFAULTS)


def test_without_defaults_a_missing_field_is_refused_as_before() -> None:
    with pytest.raises(TypedPayloadError, match="'layout'"):
        lift('{"level": 4}', _FIELDS)
    with pytest.raises(TypedPayloadError, match="no data"):
        lift("", _FIELDS)
