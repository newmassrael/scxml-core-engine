# SCE-MAP: codec_length_ref:3

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, SceCursor, SceSink

from dataclasses import dataclass
from typing import Optional


@dataclass
class CodecLengthRef:
    msg_id: int = 0
    len: int = 0
    payload: bytes = b""

    @classmethod
    def decode(cls, cursor: SceCursor) -> Optional[CodecLengthRef]:
        """Decode the next frame from ``cursor``. Returns ``None`` when
        the cursor's tail is shorter than the declared minimum frame
        (RFC §5.B L494-519); on success the cursor advances past the
        consumed bytes. VLE codecs also return ``None`` on
        ``VleWidthOverflow``."""
        try:
            _frame_len = cursor.remaining()
            if _frame_len < 2:
                return None
            raw = cursor.peek_slice(_frame_len)
        except NeedMoreBytes:
            return None
        msg_id = raw[0]
        len = raw[1]
        payload = raw[2:2 + len]
        value = cls(
            msg_id=msg_id,
            len=len,
            payload=payload,
        )
        try:
            cursor.advance(_frame_len)
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
        w.write_u8(self.len & 0xFF)
        w.write_bytes(self.payload)

    def encode_to_bytes(self) -> bytes:
        """Heap-backed convenience facade. Runs :meth:`encode` over a
        :class:`BytearraySink` and returns the freshly-encoded bytes.
        Callers targeting zero-alloc hot paths should call :meth:`encode`
        directly against a caller-owned sink."""
        _dst = bytearray()
        self.encode(BytearraySink(_dst))
        return bytes(_dst)
