# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""A byte string a codec's encode reads off a source is a value of the class of
its bound (docs/adr/0081 §2): made only within the bound, and unchanged after,
so the encode over it has nothing of the bound to refuse."""

import pytest

from sce_forge_runtime.bounded import BoundedBytes, BoundedText
from sce_forge_runtime.codec import InvalidUtf8, LengthOutOfRange


class _AtMostThree(BoundedBytes):
    LEAST = 0
    MOST = 3


class _OneOrMore(BoundedBytes):
    LEAST = 1
    MOST = None


@pytest.mark.parametrize("data", [b"", b"a", b"abc", bytearray(b"ab"), [1, 2, 3]])
def test_bytes_within_the_bound_are_made_and_equal_what_they_were_made_of(data):
    held = _AtMostThree(data)
    assert isinstance(held, bytes)
    assert held == bytes(data)
    assert len(held) == len(bytes(data))


def test_bytes_past_the_most_are_no_value_of_the_class():
    with pytest.raises(LengthOutOfRange) as refused:
        _AtMostThree(b"abcd")
    assert str(refused.value) == "_AtMostThree"


def test_bytes_short_of_the_least_are_no_value_of_the_class():
    with pytest.raises(LengthOutOfRange):
        _OneOrMore(b"")
    assert _OneOrMore(bytes(1)) == b"\x00"


def test_a_class_with_no_most_holds_any_length_past_its_least():
    assert len(_OneOrMore(bytes(100_000))) == 100_000


def test_the_default_is_no_bytes():
    assert _AtMostThree() == b""


def test_a_count_is_not_read_as_bytes():
    with pytest.raises(TypeError):
        _AtMostThree(2)


def test_a_value_made_again_of_a_value_of_its_class_is_equal_to_it():
    held = _AtMostThree(b"ab")
    assert _AtMostThree(held) == held
    assert type(_AtMostThree(held)) is _AtMostThree


class _TextOfFour(BoundedText):
    LEAST = 0
    MOST = 4


def test_a_text_is_counted_in_the_utf8_bytes_it_encodes_to():
    assert _TextOfFour("abcd") == "abcd"
    assert _TextOfFour("éé") == "éé"
    with pytest.raises(LengthOutOfRange):
        _TextOfFour("é€")
    with pytest.raises(LengthOutOfRange):
        _TextOfFour("abcde")


def test_a_text_no_utf8_holds_is_refused_as_invalid():
    with pytest.raises(InvalidUtf8):
        _TextOfFour("\ud800")


def test_a_text_is_made_of_a_text_alone():
    with pytest.raises(TypeError):
        _TextOfFour(b"ab")
