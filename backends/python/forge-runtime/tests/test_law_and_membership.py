# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""A list a codec's encode reads off a source is a value of the class of its
refinement (docs/adr/0081 §2): a list held to the law its entries say their
inputs by (docs/adr/0065 §6 R8), and a chain held to the kinds of entry an arm
of the gates on it says (R6), each made only where it holds them and unchanged
after."""

from dataclasses import dataclass
from typing import Optional

import pytest

from sce_forge_runtime.bounded import BoundedTuple
from sce_forge_runtime.codec import LengthOutOfRange, PresentIfMismatch, TooManyElements
from sce_forge_runtime.law import AfterKey, Lawful, Uniform
from sce_forge_runtime.membership import Chain, Placed


@dataclass(frozen=True)
class _Entry:
    kind: int
    says: Optional[int] = None


def _says(entry: _Entry) -> Optional[int]:
    return entry.says


class _MarkerKey:
    @staticmethod
    def matches(entry: _Entry) -> bool:
        return entry.kind == 4

    @staticmethod
    def named() -> _Entry:
        return _Entry(kind=4)


class _AtMostThree(BoundedTuple):
    LEAST = 0
    MOST = 3


class _Wide(Lawful):
    ENTRIES = _AtMostThree
    LAW = (Uniform(_says),)


class _AfterMarker(Lawful):
    ENTRIES = _AtMostThree
    LAW = (AfterKey(_says, _MarkerKey.matches),)


class _Both(Lawful):
    ENTRIES = _AtMostThree
    LAW = (Uniform(_says), AfterKey(lambda entry: None, _MarkerKey.matches))


def test_a_list_whose_entries_say_the_start_is_held_and_reads_as_its_entries():
    held = _Wide(1, [_Entry(1, 1), _Entry(2, None), _Entry(3, 1)])
    assert held.start is True
    assert len(held) == 3
    assert [entry.kind for entry in held] == [1, 2, 3]
    assert isinstance(held.entries, _AtMostThree)
    assert held == _Wide(True, (_Entry(1, 1), _Entry(2, None), _Entry(3, 1)))
    assert held != _Wide(1, [_Entry(1, 1)])


def test_an_entry_that_says_otherwise_than_the_start_is_no_value():
    with pytest.raises(PresentIfMismatch, match="_Wide"):
        _Wide(0, [_Entry(1, 0), _Entry(2, 1)])


def test_a_start_that_is_no_bit_is_refused():
    with pytest.raises(ValueError):
        _Wide(2, [])


def test_a_list_past_its_bound_is_refused_before_the_law():
    with pytest.raises(TooManyElements):
        _Wide(0, [_Entry(1)] * 4)


def test_a_list_made_without_a_start_starts_from_what_its_first_entry_says():
    assert _Wide.of([_Entry(1, 1)]).start is True
    assert _Wide.of([]).start is False


def test_an_entry_says_whether_the_one_before_was_of_the_kind():
    held = _AfterMarker(None, [_Entry(4, 0), _Entry(1, 1), _Entry(2, 0)])
    assert held.start is None
    with pytest.raises(PresentIfMismatch):
        _AfterMarker(None, [_Entry(1, 1)])
    with pytest.raises(ValueError):
        _AfterMarker(1, [])


def test_several_laws_start_from_one_start_each():
    held = _Both((1, None), [_Entry(4, 1)])
    assert held.start == (True, None)
    with pytest.raises(ValueError):
        _Both(1, [])


def test_a_lawful_list_cannot_change_after_it_is_made():
    held = _Wide(0, [])
    with pytest.raises(AttributeError):
        held._start = True


class _HasMarker(Chain):
    LEAST = 1
    MOST = 4
    HOLDS = (_MarkerKey,)


class _NoMarker(Chain):
    LEAST = 1
    MOST = 4
    LACKS = (_MarkerKey,)


def test_a_chain_holds_each_kind_its_arm_says_and_none_it_clears():
    assert _HasMarker([_Entry(1), _Entry(4)]) == (_Entry(1), _Entry(4))
    with pytest.raises(PresentIfMismatch, match="_HasMarker"):
        _HasMarker([_Entry(1)])
    assert _NoMarker([_Entry(1)]) == (_Entry(1),)
    with pytest.raises(PresentIfMismatch, match="_NoMarker"):
        _NoMarker([_Entry(4)])
    with pytest.raises(LengthOutOfRange):
        _NoMarker([])


class _Marker(Placed):
    LEAST = 1
    MOST = 3
    KEY = _MarkerKey


def test_a_bare_entrys_chain_reads_as_the_others_with_the_named_entry_at_its_place():
    placed = _Marker([_Entry(1), _Entry(2)], 1)
    assert len(placed) == 3
    assert list(placed) == [_Entry(1), _Entry(4), _Entry(2)]
    assert placed == _Marker.place_first([_Entry(1), _Entry(4), _Entry(2)])
    assert _Marker() == _Marker((), 0)


def test_a_place_after_an_entry_of_the_kind_or_past_the_others_is_refused():
    with pytest.raises(PresentIfMismatch):
        _Marker([_Entry(4)], 1)
    with pytest.raises(ValueError):
        _Marker([_Entry(1)], 2)
    with pytest.raises(TooManyElements):
        _Marker([_Entry(1)] * 3, 0)
    with pytest.raises(PresentIfMismatch):
        _Marker.place_first([_Entry(1)])
