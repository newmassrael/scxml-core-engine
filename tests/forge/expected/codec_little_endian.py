# SCE-MAP: codec_little_endian:3 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, SceCursor, SceSink

from dataclasses import dataclass
from typing import Optional


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
        ``VleWidthOverflow``."""
        try:
            raw = cursor.peek_slice(4)
        except NeedMoreBytes:
            return None
        sensor_id = raw[0]
        value = raw[1] | (raw[2] << 8)
        status = raw[3]
        value = cls(
            sensor_id=sensor_id,
            value=value,
            status=status,
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
        w.write_u8(self.sensor_id & 0xFF)
        w.write_u8(self.value & 0xFF)
        w.write_u8((self.value >> 8) & 0xFF)
        w.write_u8(self.status & 0xFF)

    def encode_to_bytes(self) -> bytes:
        """Heap-backed convenience facade. Runs :meth:`encode` over a
        :class:`BytearraySink` and returns the freshly-encoded bytes.
        Callers targeting zero-alloc hot paths should call :meth:`encode`
        directly against a caller-owned sink."""
        _dst = bytearray()
        self.encode(BytearraySink(_dst))
        return bytes(_dst)
