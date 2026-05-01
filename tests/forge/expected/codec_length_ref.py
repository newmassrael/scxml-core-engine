# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import CodecError, NeedMoreBytes, SceCursor

from dataclasses import dataclass
from typing import Optional


@dataclass
class CodecLengthRef:
    msg_id: int = 0
    len: int = 0
    payload: bytes = b""

    @classmethod
    def decode(cls, cursor: SceCursor) -> Optional[CodecLengthRef]:
        """Decode the next frame from ``cursor``. Returns ``None`` when
        the cursor's tail is shorter than the declared minimum frame
        (RFC §5.B L494-519); on success the cursor advances past the
        consumed bytes. VLE codecs also return ``None`` on
        ``VleWidthOverflow``."""
        try:
            _frame_len = cursor.remaining()
            if _frame_len < 2:
                return None
            raw = cursor.peek_slice(_frame_len)
        except NeedMoreBytes:
            return None
        value = cls(
            msg_id=raw[0],
            len=raw[1],
            payload=raw[2:2 + raw[1]],
        )
        try:
            cursor.advance(_frame_len)
        except NeedMoreBytes:
            return None
        return value

    def encode(self) -> bytes:
        r = bytearray()
        r.append(self.msg_id & 0xFF)
        r.append(self.len & 0xFF)
        r.extend(self.payload)
        return bytes(r)
