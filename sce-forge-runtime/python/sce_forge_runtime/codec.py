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
    """Base class for typed codec decode errors."""


class NeedMoreBytes(CodecError):
    """Raised when the cursor's remaining buffer is shorter than the
    codec's declared minimum frame. Caller should resume after
    appending more bytes."""


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
