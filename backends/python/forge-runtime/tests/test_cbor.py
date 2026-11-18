# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

"""sce_forge_runtime.cbor — the properties the Rust runtime's own tests pin
(backends/rust/forge-runtime/src/cbor.rs): shortest heads and floats on
write, every well-formed definite-length encoding of a value on read, a signed
integer and a float at every head and precision boundary (docs/adr/0101,
"Conformance" item 2), an indefinite length refused at any depth, each
refusal named by its class, and a skip that takes an unknown value whole and
refuses one too deep. What a generated codec does with these is the
conformance harness's (codec_cbor_map), which this file does not repeat."""

import math
import struct

import pytest

from sce_forge_runtime import cbor
from sce_forge_runtime.codec import BytearraySink, InvalidUtf8, SceCursor


def _written(write) -> bytes:
    out = bytearray()
    write(BytearraySink(out))
    return bytes(out)


def _read_whole(data: bytes, read):
    c = SceCursor(data)
    value = read(c)
    assert c.remaining() == 0, f"{data.hex()} left bytes unread"
    return value


def _int64(c: SceCursor) -> int:
    return cbor.read_int_within(c, -(1 << 63), (1 << 63) - 1)


@pytest.mark.parametrize(
    "write, want",
    [
        # RFC 8949 Appendix A: 0, 23, 24, 255, 256, 65535, 65536, 2^32.
        (lambda w: cbor.write_uint(w, 0), b"\x00"),
        (lambda w: cbor.write_uint(w, 23), b"\x17"),
        (lambda w: cbor.write_uint(w, 24), b"\x18\x18"),
        (lambda w: cbor.write_uint(w, 255), b"\x18\xff"),
        (lambda w: cbor.write_uint(w, 256), b"\x19\x01\x00"),
        (lambda w: cbor.write_uint(w, 65535), b"\x19\xff\xff"),
        (lambda w: cbor.write_uint(w, 65536), b"\x1a\x00\x01\x00\x00"),
        (lambda w: cbor.write_uint(w, 1 << 32), b"\x1b\x00\x00\x00\x01\x00\x00\x00\x00"),
        (lambda w: cbor.write_text(w, "a"), b"\x61a"),
        (lambda w: cbor.write_bytes(w, b"\x01\x02"), b"\x42\x01\x02"),
        (lambda w: cbor.write_bool(w, True), b"\xf5"),
        (lambda w: cbor.write_map_head(w, 2), b"\xa2"),
        (lambda w: cbor.write_array_head(w, 2), b"\x82"),
        (lambda w: cbor.write_array_head(w, 24), b"\x98\x18"),
    ],
)
def test_every_head_is_written_in_its_shortest_form(write, want):
    assert _written(write) == want


@pytest.mark.parametrize(
    "value, want",
    [
        (0, "00"),
        (23, "17"),
        (24, "1818"),
        (255, "18ff"),
        (256, "190100"),
        (65535, "19ffff"),
        (65536, "1a00010000"),
        ((1 << 32) - 1, "1affffffff"),
        (1 << 32, "1b0000000100000000"),
        ((1 << 63) - 1, "1b7fffffffffffffff"),
        (-1, "20"),
        (-24, "37"),
        (-25, "3818"),
        (-256, "38ff"),
        (-257, "390100"),
        (-500, "3901f3"),
        (-65536, "39ffff"),
        (-65537, "3a00010000"),
        (-(1 << 32), "3affffffff"),
        (-(1 << 32) - 1, "3b0000000100000000"),
        (-(1 << 63), "3b7fffffffffffffff"),
    ],
)
def test_a_signed_integer_is_written_in_the_shortest_head_of_its_sign(value, want):
    data = bytes.fromhex(want)
    assert _written(lambda w: cbor.write_int(w, value)) == data
    assert _read_whole(data, _int64) == value


def test_a_signed_integer_is_read_from_a_longer_head_and_held_to_its_type():
    # -1 with its argument in a one-byte head, and 1 in an eight-byte one.
    assert _read_whole(b"\x38\x00", _int64) == -1
    assert _read_whole(b"\x1b\x00\x00\x00\x00\x00\x00\x00\x01", _int64) == 1

    def int8(c):
        return cbor.read_int_within(c, -128, 127)

    assert _read_whole(b"\x38\x7f", int8) == -128
    assert _read_whole(b"\x18\x7f", int8) == 127
    for data in (b"\x38\x80", b"\x18\x80"):
        with pytest.raises(cbor.CborOutOfRange):
            int8(SceCursor(data))
    # An int64 holds neither -2^63 - 1 nor 2^63.
    for data in (b"\x3b\x80" + bytes(7), b"\x1b\x80" + bytes(7)):
        with pytest.raises(cbor.CborOutOfRange):
            _int64(SceCursor(data))
    # A text, a float and a tagged item are no integer.
    for data in (b"\x61a", b"\xf9\x3c\x00", b"\xc2\x41\x01"):
        with pytest.raises(cbor.CborWrongType):
            _int64(SceCursor(data))


def test_an_unsigned_type_takes_no_negative_integer_and_no_bignum():
    for data in (b"\x20", b"\xc2\x41\x01", b"\x61a"):
        with pytest.raises(cbor.CborWrongType):
            cbor.read_uint(SceCursor(data))


@pytest.mark.parametrize(
    "value, want",
    [
        (0.0, "f90000"),
        (-0.0, "f98000"),
        (1.0, "f93c00"),
        (1.5, "f93e00"),
        # The largest half, the least normal half and the least subnormal half.
        (65504.0, "f97bff"),
        (2.0**-14, "f90400"),
        (2.0**-24, "f90001"),
        (math.inf, "f97c00"),
        (-math.inf, "f9fc00"),
        # Past a half's range, or between its values: a single holds it.
        (65536.0, "fa47800000"),
        (100000.0, "fa47c35000"),
        (3.4028234663852886e38, "fa7f7fffff"),
        # Only a double holds it.
        (1.1, "fb3ff199999999999a"),
        (1.0e300, "fb7e37e43c8800759c"),
    ],
)
def test_a_float_is_written_in_the_shortest_precision_that_holds_it_exactly(value, want):
    data = bytes.fromhex(want)
    assert _written(lambda w: cbor.write_float(w, value)) == data
    read = _read_whole(data, cbor.read_float)
    assert struct.pack(">d", read) == struct.pack(">d", value)


def test_every_nan_is_written_as_the_one_nan():
    signalling = struct.unpack(">d", bytes.fromhex("7ff0000000000001"))[0]
    for nan in (math.nan, -math.nan, signalling):
        assert _written(lambda w: cbor.write_float(w, nan)) == b"\xf9\x7e\x00"
    # A float32 is written as the double it is exactly: 0.1 as a single.
    single = struct.unpack(">f", struct.pack(">f", 0.1))[0]
    assert _written(lambda w: cbor.write_float(w, single)) == bytes.fromhex("fa3dcccccd")


def test_a_float_is_read_from_a_wider_precision_than_it_needs():
    # 1.5 as a single and as a double.
    assert _read_whole(bytes.fromhex("fa3fc00000"), cbor.read_float) == 1.5
    assert _read_whole(bytes.fromhex("fb3ff8000000000000"), cbor.read_float) == 1.5
    # A float32 given as the double that is exactly 0.1 as a single, and as a half.
    single = struct.unpack(">f", struct.pack(">f", 0.1))[0]
    assert _read_whole(bytes.fromhex("fb3fb99999a0000000"), cbor.read_float32) == single
    assert _read_whole(bytes.fromhex("f93e00"), cbor.read_float32) == 1.5
    # A NaN of any precision is a NaN of either type.
    assert math.isnan(_read_whole(bytes.fromhex("f97e00"), cbor.read_float))
    assert math.isnan(_read_whole(bytes.fromhex("fb7ff8000000000000"), cbor.read_float32))


def test_a_float_that_no_single_is_is_no_float32_and_an_integer_is_no_float():
    # 0.1 as a double is between two singles.
    with pytest.raises(cbor.CborOutOfRange):
        cbor.read_float32(SceCursor(bytes.fromhex("fb3fb999999999999a")))
    for data in (b"\x01", b"\xf5", b"\xf6"):
        with pytest.raises(cbor.CborWrongType):
            cbor.read_float(SceCursor(data))


def test_a_truth_value_is_neither_null_nor_a_float_and_a_small_simple_value_has_one_form():
    assert cbor.read_bool(SceCursor(b"\xf4")) is False
    assert cbor.read_bool(SceCursor(b"\xf5")) is True
    # `null`, a half whose bits are 20, and the simple value 32.
    for data in (b"\xf6", b"\xf9\x00\x14", b"\xf8\x20"):
        with pytest.raises(cbor.CborWrongType):
            cbor.read_bool(SceCursor(data))
    # `false` in the two-byte form, and a break code standing alone: not
    # well-formed.
    for data in (b"\xf8\x14", b"\xff"):
        with pytest.raises(cbor.CborMalformed):
            cbor.read_bool(SceCursor(data))


def test_a_head_is_read_in_any_valid_length():
    # 1 written in its 2-byte form still reads as 1.
    assert cbor.read_uint(SceCursor(b"\x18\x01")) == 1
    assert cbor.read_uint(SceCursor(b"\x1b\x00\x00\x00\x01\x00\x00\x00\x00")) == 1 << 32


def test_an_array_head_is_read_as_its_count_and_no_other_item_is():
    assert cbor.read_array_len(SceCursor(b"\x83")) == 3
    assert cbor.read_array_len(SceCursor(b"\x98\x18")) == 24
    # An indefinite-length array, outside the profile.
    with pytest.raises(cbor.CborMalformed):
        cbor.read_array_len(SceCursor(b"\x9f"))
    # A map where an array stands.
    with pytest.raises(cbor.CborWrongType):
        cbor.read_array_len(SceCursor(b"\xa1"))


def test_what_the_codec_cannot_read_is_refused():
    # Indefinite-length map, reserved additional information: not a head the
    # codec reads.
    for data in (b"\xbf", b"\x1c"):
        with pytest.raises(cbor.CborMalformed):
            cbor.read_uint(SceCursor(data))
    with pytest.raises(cbor.CborMalformed):
        cbor.read_map_len(SceCursor(b"\xbf"))
    # Another major type, a width the entry cannot hold, a wrong exact
    # length, a text past its bound, a text that is not UTF-8.
    with pytest.raises(cbor.CborWrongType):
        cbor.read_uint(SceCursor(b"\x61a"))
    with pytest.raises(cbor.CborOutOfRange):
        cbor.read_uint_upto(SceCursor(b"\x19\x01\x00"), 255)
    with pytest.raises(cbor.CborWrongLength):
        cbor.read_bytes_exact(SceCursor(b"\x42\x01\x02"), 16)
    with pytest.raises(cbor.CborOutOfRange):
        cbor.read_text(SceCursor(b"\x62ab"), 1)
    with pytest.raises(InvalidUtf8):
        cbor.read_text(SceCursor(b"\x61\xff"), None)


def test_an_indefinite_length_is_outside_the_profile_at_any_depth():
    # A text in chunks, a byte string in chunks.
    with pytest.raises(cbor.CborMalformed):
        cbor.read_text(SceCursor(bytes.fromhex("7f61616162ff")), None)
    with pytest.raises(cbor.CborMalformed):
        cbor.read_bytes(SceCursor(bytes.fromhex("5f4101ff")), None)
    # An indefinite array skipped, and an indefinite text inside a map skipped.
    for data in (bytes.fromhex("9f0102ff"), bytes.fromhex("a1017f6161ff")):
        with pytest.raises(cbor.CborMalformed):
            cbor.skip(SceCursor(data))


def test_a_union_is_a_map_of_one_entry_keyed_by_its_variant_name():
    cases = ["card", "cash", "voucher"]
    # `cash`, a case that holds no record.
    cash = bytes.fromhex("a16463617368a0")

    def write(w):
        cbor.write_variant(w, "cash")
        cbor.write_map_head(w, 0)

    assert _written(write) == cash
    c = SceCursor(cash)
    assert cbor.read_variant(c, cases) == 1
    assert cbor.read_map_len(c) == 0
    assert c.remaining() == 0


@pytest.mark.parametrize(
    "data, kind",
    [
        # No entry, and two entries refused before the key is read: no key
        # follows.
        ("a0", cbor.CborUnionEntries),
        ("a2", cbor.CborUnionEntries),
        # A name no case declares.
        ("a164636f696ea0", cbor.CborUnknownVariant),
        # A key that is not a text, and an item that is no map.
        ("a101a0", cbor.CborWrongType),
        ("80", cbor.CborWrongType),
        # An indefinite union map.
        ("bf6463617368a0ff", cbor.CborMalformed),
    ],
)
def test_a_union_map_is_judged_for_one_entry_then_for_its_key(data, kind):
    with pytest.raises(kind):
        cbor.read_variant(SceCursor(bytes.fromhex(data)), ["card", "cash", "voucher"])


def test_an_unknown_value_is_skipped_whole_and_a_deep_one_refused():
    # {1: [2, {3: h'00'}]} then 7: the skip lands on the 7.
    c = SceCursor(b"\xa1\x01\x82\x02\xa1\x03\x41\x00\x07")
    cbor.skip(c)
    assert cbor.read_uint(c) == 7
    with pytest.raises(cbor.CborTooDeep):
        cbor.skip(SceCursor(b"\x81" * 20 + b"\x00"))
