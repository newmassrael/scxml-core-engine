# SCE-MAP: codec_zenoh_decl_ext_keyexpr_inner:64

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import CodecError, NeedMoreBytes, SceCursor

from dataclasses import dataclass
from typing import Optional


@dataclass
class CodecZenohDeclExtKeyexprInner:
    inner_header: int = 0
    id: int = 0
    suffix: Optional[bytes] = None

    @classmethod
    def decode(cls, cursor: SceCursor) -> Optional[CodecZenohDeclExtKeyexprInner]:
        """Decode the next frame from ``cursor``. Returns ``None`` when
        the cursor's tail is shorter than the declared minimum frame
        (RFC §5.B L494-519); on success the cursor advances past the
        consumed bytes. VLE codecs also return ``None`` on
        ``VleWidthOverflow``."""
        # RFC §5.B B1-δ + B2-β present-if primitive: streaming decode
        # advances the cursor per field. Per-field statements live
        # inside one outer `try:` block so the first peek/advance
        # failure unwinds to a single `except NeedMoreBytes`. Per-
        # field `is_repeat` routes Repeat fields to the dedicated
        # helper. Branch fires before has_vle_fields so a codec mixing
        # VLE + present-if uses the unified streaming path.
        try:
            raw = cursor.peek_slice(1)
            inner_header = raw[0]
            cursor.advance(1)
            id = cursor.read_vle_u64()
            if (inner_header & 0x01) != 0:
                _n = cursor.remaining()
                raw = cursor.peek_slice(_n)
                _v = bytes(raw)
                cursor.advance(_n)
                suffix = _v
            else:
                suffix = None
        except NeedMoreBytes:
            return None
        return cls(
            inner_header=inner_header,
            id=id,
            suffix=suffix,
        )

    # RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors over
    # the carrier field. Single-bit (width=1) reads as bool; multi-bit
    # (width>=2) reads as ``int`` (Python ints are unbounded, so a single
    # ``int`` covers every result-type width). Setters mask + shift on
    # the way in so out-of-range callers can't corrupt sibling bits.
    # Plain methods (rather than @property) for API symmetry with
    # Rust / Cpp / Kotlin / Go / C11. Wire layout is unchanged.
    def n(self) -> bool:
        return (self.inner_header & 0x01) != 0

    def set_n(self, v: bool) -> None:
        if v:
            self.inner_header = (self.inner_header | 0x01) & 0xFF
        else:
            self.inner_header = self.inner_header & (0xFF ^ 0x01)

    def m(self) -> bool:
        return (self.inner_header & 0x02) != 0

    def set_m(self, v: bool) -> None:
        if v:
            self.inner_header = (self.inner_header | 0x02) & 0xFF
        else:
            self.inner_header = self.inner_header & (0xFF ^ 0x02)

    def encode(self) -> bytes:
        # RFC §5.B B1-δ + B2-β present-if encode: per-field byte
        # append. Gated fields skip the append when the optional is
        # `None`. Per-field `is_repeat` routes Repeat fields to the
        # dedicated helper. Branch fires before has_vle_fields so a
        # codec mixing VLE + present-if uses the unified encode path.
        r = bytearray()
        r.append(self.inner_header & 0xFF)
        _w = int(self.id)
        while _w >= 0x80:
            r.append((_w & 0x7F) | 0x80)
            _w >>= 7
        r.append(_w)
        if self.suffix is not None:
            r.extend(self.suffix)
        return bytes(r)
