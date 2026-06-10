# SCE-MAP: codec_init_cookie_body:36

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, SceCursor, SceSink

from dataclasses import dataclass
from typing import Optional


@dataclass
class CodecInitCookieBody:
    version: int = 0
    cookie_size: Optional[int] = None
    cookie: Optional[bytes] = None

    @classmethod
    def decode(cls, cursor: SceCursor, a: int) -> Optional[CodecInitCookieBody]:
        """Decode the next frame from ``cursor``. Returns ``None`` when
        the cursor's tail is shorter than the declared minimum frame
        (RFC §synth-5-B L494-519); on success the cursor advances past the
        consumed bytes. VLE codecs also return ``None`` on
        ``VleWidthOverflow``."""
        # RFC §synth-5-B present-if primitive: streaming decode
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
            if (a & 0x01) != 0:
                _v = cursor.read_vle_u16()
                cookie_size = _v
            else:
                cookie_size = None
            if (a & 0x01) != 0:
                _n = cookie_size
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
            cookie_size=cookie_size,
            cookie=cookie,
        )

    def encode(self, w: SceSink, a: int) -> None:
        """RFC §synth-5-B encode-side primary: write ``self`` into the
        caller-owned ``w`` sink. Returns ``None`` on success; raises
        :class:`BufferOverflow` from a bounded sink when the destination
        has insufficient remaining capacity; growable sinks (e.g.
        :class:`BytearraySink`) are effectively infallible."""
        # RFC §synth-5-B present-if encode.
        w.write_u8(self.version & 0xFF)
        if self.cookie_size is not None:
            _vle = int(self.cookie_size)
            while _vle >= 0x80:
                w.write_u8((_vle & 0x7F) | 0x80)
                _vle >>= 7
            w.write_u8(_vle)
        if self.cookie is not None:
            w.write_bytes(self.cookie)

    def encode_to_bytes(self, a: int) -> bytes:
        """Heap-backed convenience facade. Runs :meth:`encode` over a
        :class:`BytearraySink` and returns the freshly-encoded bytes.
        Callers targeting zero-alloc hot paths should call :meth:`encode`
        directly against a caller-owned sink."""
        _dst = bytearray()
        self.encode(BytearraySink(_dst), a)
        return bytes(_dst)
