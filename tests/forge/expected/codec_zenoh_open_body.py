# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import CodecError, NeedMoreBytes, SceCursor

from dataclasses import dataclass
from typing import Optional


@dataclass
class CodecZenohOpenBody:
    lease: int = 0
    initial_sn: int = 0
    cookie_len: Optional[int] = None
    cookie: Optional[bytes] = None

    @classmethod
    def decode(cls, cursor: SceCursor, parent_flags: int) -> Optional[CodecZenohOpenBody]:
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
            lease = cursor.read_vle_u64()
            initial_sn = cursor.read_vle_u64()
            if (parent_flags & 0x20) == 0:
                _v = cursor.read_vle_u64()
                cookie_len = _v
            else:
                cookie_len = None
            if (parent_flags & 0x20) == 0:
                _n = cookie_len
                raw = cursor.peek_slice(_n)
                _v = bytes(raw)
                cursor.advance(_n)
                cookie = _v
            else:
                cookie = None
        except NeedMoreBytes:
            return None
        return cls(
            lease=lease,
            initial_sn=initial_sn,
            cookie_len=cookie_len,
            cookie=cookie,
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
        _w = int(self.lease)
        while _w >= 0x80:
            r.append((_w & 0x7F) | 0x80)
            _w >>= 7
        r.append(_w)
        _w = int(self.initial_sn)
        while _w >= 0x80:
            r.append((_w & 0x7F) | 0x80)
            _w >>= 7
        r.append(_w)
        if self.cookie_len is not None:
            _v = self.cookie_len
        _w = int(_v)
        while _w >= 0x80:
            r.append((_w & 0x7F) | 0x80)
            _w >>= 7
        r.append(_w)
        if self.cookie is not None:
            r.extend(self.cookie)
        return bytes(r)
