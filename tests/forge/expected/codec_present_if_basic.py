# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import CodecError, NeedMoreBytes, SceCursor

from dataclasses import dataclass
from typing import Optional


@dataclass
class CodecPresentIfBasic:
    flags: int = 0
    seq: Optional[int] = None

    @classmethod
    def decode(cls, cursor: SceCursor) -> Optional[CodecPresentIfBasic]:
        """Decode the next frame from ``cursor``. Returns ``None`` when
        the cursor's tail is shorter than the declared minimum frame
        (RFC §5.B L494-519); on success the cursor advances past the
        consumed bytes. VLE codecs also return ``None`` on
        ``VleWidthOverflow``."""
        # RFC §5.B B1-δ present-if primitive: streaming decode advances
        # the cursor per field. Per-field statements live inside one
        # outer `try:` block so the first peek/advance failure unwinds
        # to a single `except NeedMoreBytes: return None`. Gated fields
        # bind the local to `None` on the absent branch so the dataclass
        # passes the local through unchanged.
        try:
            raw = cursor.peek_slice(1)
            flags = raw[0]
            cursor.advance(1)
            if (flags & 0x01) != 0:
                raw = cursor.peek_slice(2)
                _v = (raw[0] << 8) | raw[1]
                cursor.advance(2)
                seq = _v
            else:
                seq = None
        except NeedMoreBytes:
            return None
        return cls(
            flags=flags,
            seq=seq,
        )

    # RFC §5.B B1-γ flags primitive: per-bit accessors over the carrier
    # field. Plain methods (rather than @property) for API symmetry with
    # Rust / Cpp / Kotlin / Go / C11. Python ints are unbounded, so the
    # clear path masks back to the carrier's natural width to keep the
    # value inside the unsigned domain after `~mask` flips the sign.
    # Wire layout is unchanged.
    def has_seq(self) -> bool:
        return (self.flags & 0x01) != 0

    def set_has_seq(self, v: bool) -> None:
        if v:
            self.flags = (self.flags | 0x01) & 0xFF
        else:
            self.flags = self.flags & (0xFF ^ 0x01)

    def encode(self) -> bytes:
        # RFC §5.B B1-δ encode: per-field byte append. Gated fields skip
        # the append when the optional is `None` (author keeps the
        # carrier's flag bit and the optional's truth value in sync —
        # same trust contract as the variant primitive).
        r = bytearray()
        r.append(self.flags & 0xFF)
        if self.seq is not None:
            r.append((self.seq >> 8) & 0xFF)
            r.append(self.seq & 0xFF)
        return bytes(r)
