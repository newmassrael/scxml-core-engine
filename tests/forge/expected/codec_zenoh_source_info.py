# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import CodecError, NeedMoreBytes, SceCursor

from dataclasses import dataclass
from typing import Optional


@dataclass
class CodecZenohSourceInfo:
    header: int = 0
    zid: bytes = b""
    eid: int = 0
    sn: int = 0

    @classmethod
    def decode(cls, cursor: SceCursor) -> Optional[CodecZenohSourceInfo]:
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
            raw = cursor.peek_slice(1)
            header = raw[0]
            cursor.advance(1)
            _n = (((header >> 4) & 0xF) + 1)
            raw = cursor.peek_slice(_n)
            zid = bytes(raw)
            cursor.advance(_n)
            eid = cursor.read_vle_u32()
            sn = cursor.read_vle_u32()
        except CodecError:
            return None
        return cls(
            header=header,
            zid=zid,
            eid=eid,
            sn=sn,
        )

    # RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors over
    # the carrier field. Single-bit (width=1) reads as bool; multi-bit
    # (width>=2) reads as ``int`` (Python ints are unbounded, so a single
    # ``int`` covers every result-type width). Setters mask + shift on
    # the way in so out-of-range callers can't corrupt sibling bits.
    # Plain methods (rather than @property) for API symmetry with
    # Rust / Cpp / Kotlin / Go / C11. Wire layout is unchanged.
    def zidlen_m1(self) -> int:
        return (self.header >> 4) & 0x0F

    def set_zidlen_m1(self, v: int) -> None:
        _shifted_mask = 0x0F << 4
        _val = (v & 0x0F) << 4
        self.header = ((self.header & (0xFF ^ _shifted_mask)) | _val) & 0xFF

    def encode(self) -> bytes:
        # RFC §5.B B4: per-field bit-size dispatch routes Fixed /
        # LengthRef siblings of VLE fields through
        # `present_if_encode_block` (predicate=None arms). Pure-VLE
        # codecs stay byte-stable.
        r = bytearray()
        r.append(self.header & 0xFF)
        r.extend(self.zid)
        _w = int(self.eid)
        while _w >= 0x80:
            r.append((_w & 0x7F) | 0x80)
            _w >>= 7
        r.append(_w)
        _w = int(self.sn)
        while _w >= 0x80:
            r.append((_w & 0x7F) | 0x80)
            _w >>= 7
        r.append(_w)
        return bytes(r)
