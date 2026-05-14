# SCE-MAP: codec_tlv_chain_present_if_basic:37

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import CodecError, NeedMoreBytes, SceCursor, TlvChainOverflow
from .codec_tlv_entry import CodecTlvEntry

from dataclasses import dataclass, field
from typing import Optional, List


@dataclass
class CodecTlvChainPresentIfBasic:
    carrier: int = 0
    entries: Optional[List[CodecTlvEntry]] = b""

    @classmethod
    def decode(cls, cursor: SceCursor) -> Optional[CodecTlvChainPresentIfBasic]:
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
            carrier = raw[0]
            cursor.advance(1)
            if (carrier & 0x01) != 0:
                entries = []
                for _ in range(4):
                    if cursor.remaining() == 0:
                        break
                    _elem = CodecTlvEntry.decode(cursor)
                    if _elem is None:
                        return None
                    entries.append(_elem)
                if cursor.remaining() > 0:
                    raise TlvChainOverflow()
            else:
                entries = None
        except NeedMoreBytes:
            return None
        return cls(
            carrier=carrier,
            entries=entries,
        )

    # RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors over
    # the carrier field. Single-bit (width=1) reads as bool; multi-bit
    # (width>=2) reads as ``int`` (Python ints are unbounded, so a single
    # ``int`` covers every result-type width). Setters mask + shift on
    # the way in so out-of-range callers can't corrupt sibling bits.
    # Plain methods (rather than @property) for API symmetry with
    # Rust / Cpp / Kotlin / Go / C11. Wire layout is unchanged.
    def has_chain(self) -> bool:
        return (self.carrier & 0x01) != 0

    def set_has_chain(self, v: bool) -> None:
        if v:
            self.carrier = (self.carrier | 0x01) & 0xFF
        else:
            self.carrier = self.carrier & (0xFF ^ 0x01)

    def encode(self) -> bytes:
        # RFC §5.B B1-δ + B2-β present-if encode: per-field byte
        # append. Gated fields skip the append when the optional is
        # `None`. Per-field `is_repeat` routes Repeat fields to the
        # dedicated helper. Branch fires before has_vle_fields so a
        # codec mixing VLE + present-if uses the unified encode path.
        r = bytearray()
        r.append(self.carrier & 0xFF)
        if self.entries is not None:
            for _e in self.entries:
                r.extend(_e.encode())
        return bytes(r)
