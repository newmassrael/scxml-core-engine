# SCE-MAP: codec_zenoh_undecl_subscriber:46

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, SceCursor, SceSink
from .codec_zenoh_decl_ext_keyexpr import CodecZenohDeclExtKeyexpr

from dataclasses import dataclass
from typing import Optional


@dataclass
class CodecZenohUndeclSubscriber:
    id: int = 0
    ext_keyexpr: Optional[CodecZenohDeclExtKeyexpr] = None

    @classmethod
    def decode(cls, cursor: SceCursor, z: int) -> Optional[CodecZenohUndeclSubscriber]:
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
            id = cursor.read_vle_u32()
            if (z & 0x01) != 0:
                ext_keyexpr = CodecZenohDeclExtKeyexpr.decode(cursor)
                if ext_keyexpr is None:
                    return None
            else:
                ext_keyexpr = None
        except NeedMoreBytes:
            return None
        return cls(
            id=id,
            ext_keyexpr=ext_keyexpr,
        )

    def encode(self, w: SceSink, z: int) -> None:
        """RFC §5.B B1-α encode-side primary: write ``self`` into the
        caller-owned ``w`` sink. Returns ``None`` on success; raises
        :class:`BufferOverflow` from a bounded sink when the destination
        has insufficient remaining capacity; growable sinks (e.g.
        :class:`BytearraySink`) are effectively infallible."""
        # RFC §5.B B1-δ + B2-β present-if encode.
        _vle = int(self.id)
        while _vle >= 0x80:
            w.write_u8((_vle & 0x7F) | 0x80)
            _vle >>= 7
        w.write_u8(_vle)
        if self.ext_keyexpr is not None:
            self.ext_keyexpr.encode(w)

    def encode_to_bytes(self, z: int) -> bytes:
        """Heap-backed convenience facade. Runs :meth:`encode` over a
        :class:`BytearraySink` and returns the freshly-encoded bytes.
        Callers targeting zero-alloc hot paths should call :meth:`encode`
        directly against a caller-owned sink."""
        _dst = bytearray()
        self.encode(BytearraySink(_dst), z)
        return bytes(_dst)
