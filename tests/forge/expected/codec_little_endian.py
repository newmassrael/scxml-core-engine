# SCE-MAP: codec_little_endian:3 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, SceCursor, SceSink

from dataclasses import dataclass
from typing import Optional, Protocol


class CodecLittleEndianSource(Protocol):
    """The elements a value of :class:`CodecLittleEndian` presents, which its encode
    reads (docs/adr/0081 §2): any object that presents them is a source of it.
    The encode derives every other element of the frame."""

    @property
    def sensor_id(self) -> int: ...

    @property
    def value(self) -> int: ...

    @property
    def status(self) -> int: ...


def encode_source(src: CodecLittleEndianSource, w: SceSink) -> None:
    """Write any source of :class:`CodecLittleEndian` into ``w``, each element read
    off ``src`` and written where the codec's encode plan lays it out
    (docs/adr/0081). It refuses no value; a bounded sink raises
    :class:`BufferOverflow` where it has no room left."""
    w.write_u8(src.sensor_id & 0xFF)
    w.write_u8(src.value & 0xFF)
    w.write_u8((src.value >> 8) & 0xFF)
    w.write_u8(src.status & 0xFF)


@dataclass
class CodecLittleEndian:
    sensor_id: int = 0
    value: int = 0
    status: int = 0

    @classmethod
    def decode(cls, cursor: SceCursor) -> CodecLittleEndian:
        """Decode the next frame from ``cursor``; on success the cursor
        advances past the consumed bytes. A frame the codec refuses raises
        the ``CodecError`` subclass that names the refusal (docs/adr/0099,
        amendment of 2026-11-16): ``NeedMoreBytes`` when the cursor's tail
        is shorter than the frame (RFC §synth-5-B L494-519), which the
        caller resumes after appending bytes, and for a frame no more bytes
        make whole ``VleWidthOverflow``, ``InvalidUtf8``,
        ``UndeclaredEnumValue``, or a length or a count past its bound
        (``LengthOutOfRange``, ``TooManyElements``, docs/adr/0077)."""
        raw = cursor.peek_slice(4)
        f_sensor_id = raw[0]
        f_value = raw[1] | (raw[2] << 8)
        f_status = raw[3]
        value = cls(
            sensor_id=f_sensor_id,
            value=f_value,
            status=f_status,
        )
        cursor.advance(4)
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
