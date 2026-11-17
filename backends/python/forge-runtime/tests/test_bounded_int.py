# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""An integer a codec's encode reads off a source is a value of the class of its
field's width (docs/adr/0081 §2): made only within the width and signedness,
so no encode writes the field's bits of a number and loses the rest."""

import pytest

from sce_forge_runtime.bounded import BoundedInt


class _ThreeBits(BoundedInt):
    BITS = 3
    SIGNED = False


class _SignedByte(BoundedInt):
    BITS = 8
    SIGNED = True


@pytest.mark.parametrize("value", [0, 1, 7])
def test_a_number_within_the_bits_is_made_and_equals_what_it_was_made_of(value):
    held = _ThreeBits(value)
    assert isinstance(held, int)
    assert held == value


@pytest.mark.parametrize("value", [8, 255, -1])
def test_a_number_past_the_unsigned_bits_is_no_value_of_the_class(value):
    with pytest.raises(OverflowError):
        _ThreeBits(value)


@pytest.mark.parametrize("value", [-128, -1, 0, 127])
def test_a_signed_class_holds_its_twos_complement_span(value):
    assert _SignedByte(value) == value


@pytest.mark.parametrize("value", [-129, 128, 255])
def test_a_number_past_the_signed_span_is_no_value_of_the_class(value):
    with pytest.raises(OverflowError):
        _SignedByte(value)


def test_the_span_is_the_least_and_the_most_number_held():
    assert _ThreeBits.span() == (0, 7)
    assert _SignedByte.span() == (-128, 127)
    assert BoundedInt.span() == (0, (1 << 64) - 1)


def test_a_number_of_another_kind_is_refused_and_a_bool_is_an_int():
    with pytest.raises(TypeError):
        _ThreeBits(1.0)
    with pytest.raises(TypeError):
        _ThreeBits("1")
    assert _ThreeBits(True) == 1


def test_the_default_is_zero():
    assert _ThreeBits() == 0


def test_the_wire_bits_of_a_signed_field_read_as_its_twos_complement():
    assert _SignedByte.from_wire(0xFB) == -5
    assert _SignedByte.from_wire(0x7F) == 127
    assert _SignedByte.from_wire(0x80) == -128
    assert _ThreeBits.from_wire(7) == 7
    assert isinstance(_SignedByte.from_wire(0xFB), _SignedByte)


@pytest.mark.parametrize("bits", [-1, 0x100])
def test_wire_bits_past_the_field_are_refused(bits):
    with pytest.raises(OverflowError):
        _SignedByte.from_wire(bits)


def test_arithmetic_answers_a_plain_int_that_the_holder_holds_again():
    total = _ThreeBits(7) + 1
    assert type(total) is int
    with pytest.raises(OverflowError):
        _ThreeBits(total)
