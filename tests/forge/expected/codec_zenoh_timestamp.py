# SCE-MAP: codec_zenoh_timestamp:34

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import CodecError, NeedMoreBytes, SceCursor

from dataclasses import dataclass
from typing import Optional


@dataclass
class CodecZenohTimestamp:
    time: int = 0
    zid_len: int = 0
    zid: bytes = b""

    @classmethod
    def decode(cls, cursor: SceCursor) -> Optional[CodecZenohTimestamp]:
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
            time = cursor.read_vle_u64()
            zid_len = cursor.read_vle_u64()
            _n = zid_len
            raw = cursor.peek_slice(_n)
            zid = bytes(raw)
            cursor.advance(_n)
        except CodecError:
            return None
        return cls(
            time=time,
            zid_len=zid_len,
            zid=zid,
        )

    def encode(self) -> bytes:
        # RFC §5.B B4: per-field bit-size dispatch routes Fixed /
        # LengthRef siblings of VLE fields through
        # `present_if_encode_block` (predicate=None arms). Pure-VLE
        # codecs stay byte-stable.
        r = bytearray()
        _w = int(self.time)
        while _w >= 0x80:
            r.append((_w & 0x7F) | 0x80)
            _w >>= 7
        r.append(_w)
        _w = int(self.zid_len)
        while _w >= 0x80:
            r.append((_w & 0x7F) | 0x80)
            _w >>= 7
        r.append(_w)
        r.extend(self.zid)
        return bytes(r)
