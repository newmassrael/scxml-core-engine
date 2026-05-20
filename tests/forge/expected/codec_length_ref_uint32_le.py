# SCE-MAP: codec_length_ref_uint32_le:13

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import CodecError, NeedMoreBytes, SceCursor

from dataclasses import dataclass
from typing import Optional


@dataclass
class CodecLengthRefUint32Le:
    payload_len: int = 0
    payload: bytes = b""

    @classmethod
    def decode(cls, cursor: SceCursor) -> Optional[CodecLengthRefUint32Le]:
        """Decode the next frame from ``cursor``. Returns ``None`` when
        the cursor's tail is shorter than the declared minimum frame
        (RFC §5.B L494-519); on success the cursor advances past the
        consumed bytes. VLE codecs also return ``None`` on
        ``VleWidthOverflow``."""
        try:
            _frame_len = cursor.remaining()
            if _frame_len < 4:
                return None
            raw = cursor.peek_slice(_frame_len)
        except NeedMoreBytes:
            return None
        payload_len = raw[0] | (raw[1] << 8) | (raw[2] << 16) | (raw[3] << 24)
        payload = raw[4:4 + payload_len]
        value = cls(
            payload_len=payload_len,
            payload=payload,
        )
        try:
            cursor.advance(_frame_len)
        except NeedMoreBytes:
            return None
        return value

    def encode(self) -> bytes:
        r = bytearray()
        r.append(self.payload_len & 0xFF)
        r.append((self.payload_len >> 8) & 0xFF)
        r.append((self.payload_len >> 16) & 0xFF)
        r.append((self.payload_len >> 24) & 0xFF)
        r.extend(self.payload)
        return bytes(r)
