# SCE-MAP: codec_zenoh_frame:25

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, SceCursor, SceSink

from dataclasses import dataclass
from typing import Optional


@dataclass
class CodecZenohFrame:
    sn: int = 0
    payload: bytes = b""

    @classmethod
    def decode(cls, cursor: SceCursor) -> Optional[CodecZenohFrame]:
        """Decode the next frame from ``cursor``. Returns ``None`` when
        the cursor's tail is shorter than the declared minimum frame
        (RFC §synth-5-B L494-519); on success the cursor advances past the
        consumed bytes. VLE codecs also return ``None`` on
        ``VleWidthOverflow``."""
        # RFC §synth-5-B B4: per-field bit-size dispatch routes Fixed /
        # LengthRef siblings of VLE fields through
        # `present_if_decode_stmt` (predicate=None arms). Pure-VLE
        # codecs stay byte-stable.
        try:
            sn = cursor.read_vle_u64()
            _n = cursor.remaining()
            raw = cursor.peek_slice(_n)
            payload = bytes(raw)
            cursor.advance(_n)
        except CodecError:
            return None
        return cls(
            sn=sn,
            payload=payload,
        )

    def encode(self, w: SceSink) -> None:
        """RFC §synth-5-B encode-side primary: write ``self`` into the
        caller-owned ``w`` sink. Returns ``None`` on success; raises
        :class:`BufferOverflow` from a bounded sink when the destination
        has insufficient remaining capacity; growable sinks (e.g.
        :class:`BytearraySink`) are effectively infallible."""
        # RFC §synth-5-B B4: per-field bit-size dispatch.
        _vle = int(self.sn)
        while _vle >= 0x80:
            w.write_u8((_vle & 0x7F) | 0x80)
            _vle >>= 7
        w.write_u8(_vle)
        w.write_bytes(self.payload)

    def encode_to_bytes(self) -> bytes:
        """Heap-backed convenience facade. Runs :meth:`encode` over a
        :class:`BytearraySink` and returns the freshly-encoded bytes.
        Callers targeting zero-alloc hot paths should call :meth:`encode`
        directly against a caller-owned sink."""
        _dst = bytearray()
        self.encode(BytearraySink(_dst))
        return bytes(_dst)
