# SCE-MAP: codec_zenoh_query:51

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import CodecError, NeedMoreBytes, SceCursor, TlvChainOverflow
from .codec_zenoh_ext_entry import CodecZenohExtEntry

from dataclasses import dataclass, field
from typing import Optional, List


@dataclass
class CodecZenohQuery:
    header: int = 0
    consolidation: Optional[int] = None
    parameters_len: Optional[int] = None
    parameters: Optional[bytes] = None
    extensions: Optional[List[CodecZenohExtEntry]] = b""

    @classmethod
    def decode(cls, cursor: SceCursor) -> Optional[CodecZenohQuery]:
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
            header = raw[0]
            cursor.advance(1)
            if (header & 0x20) != 0:
                raw = cursor.peek_slice(1)
                _v = raw[0]
                cursor.advance(1)
                consolidation = _v
            else:
                consolidation = None
            if (header & 0x40) != 0:
                _v = cursor.read_vle_u64()
                parameters_len = _v
            else:
                parameters_len = None
            if (header & 0x40) != 0:
                _n = parameters_len
                raw = cursor.peek_slice(_n)
                _v = bytes(raw)
                cursor.advance(_n)
                parameters = _v
            else:
                parameters = None
            if (header & 0x80) != 0:
                extensions = []
                for _ in range(8):
                    if cursor.remaining() == 0:
                        break
                    _elem = CodecZenohExtEntry.decode(cursor)
                    if _elem is None:
                        return None
                    extensions.append(_elem)
                if cursor.remaining() > 0:
                    raise TlvChainOverflow()
            else:
                extensions = None
        except NeedMoreBytes:
            return None
        return cls(
            header=header,
            consolidation=consolidation,
            parameters_len=parameters_len,
            parameters=parameters,
            extensions=extensions,
        )

    # RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors over
    # the carrier field. Single-bit (width=1) reads as bool; multi-bit
    # (width>=2) reads as ``int`` (Python ints are unbounded, so a single
    # ``int`` covers every result-type width). Setters mask + shift on
    # the way in so out-of-range callers can't corrupt sibling bits.
    # Plain methods (rather than @property) for API symmetry with
    # Rust / Cpp / Kotlin / Go / C11. Wire layout is unchanged.
    def mid(self) -> int:
        return (self.header >> 0) & 0x1F

    def set_mid(self, v: int) -> None:
        _shifted_mask = 0x1F << 0
        _val = (v & 0x1F) << 0
        self.header = ((self.header & (0xFF ^ _shifted_mask)) | _val) & 0xFF

    def c(self) -> bool:
        return (self.header & 0x20) != 0

    def set_c(self, v: bool) -> None:
        if v:
            self.header = (self.header | 0x20) & 0xFF
        else:
            self.header = self.header & (0xFF ^ 0x20)

    def p(self) -> bool:
        return (self.header & 0x40) != 0

    def set_p(self, v: bool) -> None:
        if v:
            self.header = (self.header | 0x40) & 0xFF
        else:
            self.header = self.header & (0xFF ^ 0x40)

    def z(self) -> bool:
        return (self.header & 0x80) != 0

    def set_z(self, v: bool) -> None:
        if v:
            self.header = (self.header | 0x80) & 0xFF
        else:
            self.header = self.header & (0xFF ^ 0x80)

    def encode(self) -> bytes:
        # RFC §5.B B1-δ + B2-β present-if encode: per-field byte
        # append. Gated fields skip the append when the optional is
        # `None`. Per-field `is_repeat` routes Repeat fields to the
        # dedicated helper. Branch fires before has_vle_fields so a
        # codec mixing VLE + present-if uses the unified encode path.
        r = bytearray()
        r.append(self.header & 0xFF)
        if self.consolidation is not None:
            r.append(self.consolidation & 0xFF)
        if self.parameters_len is not None:
            _w = int(self.parameters_len)
            while _w >= 0x80:
                r.append((_w & 0x7F) | 0x80)
                _w >>= 7
            r.append(_w)
        if self.parameters is not None:
            r.extend(self.parameters)
        if self.extensions is not None:
            for _e in self.extensions:
                r.extend(_e.encode())
        return bytes(r)
