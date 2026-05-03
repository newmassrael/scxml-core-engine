# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import CodecError, NeedMoreBytes, SceCursor

from dataclasses import dataclass
from typing import Optional


@dataclass
class CodecZenohPut:
    payload: int = 0

    @classmethod
    def decode(cls, cursor: SceCursor) -> Optional[CodecZenohPut]:
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
            payload=raw[0],
        )
        try:
            cursor.advance(1)
        except NeedMoreBytes:
            return None
        return value

    def encode(self) -> bytes:
        return bytes([
            self.payload & 0xFF,
        ])
