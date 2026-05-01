# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import CodecError, NeedMoreBytes, SceCursor

from dataclasses import dataclass
from typing import Optional


@dataclass
class CodecSubbyte:
    priority: int = 0
    channel: int = 0
    direction: int = 0

    @classmethod
    def decode(cls, cursor: SceCursor) -> Optional[CodecSubbyte]:
        """Decode the next frame from ``cursor``. Returns ``None`` when
        the cursor's tail is shorter than the declared minimum frame
        (RFC §5.B L494-519); on success the cursor advances past the
        consumed bytes. VLE codecs also return ``None`` on
        ``VleWidthOverflow``."""
        try:
            raw = cursor.peek_slice(1)
        except NeedMoreBytes:
            return None
        value = cls(
            priority=(raw[0] >> 5) & 0x07,
            channel=(raw[0] >> 2) & 0x07,
            direction=(raw[0] >> 0) & 0x03,
        )
        try:
            cursor.advance(1)
        except NeedMoreBytes:
            return None
        return value

    def encode(self) -> bytes:
        return bytes([
            ((self.priority & 0x07) << 5 | (self.channel & 0x07) << 2 | (self.direction & 0x03) << 0) & 0xFF,
        ])
