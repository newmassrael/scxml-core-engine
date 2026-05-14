# SCE-MAP: codec_ext_attachment:27

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import CodecError, NeedMoreBytes, SceCursor

from dataclasses import dataclass
from typing import Optional


@dataclass
class CodecExtAttachment:
    length: int = 0
    body: bytes = b""

    @classmethod
    def decode(cls, cursor: SceCursor) -> Optional[CodecExtAttachment]:
        """Decode the next frame from ``cursor``. Returns ``None`` when
        the cursor's tail is shorter than the declared minimum frame
        (RFC §5.B L494-519); on success the cursor advances past the
        consumed bytes. VLE codecs also return ``None`` on
        ``VleWidthOverflow``."""
        try:
            _frame_len = cursor.remaining()
            if _frame_len < 1:
                return None
            raw = cursor.peek_slice(_frame_len)
        except NeedMoreBytes:
            return None
        value = cls(
            length=raw[0],
            body=raw[1:1 + raw[0]],
        )
        try:
            cursor.advance(_frame_len)
        except NeedMoreBytes:
            return None
        return value

    def encode(self) -> bytes:
        r = bytearray()
        r.append(self.length & 0xFF)
        r.extend(self.body)
        return bytes(r)
