# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

"""Codec cursor + typed error contract for sce:kind="codec" decode bodies.

Mirrors ``sce-forge-runtime/rust/src/codec.rs``. RFC §5.B L494-519 pins a
per-language cursor + ``NeedMoreBytes`` contract on decode so a truncated
input never aborts.

Phase B1-prep ships peek_slice / advance / remaining. Streaming readers
(read_u8, read_vle_*, read_tag) land in B1-α/β with their first
consumer.
"""

from __future__ import annotations


class CodecError(Exception):
    """Base class for typed codec decode errors.

    The B1-β variant primitive intentionally does NOT need a typed
    ``UnknownVariantTag`` — RFC §5.B requires ``<sce:default>`` when
    arms don't exhaust the tag domain (build-time
    ``codec/variant-arm-unreachable`` otherwise), so the default arm
    catches every unmatched tag at runtime.

    """


class NeedMoreBytes(CodecError):
    """Raised when the cursor's remaining buffer is shorter than the
    codec's declared minimum frame. Caller should resume after
    appending more bytes."""


class VleWidthOverflow(CodecError):
    """Raised when a vle_u<N> field's continuation chain implies a value
    wider than the declared type. RFC §5.B
    ``codec/vle-width-overflow``."""


class InvalidUtf8(CodecError):
    """Raised when a ``sce:type="string"`` field's length-prefixed
    payload is not well-formed UTF-8. RFC §5.B B5-ζ Surface H. Forge-
    fail-fast contract — zenoh-pico itself aliases the bytes without
    validating, but SCE-side codecs reject malformed text early so
    downstream procedures never see a malformed ``str``. The Cpp +
    Kotlin runtimes collapse this to their existing truncation
    sentinel (``std::nullopt`` / ``null``) instead, mirroring the
    VleWidthOverflow declaration-only convention there."""


class TlvChainOverflow(CodecError):
    """Raised when a ``<sce:tlv-chain on-overflow="reject">`` field has
    residual cursor bytes after ``max_depth`` entries have been
    consumed (RFC §5.B B3-α). Truncate policy silently drops the
    residual bytes and never raises this exception; the on-overflow
    attribute is parser-mandatory so the codec emit always picks one
    of the two policies. The Cpp + Kotlin runtimes collapse this to
    their truncation sentinel (mirrors VleWidthOverflow declaration-
    only convention)."""


class SceCursor:
    """Read-only cursor over a borrowed input ``bytes`` / ``memoryview``.

    Decode bodies use ``peek_slice`` to bounds-check + read fixed-offset
    bytes positionally, then ``advance`` after the construction succeeds.
    """

    __slots__ = ("_buf", "_pos")

    def __init__(self, buf: bytes) -> None:
        self._buf = buf
        self._pos = 0

    def remaining(self) -> int:
        return len(self._buf) - self._pos

    def peek_slice(self, n: int) -> bytes:
        """Borrow the next ``n`` bytes without advancing.

        Raises ``NeedMoreBytes`` when the cursor's tail is shorter than
        ``n``.
        """
        if self.remaining() < n:
            raise NeedMoreBytes()
        return self._buf[self._pos : self._pos + n]

    def advance(self, n: int) -> None:
        """Advance the cursor by ``n`` bytes.

        Raises ``NeedMoreBytes`` if ``n`` would overrun the buffer.
        """
        if self.remaining() < n:
            raise NeedMoreBytes()
        self._pos += n

    def _read_vle_inner(self, max_bits: int) -> int:
        """Read a base-128 variable-length encoded unsigned value of up
        to ``max_bits`` payload width. LSB-first byte order; bit 7 is
        the continuation flag. Mirrors Zenoh ZInt (RFC §5.B Appendix B).
        """
        max_bytes = (max_bits + 6) // 7
        value = 0
        shift = 0
        for _ in range(max_bytes):
            if self.remaining() < 1:
                raise NeedMoreBytes()
            b = self._buf[self._pos]
            self._pos += 1
            payload = b & 0x7F
            if shift + 7 > max_bits:
                allowed = max_bits - shift
                max_payload = (1 << allowed) - 1
                if payload > max_payload:
                    raise VleWidthOverflow()
            value |= payload << shift
            if (b & 0x80) == 0:
                return value
            shift += 7
        raise VleWidthOverflow()

    def read_vle_u16(self) -> int:
        """Read a vle_u16 field (1-3 wire bytes)."""
        return self._read_vle_inner(16)

    def read_vle_u32(self) -> int:
        """Read a vle_u32 field (1-5 wire bytes)."""
        return self._read_vle_inner(32)

    def read_vle_u64(self) -> int:
        """Read a vle_u64 field (1-10 wire bytes). Canonical Zenoh ZInt."""
        return self._read_vle_inner(64)
