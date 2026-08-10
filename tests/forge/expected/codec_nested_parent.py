# SCE-MAP: codec_nested_parent:22 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, SceCursor, SceSink
from .codec_nested_body import CodecNestedBody

from dataclasses import dataclass, field
from typing import Optional, List


@dataclass
class CodecNestedParent:
    hdr: int = 0
    m: int = 0
    required_body: CodecNestedBody = b""
    optional_body: Optional[CodecNestedBody] = None
    body_list: List[CodecNestedBody] = field(default_factory=list)

    @classmethod
    def decode(cls, cursor: SceCursor) -> Optional[CodecNestedParent]:
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
            raw = cursor.peek_slice(1)
            hdr = raw[0]
            cursor.advance(1)
            raw = cursor.peek_slice(1)
            m = raw[0]
            cursor.advance(1)
            required_body = CodecNestedBody.decode(cursor)
            if required_body is None:
                return None
            if (hdr & 0x01) != 0:
                optional_body = CodecNestedBody.decode(cursor)
                if optional_body is None:
                    return None
            else:
                optional_body = None
            body_list = []
            for _ in range(m):
                _elem = CodecNestedBody.decode(cursor)
                if _elem is None:
                    return None
                body_list.append(_elem)
        except CodecError:
            return None
        return cls(
            hdr=hdr,
            m=m,
            required_body=required_body,
            optional_body=optional_body,
            body_list=body_list,
        )

    # RFC §synth-5-B flags primitive: per-bit-range accessors over
    # the carrier field. Single-bit (width=1) reads as bool; multi-bit
    # (width>=2) reads as ``int`` (Python ints are unbounded, so a single
    # ``int`` covers every result-type width). Setters mask + shift on
    # the way in so out-of-range callers can't corrupt sibling bits.
    # Plain methods (rather than @property) for API symmetry with
    # Rust / Cpp / Kotlin / Go / C11. Wire layout is unchanged.
    def has_opt(self) -> bool:
        return (self.hdr & 0x01) != 0

    def set_has_opt(self, v: bool) -> None:
        if v:
            self.hdr = (self.hdr | 0x01) & 0xFF
        else:
            self.hdr = self.hdr & (0xFF ^ 0x01)

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
        w.write_u8(self.hdr & 0xFF)
        w.write_u8(self.m & 0xFF)
        self.required_body.encode(w)
        if self.optional_body is not None:
            self.optional_body.encode(w)
        for _e in self.body_list:
            _e.encode(w)

    def encode_to_bytes(self) -> bytes:
        """Heap-backed convenience facade. Runs :meth:`encode` over a
        :class:`BytearraySink` and returns the freshly-encoded bytes.
        Callers targeting zero-alloc hot paths should call :meth:`encode`
        directly against a caller-owned sink."""
        _dst = bytearray()
        self.encode(BytearraySink(_dst))
        return bytes(_dst)
