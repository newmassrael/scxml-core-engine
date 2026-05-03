# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import CodecError, NeedMoreBytes, SceCursor

from dataclasses import dataclass
from typing import Optional


@dataclass
class CodecZenohScout:
    version: int = 0
    cbyte: int = 0
    zid: Optional[bytes] = None

    @classmethod
    def decode(cls, cursor: SceCursor) -> Optional[CodecZenohScout]:
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
            version = raw[0]
            cursor.advance(1)
            raw = cursor.peek_slice(1)
            cbyte = raw[0]
            cursor.advance(1)
            if (cbyte & 0x08) != 0:
                _n = (((cbyte >> 4) & 0xF) + 1)
                raw = cursor.peek_slice(_n)
                _v = bytes(raw)
                cursor.advance(_n)
                zid = _v
            else:
                zid = None
        except NeedMoreBytes:
            return None
        return cls(
            version=version,
            cbyte=cbyte,
            zid=zid,
        )

    # RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors over
    # the carrier field. Single-bit (width=1) reads as bool; multi-bit
    # (width>=2) reads as ``int`` (Python ints are unbounded, so a single
    # ``int`` covers every result-type width). Setters mask + shift on
    # the way in so out-of-range callers can't corrupt sibling bits.
    # Plain methods (rather than @property) for API symmetry with
    # Rust / Cpp / Kotlin / Go / C11. Wire layout is unchanged.
    def what(self) -> int:
        return (self.cbyte >> 0) & 0x07

    def set_what(self, v: int) -> None:
        _shifted_mask = 0x07 << 0
        _val = (v & 0x07) << 0
        self.cbyte = ((self.cbyte & (0xFF ^ _shifted_mask)) | _val) & 0xFF

    def i(self) -> bool:
        return (self.cbyte & 0x08) != 0

    def set_i(self, v: bool) -> None:
        if v:
            self.cbyte = (self.cbyte | 0x08) & 0xFF
        else:
            self.cbyte = self.cbyte & (0xFF ^ 0x08)

    def zid_len_m1(self) -> int:
        return (self.cbyte >> 4) & 0x0F

    def set_zid_len_m1(self, v: int) -> None:
        _shifted_mask = 0x0F << 4
        _val = (v & 0x0F) << 4
        self.cbyte = ((self.cbyte & (0xFF ^ _shifted_mask)) | _val) & 0xFF

    def encode(self) -> bytes:
        # RFC §5.B B1-δ + B2-β present-if encode: per-field byte
        # append. Gated fields skip the append when the optional is
        # `None`. Per-field `is_repeat` routes Repeat fields to the
        # dedicated helper. Branch fires before has_vle_fields so a
        # codec mixing VLE + present-if uses the unified encode path.
        r = bytearray()
        r.append(self.version & 0xFF)
        r.append(self.cbyte & 0xFF)
        if self.zid is not None:
            r.extend(self.zid)
        return bytes(r)
