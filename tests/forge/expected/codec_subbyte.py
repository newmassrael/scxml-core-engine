# SCE-MAP: codec_subbyte:3 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, SceCursor, SceSink
from sce_forge_runtime.bounded import BoundedInt

from dataclasses import dataclass
from typing import Optional, Protocol


class CodecSubbytePriorityInt(BoundedInt):
    """The ``priority`` element of :class:`CodecSubbyte`, an unsigned integer of
    3 bits, held to them where a value of it is made (docs/adr/0081 §2)."""

    BITS = 3
    SIGNED = False


class CodecSubbyteChannelInt(BoundedInt):
    """The ``channel`` element of :class:`CodecSubbyte`, an unsigned integer of
    3 bits, held to them where a value of it is made (docs/adr/0081 §2)."""

    BITS = 3
    SIGNED = False


class CodecSubbyteDirectionInt(BoundedInt):
    """The ``direction`` element of :class:`CodecSubbyte`, an unsigned integer of
    2 bits, held to them where a value of it is made (docs/adr/0081 §2)."""

    BITS = 2
    SIGNED = False


class CodecSubbyteSource(Protocol):
    """The elements a value of :class:`CodecSubbyte` presents, which its encode
    reads (docs/adr/0081 §2): any object that presents them is a source of it.
    The encode derives every other element of the frame."""

    @property
    def priority(self) -> CodecSubbytePriorityInt: ...

    @property
    def channel(self) -> CodecSubbyteChannelInt: ...

    @property
    def direction(self) -> CodecSubbyteDirectionInt: ...


def encode_source(src: CodecSubbyteSource, w: SceSink) -> None:
    """Write any source of :class:`CodecSubbyte` into ``w``, each element read
    off ``src`` and written where the codec's encode plan lays it out
    (docs/adr/0081). It refuses no value; a bounded sink raises
    :class:`BufferOverflow` where it has no room left."""
    w.write_u8(((src.priority & 0x07) << 5 | (src.channel & 0x07) << 2 | (src.direction & 0x03) << 0) & 0xFF)


@dataclass
class CodecSubbyte:
    priority: CodecSubbytePriorityInt = 0
    channel: CodecSubbyteChannelInt = 0
    direction: CodecSubbyteDirectionInt = 0

    def __setattr__(self, name: str, value: object) -> None:
        # Each integer, byte string and text is held to its bound
        # wherever it is set, where the value is made and after
        # (docs/adr/0081 §2), whatever it was handed.
        if name == "priority":
            value = CodecSubbytePriorityInt(value)
        elif name == "channel":
            value = CodecSubbyteChannelInt(value)
        elif name == "direction":
            value = CodecSubbyteDirectionInt(value)
        object.__setattr__(self, name, value)

    @classmethod
    def decode(cls, cursor: SceCursor) -> CodecSubbyte:
        """Decode the next frame from ``cursor``; on success the cursor
        advances past the consumed bytes. A frame the codec refuses raises
        the ``CodecError`` subclass that names the refusal (docs/adr/0099,
        amendment of 2026-11-16): ``NeedMoreBytes`` when the cursor's tail
        is shorter than the frame (RFC §synth-5-B L494-519), which the
        caller resumes after appending bytes, and for a frame no more bytes
        make whole ``VleWidthOverflow``, ``InvalidUtf8``,
        ``UndeclaredEnumValue``, or a length or a count past its bound
        (``LengthOutOfRange``, ``TooManyElements``, docs/adr/0077)."""
        raw = cursor.peek_slice(1)
        f_priority = (raw[0] >> 5) & 0x07
        f_channel = (raw[0] >> 2) & 0x07
        f_direction = (raw[0] >> 0) & 0x03
        value = cls(
            priority=f_priority,
            channel=f_channel,
            direction=f_direction,
        )
        cursor.advance(1)
        return value

    def encode(self, w: SceSink) -> None:
        """RFC §synth-5-B encode-side primary: write ``self`` into the
        caller-owned ``w`` sink. Returns ``None`` on success; raises
        :class:`BufferOverflow` from a bounded sink when the destination
        has insufficient remaining capacity; growable sinks (e.g.
        :class:`BytearraySink`) are effectively infallible."""
        encode_source(self, w)

    def encode_to_bytes(self) -> bytes:
        """Heap-backed convenience facade. Runs :meth:`encode` over a
        :class:`BytearraySink` and returns the freshly-encoded bytes.
        Callers targeting zero-alloc hot paths should call :meth:`encode`
        directly against a caller-owned sink."""
        _dst = bytearray()
        self.encode(BytearraySink(_dst))
        return bytes(_dst)
