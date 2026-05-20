# SCE-MAP: codec_length_ref_dotted_basic:27

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import CodecError, NeedMoreBytes, SceCursor

from dataclasses import dataclass
from typing import Optional


@dataclass
class CodecLengthRefDottedBasic:
    carrier: int = 0
    payload: bytes = b""

    @classmethod
    def decode(cls, cursor: SceCursor) -> Optional[CodecLengthRefDottedBasic]:
        """Decode the next frame from ``cursor``. Returns ``None`` when
        the cursor's tail is shorter than the declared minimum frame
        (RFC §5.B L494-519); on success the cursor advances past the
        consumed bytes. VLE codecs also return ``None`` on
        ``VleWidthOverflow``."""
        try:
            _frame_len = cursor.remaining()
            if _frame_len < 1:
                return None
            raw = cursor.peek_slice(_frame_len)
        except NeedMoreBytes:
            return None
        carrier = raw[0]
        payload = raw[1:1 + ((carrier >> 4) & 0xF)]
        value = cls(
            carrier=carrier,
            payload=payload,
        )
        try:
            cursor.advance(_frame_len)
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
    def hdr(self) -> int:
        return (self.carrier >> 0) & 0x0F

    def set_hdr(self, v: int) -> None:
        _shifted_mask = 0x0F << 0
        _val = (v & 0x0F) << 0
        self.carrier = ((self.carrier & (0xFF ^ _shifted_mask)) | _val) & 0xFF

    def payload_len(self) -> int:
        return (self.carrier >> 4) & 0x0F

    def set_payload_len(self, v: int) -> None:
        _shifted_mask = 0x0F << 4
        _val = (v & 0x0F) << 4
        self.carrier = ((self.carrier & (0xFF ^ _shifted_mask)) | _val) & 0xFF

    def encode(self) -> bytes:
        r = bytearray()
        r.append(self.carrier & 0xFF)
        r.extend(self.payload)
        return bytes(r)
