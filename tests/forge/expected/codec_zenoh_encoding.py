# SCE-MAP: codec_zenoh_encoding:68

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, InvalidUtf8, NeedMoreBytes, SceCursor, SceSink

from dataclasses import dataclass
from typing import Optional


@dataclass
class CodecZenohEncoding:
    packed_id: int = 0
    schema_len: Optional[int] = None
    schema: Optional[str] = None

    @classmethod
    def decode(cls, cursor: SceCursor) -> Optional[CodecZenohEncoding]:
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
            packed_id = cursor.read_vle_u32()
            if (packed_id & 0x00000001) != 0:
                _v = cursor.read_vle_u64()
                schema_len = _v
            else:
                schema_len = None
            if (packed_id & 0x00000001) != 0:
                _n = schema_len
                raw = cursor.peek_slice(_n)
                try:
                    _v = bytes(raw).decode('utf-8')
                except UnicodeDecodeError as exc:
                    raise InvalidUtf8() from exc
                cursor.advance(_n)
                schema = _v
            else:
                schema = None
        except CodecError:
            return None
        return cls(
            packed_id=packed_id,
            schema_len=schema_len,
            schema=schema,
        )

    # RFC §synth-5-B flags primitive: per-bit-range accessors over
    # the carrier field. Single-bit (width=1) reads as bool; multi-bit
    # (width>=2) reads as ``int`` (Python ints are unbounded, so a single
    # ``int`` covers every result-type width). Setters mask + shift on
    # the way in so out-of-range callers can't corrupt sibling bits.
    # Plain methods (rather than @property) for API symmetry with
    # Rust / Cpp / Kotlin / Go / C11. Wire layout is unchanged.
    def has_schema(self) -> bool:
        return (self.packed_id & 0x00000001) != 0

    def set_has_schema(self, v: bool) -> None:
        if v:
            self.packed_id = (self.packed_id | 0x00000001) & 0xFFFFFFFF
        else:
            self.packed_id = self.packed_id & (0xFFFFFFFF ^ 0x00000001)

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
        w.write_vle_u32(self.packed_id)
        if self.schema_len is not None:
            _vle = int(self.schema_len)
            while _vle >= 0x80:
                w.write_u8((_vle & 0x7F) | 0x80)
                _vle >>= 7
            w.write_u8(_vle)
        if self.schema is not None:
            w.write_bytes(self.schema.encode('utf-8'))

    def encode_to_bytes(self) -> bytes:
        """Heap-backed convenience facade. Runs :meth:`encode` over a
        :class:`BytearraySink` and returns the freshly-encoded bytes.
        Callers targeting zero-alloc hot paths should call :meth:`encode`
        directly against a caller-owned sink."""
        _dst = bytearray()
        self.encode(BytearraySink(_dst))
        return bytes(_dst)
