# SCE-MAP: codec_zenoh_wireexpr:53

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import CodecError, InvalidUtf8, NeedMoreBytes, SceCursor

from dataclasses import dataclass
from typing import Optional


@dataclass
class CodecZenohWireexpr:
    id: int = 0
    suffix_len: Optional[int] = None
    suffix: Optional[str] = None

    @classmethod
    def decode(cls, cursor: SceCursor, n: int) -> Optional[CodecZenohWireexpr]:
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
            id = cursor.read_vle_u64()
            if (n & 0x01) != 0:
                _v = cursor.read_vle_u64()
                suffix_len = _v
            else:
                suffix_len = None
            if (n & 0x01) != 0:
                _n = suffix_len
                raw = cursor.peek_slice(_n)
                try:
                    _v = bytes(raw).decode('utf-8')
                except UnicodeDecodeError as exc:
                    raise InvalidUtf8() from exc
                cursor.advance(_n)
                suffix = _v
            else:
                suffix = None
        except NeedMoreBytes:
            return None
        return cls(
            id=id,
            suffix_len=suffix_len,
            suffix=suffix,
        )

    def encode(self, n: int) -> bytes:
        # RFC §5.B B1-δ + B2-β present-if encode: per-field byte
        # append. Gated fields skip the append when the optional is
        # `None`. Per-field `is_repeat` routes Repeat fields to the
        # dedicated helper. Branch fires before has_vle_fields so a
        # codec mixing VLE + present-if uses the unified encode path.
        r = bytearray()
        _w = int(self.id)
        while _w >= 0x80:
            r.append((_w & 0x7F) | 0x80)
            _w >>= 7
        r.append(_w)
        if self.suffix_len is not None:
            _w = int(self.suffix_len)
            while _w >= 0x80:
                r.append((_w & 0x7F) | 0x80)
                _w >>= 7
            r.append(_w)
        if self.suffix is not None:
            r.extend(self.suffix.encode('utf-8'))
        return bytes(r)
