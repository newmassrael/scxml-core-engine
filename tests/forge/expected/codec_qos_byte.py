# SCE-MAP: codec_qos_byte:15

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import CodecError, NeedMoreBytes, SceCursor

from dataclasses import dataclass
from typing import Optional


@dataclass
class CodecQosByte:
    qos: int = 0

    @classmethod
    def decode(cls, cursor: SceCursor) -> Optional[CodecQosByte]:
        """Decode the next frame from ``cursor``. Returns ``None`` when
        the cursor's tail is shorter than the declared minimum frame
        (RFC §5.B L494-519); on success the cursor advances past the
        consumed bytes. VLE codecs also return ``None`` on
        ``VleWidthOverflow``."""
        try:
            raw = cursor.peek_slice(1)
        except NeedMoreBytes:
            return None
        qos = raw[0]
        value = cls(
            qos=qos,
        )
        try:
            cursor.advance(1)
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
    def priority(self) -> int:
        return (self.qos >> 0) & 0x07

    def set_priority(self, v: int) -> None:
        _shifted_mask = 0x07 << 0
        _val = (v & 0x07) << 0
        self.qos = ((self.qos & (0xFF ^ _shifted_mask)) | _val) & 0xFF

    def reliable(self) -> bool:
        return (self.qos & 0x08) != 0

    def set_reliable(self, v: bool) -> None:
        if v:
            self.qos = (self.qos | 0x08) & 0xFF
        else:
            self.qos = self.qos & (0xFF ^ 0x08)

    def congestion(self) -> int:
        return (self.qos >> 4) & 0x03

    def set_congestion(self, v: int) -> None:
        _shifted_mask = 0x03 << 4
        _val = (v & 0x03) << 4
        self.qos = ((self.qos & (0xFF ^ _shifted_mask)) | _val) & 0xFF

    def express(self) -> bool:
        return (self.qos & 0x40) != 0

    def set_express(self, v: bool) -> None:
        if v:
            self.qos = (self.qos | 0x40) & 0xFF
        else:
            self.qos = self.qos & (0xFF ^ 0x40)

    def reserved(self) -> bool:
        return (self.qos & 0x80) != 0

    def set_reserved(self, v: bool) -> None:
        if v:
            self.qos = (self.qos | 0x80) & 0xFF
        else:
            self.qos = self.qos & (0xFF ^ 0x80)

    def encode(self) -> bytes:
        return bytes([
            self.qos & 0xFF,
        ])
