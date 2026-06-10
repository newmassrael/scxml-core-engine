# SCE-MAP: codec_zenoh_decl_queryable:46

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, SceCursor, SceSink
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
    def decode(cls, cursor: SceCursor, n: int, z: int) -> Optional[CodecZenohDeclQueryable]:
        """Decode the next frame from ``cursor``. Returns ``None`` when
        the cursor's tail is shorter than the declared minimum frame
        (RFC §5.B L494-519); on success the cursor advances past the
        consumed bytes. VLE codecs also return ``None`` on
        ``VleWidthOverflow``."""
        # RFC §5.B present-if primitive: streaming decode
        # advances the cursor per field. Per-field statements live
        # inside one outer `try:` block so the first peek/advance
        # failure unwinds to a single `except NeedMoreBytes`. Per-
        # field `is_repeat` routes Repeat fields to the dedicated
        # helper. Branch fires before has_vle_fields so a codec mixing
        # VLE + present-if uses the unified streaming path.
        try:
            id = cursor.read_vle_u32()
            wireexpr = CodecZenohWireexpr.decode(cursor, n)
            if wireexpr is None:
                return None
            if (z & 0x01) != 0:
                raw = cursor.peek_slice(1)
                _v = raw[0]
                cursor.advance(1)
                ext_type = _v
            else:
                ext_type = None
            if (z & 0x01) != 0:
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

    def encode(self, w: SceSink, n: int, z: int) -> None:
        """RFC §5.B encode-side primary: write ``self`` into the
        caller-owned ``w`` sink. Returns ``None`` on success; raises
        :class:`BufferOverflow` from a bounded sink when the destination
        has insufficient remaining capacity; growable sinks (e.g.
        :class:`BytearraySink`) are effectively infallible."""
        # RFC §5.B present-if encode.
        _vle = int(self.id)
        while _vle >= 0x80:
            w.write_u8((_vle & 0x7F) | 0x80)
            _vle >>= 7
        w.write_u8(_vle)
        self.wireexpr.encode(w, n)
        if self.ext_type is not None:
            w.write_u8(self.ext_type & 0xFF)
        if self.ext_value is not None:
            _vle = int(self.ext_value)
            while _vle >= 0x80:
                w.write_u8((_vle & 0x7F) | 0x80)
                _vle >>= 7
            w.write_u8(_vle)

    def encode_to_bytes(self, n: int, z: int) -> bytes:
        """Heap-backed convenience facade. Runs :meth:`encode` over a
        :class:`BytearraySink` and returns the freshly-encoded bytes.
        Callers targeting zero-alloc hot paths should call :meth:`encode`
        directly against a caller-owned sink."""
        _dst = bytearray()
        self.encode(BytearraySink(_dst), n, z)
        return bytes(_dst)
