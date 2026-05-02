# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import CodecError, NeedMoreBytes, SceCursor

from dataclasses import dataclass
from typing import Optional


@dataclass
class CodecExtEncodingInfo:
    combined_id: int = 0
    schema_size: int = 0
    schema: Optional[bytes] = None

    @classmethod
    def decode(cls, cursor: SceCursor) -> Optional[CodecExtEncodingInfo]:
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
            combined_id = cursor.read_vle_u32()
            raw = cursor.peek_slice(1)
            schema_size = raw[0]
            cursor.advance(1)
            if (combined_id & 0x00000001) != 0:
                _n = schema_size
                raw = cursor.peek_slice(_n)
                _v = bytes(raw)
                cursor.advance(_n)
                schema = _v
            else:
                schema = None
        except NeedMoreBytes:
            return None
        return cls(
            combined_id=combined_id,
            schema_size=schema_size,
            schema=schema,
        )

    # RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors over
    # the carrier field. Single-bit (width=1) reads as bool; multi-bit
    # (width>=2) reads as ``int`` (Python ints are unbounded, so a single
    # ``int`` covers every result-type width). Setters mask + shift on
    # the way in so out-of-range callers can't corrupt sibling bits.
    # Plain methods (rather than @property) for API symmetry with
    # Rust / Cpp / Kotlin / Go / C11. Wire layout is unchanged.
    def has_schema(self) -> bool:
        return (self.combined_id & 0x00000001) != 0

    def set_has_schema(self, v: bool) -> None:
        if v:
            self.combined_id = (self.combined_id | 0x00000001) & 0xFFFFFFFF
        else:
            self.combined_id = self.combined_id & (0xFFFFFFFF ^ 0x00000001)

    def encode(self) -> bytes:
        # RFC §5.B B1-δ + B2-β present-if encode: per-field byte
        # append. Gated fields skip the append when the optional is
        # `None`. Per-field `is_repeat` routes Repeat fields to the
        # dedicated helper. Branch fires before has_vle_fields so a
        # codec mixing VLE + present-if uses the unified encode path.
        r = bytearray()
        _w = int(self.combined_id)
        while _w >= 0x80:
            r.append((_w & 0x7F) | 0x80)
            _w >>= 7
        r.append(_w)
        r.append(self.schema_size & 0xFF)
        if self.schema is not None:
            r.extend(self.schema)
        return bytes(r)
