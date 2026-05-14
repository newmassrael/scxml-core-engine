# SCE-MAP: codec_zenoh_decl_queryable:46

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import CodecError, NeedMoreBytes, SceCursor
from .codec_zenoh_wireexpr import CodecZenohWireexpr

from dataclasses import dataclass
from typing import Optional


@dataclass
class CodecZenohDeclQueryable:
    id: int = 0
    wireexpr: CodecZenohWireexpr = b""
    ext_type: Optional[int] = None
    ext_value: Optional[int] = None

    @classmethod
    def decode(cls, cursor: SceCursor, parent_flags: int) -> Optional[CodecZenohDeclQueryable]:
        """Decode the next frame from ``cursor``. Returns ``None`` when
        the cursor's tail is shorter than the declared minimum frame
        (RFC §5.B L494-519); on success the cursor advances past the
        consumed bytes. VLE codecs also return ``None`` on
        ``VleWidthOverflow``."""
        # RFC §5.B B5-γ: ``parent_flags`` is the parent codec's flags
        # carrier value, threaded by the variant arm dispatcher. Body
        # fields gated via ``parent.<flag>`` predicates read from this
        # parameter; the ``_ = parent_flags`` defensive guard suppresses
        # unused-variable warnings (mirrors Rust's ``let _ = parent_flags;``,
        # Cpp's ``(void)parent_flags;`` defensive guards) for codecs
        # that declare ``<sce:requires-parent-flags>`` without any
        # consuming gated field.
        _ = parent_flags
        # RFC §5.B B1-δ + B2-β present-if primitive: streaming decode
        # advances the cursor per field. Per-field statements live
        # inside one outer `try:` block so the first peek/advance
        # failure unwinds to a single `except NeedMoreBytes`. Per-
        # field `is_repeat` routes Repeat fields to the dedicated
        # helper. Branch fires before has_vle_fields so a codec mixing
        # VLE + present-if uses the unified streaming path.
        try:
            id = cursor.read_vle_u32()
            wireexpr = CodecZenohWireexpr.decode(cursor, parent_flags)
            if wireexpr is None:
                return None
            if (parent_flags & 0x80) != 0:
                raw = cursor.peek_slice(1)
                _v = raw[0]
                cursor.advance(1)
                ext_type = _v
            else:
                ext_type = None
            if (parent_flags & 0x80) != 0:
                _v = cursor.read_vle_u64()
                ext_value = _v
            else:
                ext_value = None
        except NeedMoreBytes:
            return None
        return cls(
            id=id,
            wireexpr=wireexpr,
            ext_type=ext_type,
            ext_value=ext_value,
        )

    def encode(self, parent_flags: int) -> bytes:
        # RFC §5.B B5-γ: see ``decode`` — same parameter, same suppress.
        _ = parent_flags
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
        r.extend(self.wireexpr.encode(parent_flags))
        if self.ext_type is not None:
            r.append(self.ext_type & 0xFF)
        if self.ext_value is not None:
            _w = int(self.ext_value)
            while _w >= 0x80:
                r.append((_w & 0x7F) | 0x80)
                _w >>= 7
            r.append(_w)
        return bytes(r)
