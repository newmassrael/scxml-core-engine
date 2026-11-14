# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""The wire of a ``vle_u<N>`` field is a ZInt64 at every width, and the type
bounds the value: a narrow field reads every encoding of a value it holds, the
padded ones included, as upstream Zenoh does, and refuses a value past its
width, never a longer encoding."""

import pytest

from sce_forge_runtime.codec import NeedMoreBytes, SceCursor, VleWidthOverflow

_NINE_BYTE_ONE = bytes([0x81, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x00])


@pytest.mark.parametrize(
    "wire, value",
    [
        (bytes([0x81, 0x80, 0x80, 0x00]), 1),
        (_NINE_BYTE_ONE, 1),
        (bytes([0xFF, 0x7F]), 0x3FFF),
        (bytes([0xFF, 0xFF, 0x80, 0x00]), 0x3FFF),
        (bytes([0x80, 0x80, 0x01]), 0x4000),
        (bytes([0x80, 0x80, 0x81, 0x80, 0x00]), 0x4000),
        (bytes([0xFF, 0xFF, 0x03]), 0xFFFF),
        (bytes([0xFF, 0xFF, 0x83, 0x80, 0x80, 0x80, 0x80, 0x80, 0x00]), 0xFFFF),
    ],
)
def test_a_u16_reads_each_encoding_of_a_value_it_holds_alike(wire, value):
    cursor = SceCursor(wire)
    assert cursor.read_vle_u16() == value
    assert cursor.remaining() == 0


def test_a_u32_and_a_u64_read_a_padded_encoding_of_a_value_they_hold():
    assert SceCursor(_NINE_BYTE_ONE).read_vle_u32() == 1
    assert SceCursor(_NINE_BYTE_ONE).read_vle_u64() == 1
    padded_max = bytes([0xFF, 0xFF, 0xFF, 0xFF, 0x8F, 0x80, 0x80, 0x80, 0x00])
    assert SceCursor(padded_max).read_vle_u32() == 0xFFFFFFFF
    assert SceCursor(bytes([0xFF] * 9)).read_vle_u64() == 0xFFFFFFFFFFFFFFFF


@pytest.mark.parametrize(
    "wire, width",
    [
        (bytes([0x80, 0x80, 0x04]), 16),
        (bytes([0x80, 0x80, 0x80, 0x80, 0x10]), 32),
        # A ninth byte with its high bit set is data, not a flag asking for a
        # tenth: 2^63, past a u32.
        (bytes([0x80] * 9), 32),
    ],
)
def test_a_narrow_field_refuses_the_first_value_past_its_width(wire, width):
    cursor = SceCursor(wire)
    read = cursor.read_vle_u16 if width == 16 else cursor.read_vle_u32
    with pytest.raises(VleWidthOverflow):
        read()


def test_a_ninth_byte_with_its_high_bit_set_is_data():
    cursor = SceCursor(bytes([0x80] * 9 + [0x01]))
    assert cursor.read_vle_u64() == 1 << 63
    assert cursor.remaining() == 1


def test_a_chain_the_buffer_ends_inside_needs_more_bytes():
    short = bytes([0x81, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80])
    with pytest.raises(NeedMoreBytes):
        SceCursor(short).read_vle_u16()
    with pytest.raises(NeedMoreBytes):
        SceCursor(short).read_vle_u64()
