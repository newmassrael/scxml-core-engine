# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import CodecError, NeedMoreBytes, SceCursor
from .codec_zenoh_decl_ext_keyexpr_inner import CodecZenohDeclExtKeyexprInner

from dataclasses import dataclass
from typing import Optional


@dataclass
class CodecZenohDeclExtKeyexpr:
    outer_header: int = 0
    total_length: int = 0
    inner: CodecZenohDeclExtKeyexprInner = b""

    @classmethod
    def decode(cls, cursor: SceCursor) -> Optional[CodecZenohDeclExtKeyexpr]:
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
            outer_header = raw[0]
            cursor.advance(1)
            total_length = cursor.read_vle_u64()
            _len = int(total_length)
            _raw = cursor.peek_slice(_len)
            if _raw is None:
                return None
            _inner = SceCursor(bytes(_raw))
            inner = CodecZenohDeclExtKeyexprInner.decode(_inner)
            if inner is None:
                return None
            cursor.advance(_len)
        except CodecError:
            return None
        return cls(
            outer_header=outer_header,
            total_length=total_length,
            inner=inner,
        )

    # RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors over
    # the carrier field. Single-bit (width=1) reads as bool; multi-bit
    # (width>=2) reads as ``int`` (Python ints are unbounded, so a single
    # ``int`` covers every result-type width). Setters mask + shift on
    # the way in so out-of-range callers can't corrupt sibling bits.
    # Plain methods (rather than @property) for API symmetry with
    # Rust / Cpp / Kotlin / Go / C11. Wire layout is unchanged.
    def ext_id(self) -> int:
        return (self.outer_header >> 0) & 0x0F

    def set_ext_id(self, v: int) -> None:
        _shifted_mask = 0x0F << 0
        _val = (v & 0x0F) << 0
        self.outer_header = ((self.outer_header & (0xFF ^ _shifted_mask)) | _val) & 0xFF

    def m(self) -> bool:
        return (self.outer_header & 0x10) != 0

    def set_m(self, v: bool) -> None:
        if v:
            self.outer_header = (self.outer_header | 0x10) & 0xFF
        else:
            self.outer_header = self.outer_header & (0xFF ^ 0x10)

    def enc(self) -> int:
        return (self.outer_header >> 5) & 0x03

    def set_enc(self, v: int) -> None:
        _shifted_mask = 0x03 << 5
        _val = (v & 0x03) << 5
        self.outer_header = ((self.outer_header & (0xFF ^ _shifted_mask)) | _val) & 0xFF

    def z(self) -> bool:
        return (self.outer_header & 0x80) != 0

    def set_z(self, v: bool) -> None:
        if v:
            self.outer_header = (self.outer_header | 0x80) & 0xFF
        else:
            self.outer_header = self.outer_header & (0xFF ^ 0x80)

    def encode(self) -> bytes:
        # RFC §5.B B4: per-field bit-size dispatch routes Fixed /
        # LengthRef siblings of VLE fields through
        # `present_if_encode_block` (predicate=None arms). Pure-VLE
        # codecs stay byte-stable.
        r = bytearray()
        r.append(self.outer_header & 0xFF)
        _w = int(self.total_length)
        while _w >= 0x80:
            r.append((_w & 0x7F) | 0x80)
            _w >>= 7
        r.append(_w)
        r.extend(self.inner.encode())
        return bytes(r)
