# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

"""sce_forge_runtime.cbor — the same four properties the Rust runtime's own
tests pin (backends/rust/forge-runtime/src/cbor.rs): shortest heads on write,
any valid head length on read, refusal of what an entry cannot be, and a skip
that takes an unknown value whole and refuses one too deep. What a generated
codec does with these is the conformance harness's (codec_cbor_map), which
this file does not repeat."""

import pytest

from sce_forge_runtime import cbor
from sce_forge_runtime.codec import BytearraySink, InvalidUtf8, SceCursor


def _written(write) -> bytes:
    out = bytearray()
    write(BytearraySink(out))
    return bytes(out)


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
    ],
)
def test_every_head_is_written_in_its_shortest_form(write, want):
    assert _written(write) == want


def test_a_head_is_read_in_any_valid_length():
    # 1 written in its 2-byte form still reads as 1.
    assert cbor.read_uint(SceCursor(b"\x18\x01")) == 1
    assert cbor.read_uint(SceCursor(b"\x1b\x00\x00\x00\x01\x00\x00\x00\x00")) == 1 << 32


def test_what_the_codec_cannot_read_is_refused():
    # Indefinite-length map, reserved additional information, another major
    # type, a width the entry cannot hold, a wrong exact length, a text past
    # its bound, a text that is not UTF-8.
    for data in (b"\xbf", b"\x1c", b"\x61a"):
        with pytest.raises(cbor.CborMalformed):
            cbor.read_uint(SceCursor(data))
    with pytest.raises(cbor.CborMalformed):
        cbor.read_map_len(SceCursor(b"\xbf"))
    with pytest.raises(cbor.CborOutOfRange):
        cbor.read_uint_upto(SceCursor(b"\x19\x01\x00"), 255)
    with pytest.raises(cbor.CborWrongLength):
        cbor.read_bytes_exact(SceCursor(b"\x42\x01\x02"), 16)
    with pytest.raises(cbor.CborOutOfRange):
        cbor.read_text(SceCursor(b"\x62ab"), 1)
    with pytest.raises(InvalidUtf8):
        cbor.read_text(SceCursor(b"\x61\xff"), None)


def test_an_unknown_value_is_skipped_whole_and_a_deep_one_refused():
    # {1: [2, {3: h'00'}]} then 7: the skip lands on the 7.
    c = SceCursor(b"\xa1\x01\x82\x02\xa1\x03\x41\x00\x07")
    cbor.skip(c)
    assert cbor.read_uint(c) == 7
    with pytest.raises(cbor.CborTooDeep):
        cbor.skip(SceCursor(b"\x81" * 20 + b"\x00"))
