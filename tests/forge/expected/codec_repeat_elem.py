# SCE-MAP: codec_repeat_elem:5

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import CodecError, NeedMoreBytes, SceCursor

from dataclasses import dataclass
from typing import Optional


@dataclass
class CodecRepeatElem:
    seq: int = 0

    @classmethod
    def decode(cls, cursor: SceCursor) -> Optional[CodecRepeatElem]:
        """Decode the next frame from ``cursor``. Returns ``None`` when
        the cursor's tail is shorter than the declared minimum frame
        (RFC §5.B L494-519); on success the cursor advances past the
        consumed bytes. VLE codecs also return ``None`` on
        ``VleWidthOverflow``."""
        try:
            raw = cursor.peek_slice(2)
        except NeedMoreBytes:
            return None
        seq = (raw[0] << 8) | raw[1]
        value = cls(
            seq=seq,
        )
        try:
            cursor.advance(2)
        except NeedMoreBytes:
            return None
        return value

    def encode(self) -> bytes:
        return bytes([
            (self.seq >> 8) & 0xFF,
            self.seq & 0xFF,
        ])
