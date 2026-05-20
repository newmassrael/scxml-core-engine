# SCE-MAP: codec_zenoh_decl_subscriber:41

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import CodecError, NeedMoreBytes, SceCursor
from .codec_zenoh_wireexpr import CodecZenohWireexpr

from dataclasses import dataclass
from typing import Optional


@dataclass
class CodecZenohDeclSubscriber:
    id: int = 0
    wireexpr: CodecZenohWireexpr = b""

    @classmethod
    def decode(cls, cursor: SceCursor, n: int) -> Optional[CodecZenohDeclSubscriber]:
        """Decode the next frame from ``cursor``. Returns ``None`` when
        the cursor's tail is shorter than the declared minimum frame
        (RFC §5.B L494-519); on success the cursor advances past the
        consumed bytes. VLE codecs also return ``None`` on
        ``VleWidthOverflow``."""
        # RFC §5.B B4: per-field bit-size dispatch routes Fixed /
        # LengthRef siblings of VLE fields through
        # `present_if_decode_stmt` (predicate=None arms). Pure-VLE
        # codecs stay byte-stable.
        try:
            id = cursor.read_vle_u32()
            wireexpr = CodecZenohWireexpr.decode(cursor, n)
            if wireexpr is None:
                return None
        except CodecError:
            return None
        return cls(
            id=id,
            wireexpr=wireexpr,
        )

    def encode(self, n: int) -> bytes:
        # RFC §5.B B4: per-field bit-size dispatch routes Fixed /
        # LengthRef siblings of VLE fields through
        # `present_if_encode_block` (predicate=None arms). Pure-VLE
        # codecs stay byte-stable.
        r = bytearray()
        _w = int(self.id)
        while _w >= 0x80:
            r.append((_w & 0x7F) | 0x80)
            _w >>= 7
        r.append(_w)
        r.extend(self.wireexpr.encode(n))
        return bytes(r)
