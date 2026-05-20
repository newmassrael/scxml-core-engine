# SCE-MAP: codec_init_syn_body:30

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, SceCursor, SceSink

from dataclasses import dataclass
from typing import Optional


@dataclass
class CodecInitSynBody:
    version: int = 0
    sn_res: Optional[int] = None
    batch_size: Optional[int] = None

    @classmethod
    def decode(cls, cursor: SceCursor, s: int) -> Optional[CodecInitSynBody]:
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
            raw = cursor.peek_slice(1)
            version = raw[0]
            cursor.advance(1)
            if (s & 0x01) != 0:
                raw = cursor.peek_slice(1)
                _v = raw[0]
                cursor.advance(1)
                sn_res = _v
            else:
                sn_res = None
            if (s & 0x01) != 0:
                raw = cursor.peek_slice(2)
                _v = (raw[0] << 8) | raw[1]
                cursor.advance(2)
                batch_size = _v
            else:
                batch_size = None
        except NeedMoreBytes:
            return None
        return cls(
            version=version,
            sn_res=sn_res,
            batch_size=batch_size,
        )

    def encode(self, w: SceSink, s: int) -> None:
        """RFC §5.B B1-α encode-side primary: write ``self`` into the
        caller-owned ``w`` sink. Returns ``None`` on success; raises
        :class:`BufferOverflow` from a bounded sink when the destination
        has insufficient remaining capacity; growable sinks (e.g.
        :class:`BytearraySink`) are effectively infallible."""
        # RFC §5.B B1-δ + B2-β present-if encode.
        w.write_u8(self.version & 0xFF)
        if self.sn_res is not None:
            w.write_u8(self.sn_res & 0xFF)
        if self.batch_size is not None:
            w.write_u8((self.batch_size >> 8) & 0xFF)
            w.write_u8(self.batch_size & 0xFF)

    def encode_to_bytes(self, s: int) -> bytes:
        """Heap-backed convenience facade. Runs :meth:`encode` over a
        :class:`BytearraySink` and returns the freshly-encoded bytes.
        Callers targeting zero-alloc hot paths should call :meth:`encode`
        directly against a caller-owned sink."""
        _dst = bytearray()
        self.encode(BytearraySink(_dst), s)
        return bytes(_dst)
