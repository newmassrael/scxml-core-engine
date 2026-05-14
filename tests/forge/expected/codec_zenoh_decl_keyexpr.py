# SCE-MAP: codec_zenoh_decl_keyexpr:47

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import CodecError, NeedMoreBytes, SceCursor
from .codec_zenoh_wireexpr import CodecZenohWireexpr

from dataclasses import dataclass
from typing import Optional


@dataclass
class CodecZenohDeclKeyexpr:
    id: int = 0
    wireexpr: CodecZenohWireexpr = b""

    @classmethod
    def decode(cls, cursor: SceCursor, parent_flags: int) -> Optional[CodecZenohDeclKeyexpr]:
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
        # RFC §5.B B4: per-field bit-size dispatch routes Fixed /
        # LengthRef siblings of VLE fields through
        # `present_if_decode_stmt` (predicate=None arms). Pure-VLE
        # codecs stay byte-stable.
        try:
            id = cursor.read_vle_u16()
            wireexpr = CodecZenohWireexpr.decode(cursor, parent_flags)
            if wireexpr is None:
                return None
        except CodecError:
            return None
        return cls(
            id=id,
            wireexpr=wireexpr,
        )

    def encode(self, parent_flags: int) -> bytes:
        # RFC §5.B B5-γ: see ``decode`` — same parameter, same suppress.
        _ = parent_flags
        # RFC §5.B B4: per-field bit-size dispatch routes Fixed /
        # LengthRef siblings of VLE fields through
        # `present_if_encode_block` (predicate=None arms). Pure-VLE
        # codecs stay byte-stable.
        r = bytearray()
        _w = int(self.id)
        while _w >= 0x80:
            r.append((_w & 0x7F) | 0x80)
            _w >>= 7
        r.append(_w)
        r.extend(self.wireexpr.encode(parent_flags))
        return bytes(r)
