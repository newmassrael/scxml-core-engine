# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import CodecError, NeedMoreBytes, SceCursor

from dataclasses import dataclass
from typing import Optional


@dataclass
class CodecZenohInitBody:
    version: int = 0
    cbyte: int = 0
    zid: bytes = b""
    sn_res: Optional[int] = None
    batch_size: Optional[int] = None
    cookie_len: Optional[int] = None
    cookie: Optional[bytes] = None

    @classmethod
    def decode(cls, cursor: SceCursor, parent_flags: int) -> Optional[CodecZenohInitBody]:
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
            raw = cursor.peek_slice(1)
            version = raw[0]
            cursor.advance(1)
            raw = cursor.peek_slice(1)
            cbyte = raw[0]
            cursor.advance(1)
            _n = (((cbyte >> 4) & 0xF) + 1)
            raw = cursor.peek_slice(_n)
            zid = bytes(raw)
            cursor.advance(_n)
            if (parent_flags & 0x40) != 0:
                raw = cursor.peek_slice(1)
                _v = raw[0]
                cursor.advance(1)
                sn_res = _v
            else:
                sn_res = None
            if (parent_flags & 0x40) != 0:
                raw = cursor.peek_slice(2)
                _v = (raw[0] << 8) | raw[1]
                cursor.advance(2)
                batch_size = _v
            else:
                batch_size = None
            if (parent_flags & 0x20) != 0:
                _v = cursor.read_vle_u64()
                cookie_len = _v
            else:
                cookie_len = None
            if (parent_flags & 0x20) != 0:
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
            version=version,
            cbyte=cbyte,
            zid=zid,
            sn_res=sn_res,
            batch_size=batch_size,
            cookie_len=cookie_len,
            cookie=cookie,
        )

    # RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors over
    # the carrier field. Single-bit (width=1) reads as bool; multi-bit
    # (width>=2) reads as ``int`` (Python ints are unbounded, so a single
    # ``int`` covers every result-type width). Setters mask + shift on
    # the way in so out-of-range callers can't corrupt sibling bits.
    # Plain methods (rather than @property) for API symmetry with
    # Rust / Cpp / Kotlin / Go / C11. Wire layout is unchanged.
    def whatami(self) -> int:
        return (self.cbyte >> 0) & 0x03

    def set_whatami(self, v: int) -> None:
        _shifted_mask = 0x03 << 0
        _val = (v & 0x03) << 0
        self.cbyte = ((self.cbyte & (0xFF ^ _shifted_mask)) | _val) & 0xFF

    def zid_len_m1(self) -> int:
        return (self.cbyte >> 4) & 0x0F

    def set_zid_len_m1(self, v: int) -> None:
        _shifted_mask = 0x0F << 4
        _val = (v & 0x0F) << 4
        self.cbyte = ((self.cbyte & (0xFF ^ _shifted_mask)) | _val) & 0xFF

    def encode(self, parent_flags: int) -> bytes:
        # RFC §5.B B5-γ: see ``decode`` — same parameter, same suppress.
        _ = parent_flags
        # RFC §5.B B1-δ + B2-β present-if encode: per-field byte
        # append. Gated fields skip the append when the optional is
        # `None`. Per-field `is_repeat` routes Repeat fields to the
        # dedicated helper. Branch fires before has_vle_fields so a
        # codec mixing VLE + present-if uses the unified encode path.
        r = bytearray()
        r.append(self.version & 0xFF)
        r.append(self.cbyte & 0xFF)
        r.extend(self.zid)
        if self.sn_res is not None:
            r.append(self.sn_res & 0xFF)
        if self.batch_size is not None:
            r.append((self.batch_size >> 8) & 0xFF)
            r.append(self.batch_size & 0xFF)
        if self.cookie_len is not None:
            _w = int(self.cookie_len)
            while _w >= 0x80:
                r.append((_w & 0x7F) | 0x80)
                _w >>= 7
            r.append(_w)
        if self.cookie is not None:
            r.extend(self.cookie)
        return bytes(r)
