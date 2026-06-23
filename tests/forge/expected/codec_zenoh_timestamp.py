# SCE-MAP: codec_zenoh_timestamp:34

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, SceCursor, SceSink

from dataclasses import dataclass
from typing import Optional


@dataclass
class CodecZenohTimestamp:
    time: int = 0
    zid_len: int = 0
    zid: bytes = b""

    @classmethod
    def decode(cls, cursor: SceCursor) -> Optional[CodecZenohTimestamp]:
        """Decode the next frame from ``cursor``. Returns ``None`` when
        the cursor's tail is shorter than the declared minimum frame
        (RFC §synth-5-B L494-519); on success the cursor advances past the
        consumed bytes. VLE codecs also return ``None`` on
        ``VleWidthOverflow``."""
        # Streaming cursor decode (SSOT selection: `needs_streaming`).
        # The positional `raw[byte_off]` path is valid only when every
        # field's absolute offset is fixed at codegen time; this branch
        # handles every codec where it is not — present-if-gated fields
        # (runtime presence), VLE / repeat / TLV-chain / embed fields
        # (runtime width), string fields (UTF-8 decode), and a fixed field
        # after a variable-length payload (offset depends on the payload
        # length). Each field reads its own bytes and advances past what it
        # consumed, all inside one outer `try:`. The `except` catches the
        # base `CodecError` so a peek/advance `NeedMoreBytes` and a VLE
        # field's `VleWidthOverflow` both unwind to a single `return None`
        # (non-VLE codecs only ever raise `NeedMoreBytes`, a `CodecError`
        # subclass, so this is behaviour-identical for them).
        try:
            time = cursor.read_vle_u64()
            zid_len = cursor.read_vle_u64()
            _n = zid_len
            raw = cursor.peek_slice(_n)
            zid = bytes(raw)
            cursor.advance(_n)
        except CodecError:
            return None
        return cls(
            time=time,
            zid_len=zid_len,
            zid=zid,
        )

    def encode(self, w: SceSink) -> None:
        """RFC §synth-5-B encode-side primary: write ``self`` into the
        caller-owned ``w`` sink. Returns ``None`` on success; raises
        :class:`BufferOverflow` from a bounded sink when the destination
        has insufficient remaining capacity; growable sinks (e.g.
        :class:`BytearraySink`) are effectively infallible."""
        # Streaming cursor encode (SSOT selection: `needs_streaming`).
        # Mirrors the streaming decode: every field appends its own bytes
        # in declaration order through the per-field encode blocks, so a
        # gated field skips its append when absent, and a fixed field after
        # a variable-length payload lands after the payload (the positional
        # path appends variable fields last, placing it ahead on the wire).
        # Per-field `is_repeat` / `is_tlv_chain` / `is_embed` route to their
        # dedicated helpers; everything else uses `present_if_encode_block`.
        _vle = int(self.time)
        _vn = 0
        while _vle >= 0x80 and _vn < 8:
            w.write_u8((_vle & 0x7F) | 0x80)
            _vle >>= 7
            _vn += 1
        w.write_u8(_vle)
        _vle = int(self.zid_len)
        _vn = 0
        while _vle >= 0x80 and _vn < 8:
            w.write_u8((_vle & 0x7F) | 0x80)
            _vle >>= 7
            _vn += 1
        w.write_u8(_vle)
        w.write_bytes(self.zid)

    def encode_to_bytes(self) -> bytes:
        """Heap-backed convenience facade. Runs :meth:`encode` over a
        :class:`BytearraySink` and returns the freshly-encoded bytes.
        Callers targeting zero-alloc hot paths should call :meth:`encode`
        directly against a caller-owned sink."""
        _dst = bytearray()
        self.encode(BytearraySink(_dst))
        return bytes(_dst)
