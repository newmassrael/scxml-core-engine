# SCE-MAP: codec_zenoh_decl_token:28

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, SceCursor, SceSink
from .codec_zenoh_wireexpr import CodecZenohWireexpr

from dataclasses import dataclass
from typing import Optional


@dataclass
class CodecZenohDeclToken:
    id: int = 0
    wireexpr: CodecZenohWireexpr = b""

    @classmethod
    def decode(cls, cursor: SceCursor, n: int) -> Optional[CodecZenohDeclToken]:
        """Decode the next frame from ``cursor``. Returns ``None`` when
        the cursor's tail is shorter than the declared minimum frame
        (RFC §5.B L494-519); on success the cursor advances past the
        consumed bytes. VLE codecs also return ``None`` on
        ``VleWidthOverflow``."""
        # RFC §5.B B4: per-field bit-size dispatch routes Fixed /
        # LengthRef siblings of VLE fields through
        # `present_if_decode_stmt` (predicate=None arms). Pure-VLE
        # codecs stay byte-stable.
        try:
            id = cursor.read_vle_u32()
            wireexpr = CodecZenohWireexpr.decode(cursor, n)
            if wireexpr is None:
                return None
        except CodecError:
            return None
        return cls(
            id=id,
            wireexpr=wireexpr,
        )

    def encode(self, w: SceSink, n: int) -> None:
        """RFC §5.B B1-α encode-side primary: write ``self`` into the
        caller-owned ``w`` sink. Returns ``None`` on success; raises
        :class:`BufferOverflow` from a bounded sink when the destination
        has insufficient remaining capacity; growable sinks (e.g.
        :class:`BytearraySink`) are effectively infallible."""
        # RFC §5.B B4: per-field bit-size dispatch.
        _vle = int(self.id)
        while _vle >= 0x80:
            w.write_u8((_vle & 0x7F) | 0x80)
            _vle >>= 7
        w.write_u8(_vle)
        self.wireexpr.encode(w, n)

    def encode_to_bytes(self, n: int) -> bytes:
        """Heap-backed convenience facade. Runs :meth:`encode` over a
        :class:`BytearraySink` and returns the freshly-encoded bytes.
        Callers targeting zero-alloc hot paths should call :meth:`encode`
        directly against a caller-owned sink."""
        _dst = bytearray()
        self.encode(BytearraySink(_dst), n)
        return bytes(_dst)
