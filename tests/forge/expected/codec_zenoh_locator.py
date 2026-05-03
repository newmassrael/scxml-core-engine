# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import CodecError, InvalidUtf8, NeedMoreBytes, SceCursor

from dataclasses import dataclass
from typing import Optional


@dataclass
class CodecZenohLocator:
    locator_len: int = 0
    locator: str = ""

    @classmethod
    def decode(cls, cursor: SceCursor) -> Optional[CodecZenohLocator]:
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
            locator_len = cursor.read_vle_u64()
            _n = locator_len
            raw = cursor.peek_slice(_n)
            try:
                locator = bytes(raw).decode('utf-8')
            except UnicodeDecodeError as exc:
                raise InvalidUtf8() from exc
            cursor.advance(_n)
        except CodecError:
            return None
        return cls(
            locator_len=locator_len,
            locator=locator,
        )

    def encode(self) -> bytes:
        # RFC §5.B B4: per-field bit-size dispatch routes Fixed /
        # LengthRef siblings of VLE fields through
        # `present_if_encode_block` (predicate=None arms). Pure-VLE
        # codecs stay byte-stable.
        r = bytearray()
        _w = int(self.locator_len)
        while _w >= 0x80:
            r.append((_w & 0x7F) | 0x80)
            _w >>= 7
        r.append(_w)
        r.extend(self.locator.encode('utf-8'))
        return bytes(r)
