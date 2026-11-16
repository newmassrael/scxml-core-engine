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
    def decode(cls, cursor: SceCursor) -> Optional[CodecLittleEndian]:
        """Decode the next frame from ``cursor``. Returns ``None`` when
        the cursor's tail is shorter than the declared minimum frame
        (RFC §synth-5-B L494-519); on success the cursor advances past the
        consumed bytes. VLE codecs also return ``None`` on
        ``VleWidthOverflow``, and every codec on a field's refusal: a length
        or a count past its bound (``LengthOutOfRange``, ``TooManyElements``,
        docs/adr/0077), an undeclared enum value."""
        try:
            raw = cursor.peek_slice(4)
            f_sensor_id = raw[0]
            f_value = raw[1] | (raw[2] << 8)
            f_status = raw[3]
        except CodecError:
            return None
        value = cls(
            sensor_id=f_sensor_id,
            value=f_value,
            status=f_status,
        )
        try:
            cursor.advance(4)
        except NeedMoreBytes:
            return None
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
