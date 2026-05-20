# SCE-MAP: codec_little_endian:3

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import CodecError, NeedMoreBytes, SceCursor

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
        (RFC §5.B L494-519); on success the cursor advances past the
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

    def encode(self) -> bytes:
        return bytes([
            self.sensor_id & 0xFF,
            self.value & 0xFF,
            (self.value >> 8) & 0xFF,
            self.status & 0xFF,
        ])
