# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import CodecError, NeedMoreBytes, SceCursor

from dataclasses import dataclass
from typing import Optional


@dataclass
class CodecVleZintU64:
    value: int = 0

    @classmethod
    def decode(cls, cursor: SceCursor) -> Optional[CodecVleZintU64]:
        """Decode the next frame from ``cursor``. Returns ``None`` when
        the cursor's tail is shorter than the declared minimum frame
        (RFC §5.B L494-519); on success the cursor advances past the
        consumed bytes. VLE codecs also return ``None`` on
        ``VleWidthOverflow``."""
        try:
            value = cursor.read_vle_u64()
        except CodecError:
            return None
        return cls(
            value=value,
        )

    def encode(self) -> bytes:
        r = bytearray()
        _w = int(self.value)
        while _w >= 0x80:
            r.append((_w & 0x7F) | 0x80)
            _w >>= 7
        r.append(_w)
        return bytes(r)
