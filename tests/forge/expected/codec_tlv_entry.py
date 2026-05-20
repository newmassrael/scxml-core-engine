# SCE-MAP: codec_tlv_entry:10

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import CodecError, NeedMoreBytes, SceCursor

from dataclasses import dataclass
from typing import Optional


@dataclass
class CodecTlvEntry:
    entry_type: int = 0
    entry_len: int = 0
    entry_body: bytes = b""

    @classmethod
    def decode(cls, cursor: SceCursor) -> Optional[CodecTlvEntry]:
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
        entry_type = raw[0]
        entry_len = raw[1]
        entry_body = raw[2:2 + entry_len]
        value = cls(
            entry_type=entry_type,
            entry_len=entry_len,
            entry_body=entry_body,
        )
        try:
            cursor.advance(_frame_len)
        except NeedMoreBytes:
            return None
        return value

    def encode(self) -> bytes:
        r = bytearray()
        r.append(self.entry_type & 0xFF)
        r.append(self.entry_len & 0xFF)
        r.extend(self.entry_body)
        return bytes(r)
