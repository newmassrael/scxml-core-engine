# SCE-MAP: codec_zenoh_source_info_ext:49

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import CodecError, NeedMoreBytes, SceCursor
from .codec_zenoh_source_info import CodecZenohSourceInfo

from dataclasses import dataclass
from typing import Optional


@dataclass
class CodecZenohSourceInfoExt:
    ext_size: int = 0
    info: CodecZenohSourceInfo = b""

    @classmethod
    def decode(cls, cursor: SceCursor) -> Optional[CodecZenohSourceInfoExt]:
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
            ext_size = cursor.read_vle_u64()
            _len = int(ext_size)
            _raw = cursor.peek_slice(_len)
            if _raw is None:
                return None
            _inner = SceCursor(bytes(_raw))
            info = CodecZenohSourceInfo.decode(_inner)
            if info is None:
                return None
            cursor.advance(_len)
        except CodecError:
            return None
        return cls(
            ext_size=ext_size,
            info=info,
        )

    def encode(self) -> bytes:
        # RFC §5.B B4: per-field bit-size dispatch routes Fixed /
        # LengthRef siblings of VLE fields through
        # `present_if_encode_block` (predicate=None arms). Pure-VLE
        # codecs stay byte-stable.
        r = bytearray()
        _w = int(self.ext_size)
        while _w >= 0x80:
            r.append((_w & 0x7F) | 0x80)
            _w >>= 7
        r.append(_w)
        r.extend(self.info.encode())
        return bytes(r)
