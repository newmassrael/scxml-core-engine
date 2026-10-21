# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""A value that may be absent (docs/adr/0055), in the Python runtime.

Python holds an absent value as ``None`` and a present one as the value itself,
so the one thing every reader and writer must not do is a truth test: a present
``0``, ``''``, ``False`` or a variant whose number is 0 is a value. On the wire
absence is JSON ``null``, and a delivery that lacks the key of a first-revision
field is refused, not read as absent. ``static_optional`` holds the machine to
the answers the other engines give; the cases here pin the runtime's reader
(``lift`` over ``OptionalOf``), its writer (``write_optional``) and the default
``x ?? d`` is read with (``otherwise``).
"""
from __future__ import annotations

import json
import sys
from enum import IntEnum
from pathlib import Path

import pytest

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE.parents[2] / "forge-runtime"))

from sce_forge_runtime import algorithm  # noqa: E402 -- path inserted above
from sce_runtime.event_payload import (  # noqa: E402
    OptionalOf,
    TypedPayloadError,
    Wide,
    lift,
    write_optional,
)


class _Status(IntEnum):
    """An enum as a `sce-static` machine declares one: its first variant is 0."""

    OPEN = 0
    DONE = 1

    @property
    def sce_name(self) -> str:
        return {0: "open", 1: "done"}[int(self)]


_FIELDS = (
    ("id", int, 0, (1 << 32) - 1),
    ("due", OptionalOf(Wide), -(1 << 63), (1 << 63) - 1),
    ("title", OptionalOf(str)),
    ("done", OptionalOf(bool)),
    ("level", OptionalOf(int), 0, 255),
    ("status", OptionalOf(_Status)),
)


def test_a_carried_value_is_present_however_falsy() -> None:
    delivered = '{"id": 1, "due": "0", "title": "", "done": false, "level": 0, "status": "open"}'
    assert lift(delivered, _FIELDS) == [1, 0, "", False, 0, _Status.OPEN]
    assert lift(
        {"id": 2, "due": 12, "title": "soon", "done": True, "level": 7, "status": "done"},
        _FIELDS,
    ) == [2, 12, "soon", True, 7, _Status.DONE]


def test_null_is_an_absent_value() -> None:
    delivered = {"id": 3, "due": None, "title": None, "done": None, "level": None, "status": None}
    assert lift(delivered, _FIELDS) == [3, None, None, None, None, None]
    assert lift(json.dumps(delivered), _FIELDS) == [3, None, None, None, None, None]


def test_a_missing_key_of_the_first_revision_is_refused_not_absent() -> None:
    with pytest.raises(TypedPayloadError, match="'due'"):
        lift({"id": 4, "title": "gone", "done": True, "level": 1, "status": "open"}, _FIELDS)


def test_a_present_value_is_held_to_its_type_and_width() -> None:
    present = {"id": 5, "due": 1, "title": "t", "done": True, "level": 1, "status": "open"}
    with pytest.raises(TypedPayloadError, match="'due' is not a whole number"):
        lift({**present, "due": True}, _FIELDS)
    with pytest.raises(TypedPayloadError, match="'level' does not fit the width"):
        lift({**present, "level": 300}, _FIELDS)
    with pytest.raises(TypedPayloadError, match="'status'"):
        lift({**present, "status": "lost"}, _FIELDS)


def test_a_field_that_is_never_absent_refuses_null() -> None:
    absent_id = {"id": None, "due": 1, "title": "t", "done": True, "level": 1, "status": "open"}
    with pytest.raises(TypedPayloadError, match="'id' is not a whole number"):
        lift(absent_id, _FIELDS)


def test_a_default_of_null_reads_absent_and_a_delivered_value_wins() -> None:
    delivered = {"id": 6, "due": 5, "title": "x", "done": True, "status": None}
    assert lift(delivered, _FIELDS, '{"level": null}')[4] is None
    assert lift({**delivered, "level": 9}, _FIELDS, '{"level": null}')[4] == 9


def test_the_writer_writes_null_for_absent_and_the_value_otherwise() -> None:
    assert write_optional(None, str) is None
    assert write_optional(0, str) == "0"
    assert write_optional(False, lambda v: v) is False
    assert write_optional("", lambda v: v) == ""
    assert write_optional(_Status.OPEN, lambda v: v.sce_name) == "open"
    assert json.dumps({"due": write_optional(None, str)}) == '{"due": null}'


def test_a_default_is_computed_only_when_the_value_is_absent() -> None:
    def overflow() -> int:
        raise algorithm.AlgorithmFailure(algorithm.AlgorithmError.OVERFLOW)

    assert algorithm.otherwise(0, overflow) == 0
    assert algorithm.otherwise("", overflow) == ""
    assert algorithm.otherwise(False, overflow) is False
    assert algorithm.otherwise(None, lambda: 5) == 5
    with pytest.raises(algorithm.AlgorithmFailure) as failure:
        algorithm.otherwise(None, overflow)
    assert failure.value.error is algorithm.AlgorithmError.OVERFLOW
