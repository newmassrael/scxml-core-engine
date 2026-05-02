# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import CodecError, NeedMoreBytes, SceCursor

from dataclasses import dataclass
from typing import Optional


@dataclass
class CodecFlagsBasic:
    header: int = 0

    @classmethod
    def decode(cls, cursor: SceCursor) -> Optional[CodecFlagsBasic]:
        """Decode the next frame from ``cursor``. Returns ``None`` when
        the cursor's tail is shorter than the declared minimum frame
        (RFC §5.B L494-519); on success the cursor advances past the
        consumed bytes. VLE codecs also return ``None`` on
        ``VleWidthOverflow``."""
        try:
            raw = cursor.peek_slice(1)
        except NeedMoreBytes:
            return None
        value = cls(
            header=raw[0],
        )
        try:
            cursor.advance(1)
        except NeedMoreBytes:
            return None
        return value

    # RFC §5.B B1-γ flags primitive: per-bit accessors over the carrier
    # field. Plain methods (rather than @property) for API symmetry with
    # Rust / Cpp / Kotlin / Go / C11. Python ints are unbounded, so the
    # clear path masks back to the carrier's natural width to keep the
    # value inside the unsigned domain after `~mask` flips the sign.
    # Wire layout is unchanged.
    def reliable(self) -> bool:
        return (self.header & 0x80) != 0

    def set_reliable(self, v: bool) -> None:
        if v:
            self.header = (self.header | 0x80) & 0xFF
        else:
            self.header = self.header & (0xFF ^ 0x80)

    def more(self) -> bool:
        return (self.header & 0x40) != 0

    def set_more(self, v: bool) -> None:
        if v:
            self.header = (self.header | 0x40) & 0xFF
        else:
            self.header = self.header & (0xFF ^ 0x40)

    def drop(self) -> bool:
        return (self.header & 0x20) != 0

    def set_drop(self, v: bool) -> None:
        if v:
            self.header = (self.header | 0x20) & 0xFF
        else:
            self.header = self.header & (0xFF ^ 0x20)

    def first(self) -> bool:
        return (self.header & 0x10) != 0

    def set_first(self, v: bool) -> None:
        if v:
            self.header = (self.header | 0x10) & 0xFF
        else:
            self.header = self.header & (0xFF ^ 0x10)

    def encode(self) -> bytes:
        return bytes([
            self.header & 0xFF,
        ])
