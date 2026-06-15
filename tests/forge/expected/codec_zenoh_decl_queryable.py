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
        except CodecError:
            return None
        return cls(
            id=id,
            wireexpr=wireexpr,
            ext_type=ext_type,
            ext_value=ext_value,
        )

    def encode(self, w: SceSink, n: int, z: int) -> None:
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
