# SCE-MAP: codec_simple_frame:3

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, SceCursor, SceSink

from dataclasses import dataclass
from typing import Optional


@dataclass
class CodecSimpleFrame:
    msg_id: int = 0
    length: int = 0
    payload: int = 0

    @classmethod
    def decode(cls, cursor: SceCursor) -> Optional[CodecSimpleFrame]:
        """Decode the next frame from ``cursor``. Returns ``None`` when
        the cursor's tail is shorter than the declared minimum frame
        (RFC §5.B L494-519); on success the cursor advances past the
        consumed bytes. VLE codecs also return ``None`` on
        ``VleWidthOverflow``."""
        try:
            raw = cursor.peek_slice(4)
        except NeedMoreBytes:
            return None
        msg_id = raw[0]
        length = raw[1]
        payload = (raw[2] << 8) | raw[3]
        value = cls(
            msg_id=msg_id,
            length=length,
            payload=payload,
        )
        try:
            cursor.advance(4)
        except NeedMoreBytes:
            return None
        return value

    def encode(self, w: SceSink) -> None:
        """RFC §5.B encode-side primary: write ``self`` into the
        caller-owned ``w`` sink. Returns ``None`` on success; raises
        :class:`BufferOverflow` from a bounded sink when the destination
        has insufficient remaining capacity; growable sinks (e.g.
        :class:`BytearraySink`) are effectively infallible."""
        w.write_u8(self.msg_id & 0xFF)
        w.write_u8(self.length & 0xFF)
        w.write_u8((self.payload >> 8) & 0xFF)
        w.write_u8(self.payload & 0xFF)

    def encode_to_bytes(self) -> bytes:
        """Heap-backed convenience facade. Runs :meth:`encode` over a
        :class:`BytearraySink` and returns the freshly-encoded bytes.
        Callers targeting zero-alloc hot paths should call :meth:`encode`
        directly against a caller-owned sink."""
        _dst = bytearray()
        self.encode(BytearraySink(_dst))
        return bytes(_dst)
