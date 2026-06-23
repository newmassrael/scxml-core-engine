# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

"""Codec cursor + typed error contract for sce:kind="codec" decode bodies.

Mirrors ``sce-forge-runtime/rust/src/codec.rs``. RFC §5.B L494-519 pins a
per-language cursor + ``NeedMoreBytes`` contract on decode so a truncated
input never aborts.

The cursor ships peek_slice / advance / remaining plus the read_vle_*
readers. Other streaming readers (e.g. a dedicated read_u8 / read_tag)
are not provided until a consumer needs them.
"""

from __future__ import annotations


class CodecError(Exception):
    """Base class for typed codec decode errors.

    The variant primitive intentionally does NOT need a typed
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
    payload is not well-formed UTF-8 (RFC §5.B). Forge-
    fail-fast contract — zenoh-pico itself aliases the bytes without
    validating, but SCE-side codecs reject malformed text early so
    downstream procedures never see a malformed ``str``. The Cpp +
    Kotlin runtimes collapse this to their existing truncation
    sentinel (``std::nullopt`` / ``null``) instead, mirroring the
    VleWidthOverflow declaration-only convention there."""


class TlvChainOverflow(CodecError):
    """Raised when a ``<sce:tlv-chain on-overflow="reject">`` field has
    residual cursor bytes after ``max_depth`` entries have been
    consumed (RFC §5.B). Truncate policy silently drops the
    residual bytes and never raises this exception; the on-overflow
    attribute is parser-mandatory so the codec emit always picks one
    of the two policies. The Cpp + Kotlin runtimes collapse this to
    their truncation sentinel (mirrors VleWidthOverflow declaration-
    only convention)."""


class BufferOverflow(CodecError):
    """RFC §5.B encode-side counterpart to :class:`NeedMoreBytes`:
    raised when an encode write would exceed the destination sink's
    remaining capacity. Only the bounded :class:`MemoryviewSink`
    (caller-owned ``memoryview`` + ``cap``) can raise this; the
    growable :class:`BytearraySink` is effectively infallible."""


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
        to ``max_bits`` payload width. LSB-first byte order; leading
        bytes use bit 7 as the continuation flag, while the final byte
        (at shift ``7 * (VLE_LEN - 1)``) carries a full 8 data bits with
        no flag. Canonical Zenoh ZInt (RFC §5.B Appendix B):
        ceil((W-1)/7) bytes max, so a u64 caps at 9 bytes, not 10.
        """
        vle_len = (max_bits - 1 + 6) // 7
        final_shift = 7 * (vle_len - 1)
        value = 0
        shift = 0
        for _ in range(vle_len):
            if self.remaining() < 1:
                raise NeedMoreBytes()
            b = self._buf[self._pos]
            self._pos += 1
            if shift == final_shift:
                # Final byte: 8 data bits, continuation bit reused as
                # data. For a sub-octet tail (u16 / u32) refuse overflow.
                allowed = max_bits - shift
                if allowed < 8 and b > (1 << allowed) - 1:
                    raise VleWidthOverflow()
                value |= b << shift
                return value
            value |= (b & 0x7F) << shift
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
        """Read a vle_u64 field (1-9 wire bytes). Canonical Zenoh ZInt."""
        return self._read_vle_inner(64)


# ── Write-side sink ──────────────────────────────────────────────


import abc as _abc
from typing import Union as _Union


class SceSink(_abc.ABC):
    """RFC §5.B encode-side sink. Generated ``encode`` bodies
    append bytes through this surface.

    Concrete sinks own the destination storage (``bytearray`` or
    ``memoryview``); the sink only holds a reference. Failure mode:
    bounded sinks raise :class:`BufferOverflow`; growable sinks never
    raise. ``position()`` returns bytes written by this sink instance
    since construction (delta — not absolute destination length) so
    coalesced-send paths stay positionally consistent.

    Subclasses MUST implement ``write_bytes`` and ``position``; the
    per-width helpers (``write_u8`` / ``write_u16_le`` / …) are
    default-implemented in terms of ``write_bytes``.

    Defined as an :class:`abc.ABC` rather than a :class:`Protocol`
    so codegen can ``isinstance``-check at runtime without
    ``runtime_checkable`` ceremony, matching the cpp/kotlin/go
    interface conventions on the same boundary.
    """

    @_abc.abstractmethod
    def write_bytes(self, data: _Union[bytes, bytearray, memoryview]) -> None:
        """Append ``data`` to the underlying storage. Raises
        :class:`BufferOverflow` when a bounded destination has
        insufficient remaining capacity."""

    @_abc.abstractmethod
    def position(self) -> int:
        """Bytes written by this sink instance since construction."""

    def write_u8(self, v: int) -> None:
        """Append a single byte (``0 ≤ v < 256``)."""
        self.write_bytes(bytes((v & 0xFF,)))

    def _write_vle_inner(self, value: int, max_bits: int) -> None:
        """Append ``value`` as a base-128 VLE of ``max_bits`` payload
        width. The write-side counterpart of
        :meth:`SceCursor._read_vle_inner`: leading bytes carry 7 data
        bits + a continuation flag; the final byte (after at most
        VLE_LEN-1 continuation bytes) carries a full 8 data bits with no
        flag, so a u64 caps at 9 bytes — canonical Zenoh ZInt (RFC
        §synth-5-B Appendix B). VLE_LEN = ceil((max_bits-1)/7)."""
        cont_max = (max_bits - 1 + 6) // 7 - 1
        v = value
        n = 0
        while v >= 0x80 and n < cont_max:
            self.write_u8((v & 0x7F) | 0x80)
            v >>= 7
            n += 1
        self.write_u8(v)

    def write_vle_u16(self, v: int) -> None:
        """Append a vle_u16 field (1-3 wire bytes)."""
        self._write_vle_inner(v, 16)

    def write_vle_u32(self, v: int) -> None:
        """Append a vle_u32 field (1-5 wire bytes)."""
        self._write_vle_inner(v, 32)

    def write_vle_u64(self, v: int) -> None:
        """Append a vle_u64 field (1-9 wire bytes). Canonical Zenoh ZInt."""
        self._write_vle_inner(v, 64)

    def write_u16_le(self, v: int) -> None:
        self.write_bytes(bytes(((v & 0xFF), ((v >> 8) & 0xFF))))

    def write_u16_be(self, v: int) -> None:
        self.write_bytes(bytes((((v >> 8) & 0xFF), (v & 0xFF))))

    def write_u32_le(self, v: int) -> None:
        self.write_bytes(bytes((
            (v & 0xFF), ((v >> 8) & 0xFF), ((v >> 16) & 0xFF), ((v >> 24) & 0xFF),
        )))

    def write_u32_be(self, v: int) -> None:
        self.write_bytes(bytes((
            ((v >> 24) & 0xFF), ((v >> 16) & 0xFF), ((v >> 8) & 0xFF), (v & 0xFF),
        )))

    def write_u64_le(self, v: int) -> None:
        self.write_bytes(bytes((
            (v & 0xFF), ((v >> 8) & 0xFF), ((v >> 16) & 0xFF), ((v >> 24) & 0xFF),
            ((v >> 32) & 0xFF), ((v >> 40) & 0xFF), ((v >> 48) & 0xFF), ((v >> 56) & 0xFF),
        )))

    def write_u64_be(self, v: int) -> None:
        self.write_bytes(bytes((
            ((v >> 56) & 0xFF), ((v >> 48) & 0xFF), ((v >> 40) & 0xFF), ((v >> 32) & 0xFF),
            ((v >> 24) & 0xFF), ((v >> 16) & 0xFF), ((v >> 8) & 0xFF), (v & 0xFF),
        )))


class BytearraySink(SceSink):
    """Growable sink backed by a caller-owned :class:`bytearray`.
    Infallible (``write_bytes`` never raises :class:`BufferOverflow`).
    Natural sink behind ``encode_to_bytes()`` facades."""

    __slots__ = ("_dst", "_start_len")

    def __init__(self, dst: bytearray) -> None:
        self._dst = dst
        self._start_len = len(dst)

    def write_bytes(self, data: _Union[bytes, bytearray, memoryview]) -> None:
        if len(data) == 0:
            return
        self._dst.extend(data)

    def write_u8(self, v: int) -> None:
        self._dst.append(v & 0xFF)

    def position(self) -> int:
        return len(self._dst) - self._start_len


class MemoryviewSink(SceSink):
    """Bounded sink backed by a caller-owned :class:`memoryview` + write
    position. Raises :class:`BufferOverflow` when a write would exceed
    the view's length. Natural sink for fixed-frame / DMA-aligned
    call sites."""

    __slots__ = ("_buf", "_pos", "_start_pos")

    def __init__(self, buf: memoryview, pos: int = 0) -> None:
        if buf.format != "B" and buf.itemsize != 1:
            raise TypeError("MemoryviewSink requires a byte-wide memoryview")
        self._buf = buf
        self._pos = pos
        self._start_pos = pos

    def write_bytes(self, data: _Union[bytes, bytearray, memoryview]) -> None:
        n = len(data)
        if n == 0:
            return
        if len(self._buf) - self._pos < n:
            raise BufferOverflow()
        self._buf[self._pos : self._pos + n] = data
        self._pos += n

    def write_u8(self, v: int) -> None:
        if self._pos >= len(self._buf):
            raise BufferOverflow()
        self._buf[self._pos] = v & 0xFF
        self._pos += 1

    def position(self) -> int:
        return self._pos - self._start_pos

    def remaining(self) -> int:
        """Remaining capacity from current position to end of view."""
        return len(self._buf) - self._pos
