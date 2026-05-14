# SCE-MAP: codec_peek_arm_b:13

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import CodecError, NeedMoreBytes, SceCursor

from dataclasses import dataclass
from typing import Optional


@dataclass
class CodecPeekArmB:
    header: int = 0
    payload: int = 0

    @classmethod
    def decode(cls, cursor: SceCursor) -> Optional[CodecPeekArmB]:
        """Decode the next frame from ``cursor``. Returns ``None`` when
        the cursor's tail is shorter than the declared minimum frame
        (RFC §5.B L494-519); on success the cursor advances past the
        consumed bytes. VLE codecs also return ``None`` on
        ``VleWidthOverflow``."""
        try:
            raw = cursor.peek_slice(3)
        except NeedMoreBytes:
            return None
        value = cls(
            header=raw[0],
            payload=(raw[1] << 8) | raw[2],
        )
        try:
            cursor.advance(3)
        except NeedMoreBytes:
            return None
        return value

    # RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors over
    # the carrier field. Single-bit (width=1) reads as bool; multi-bit
    # (width>=2) reads as ``int`` (Python ints are unbounded, so a single
    # ``int`` covers every result-type width). Setters mask + shift on
    # the way in so out-of-range callers can't corrupt sibling bits.
    # Plain methods (rather than @property) for API symmetry with
    # Rust / Cpp / Kotlin / Go / C11. Wire layout is unchanged.
    def kind(self) -> bool:
        return (self.header & 0x01) != 0

    def set_kind(self, v: bool) -> None:
        if v:
            self.header = (self.header | 0x01) & 0xFF
        else:
            self.header = self.header & (0xFF ^ 0x01)

    def encode(self) -> bytes:
        return bytes([
            self.header & 0xFF,
            (self.payload >> 8) & 0xFF,
            self.payload & 0xFF,
        ])
