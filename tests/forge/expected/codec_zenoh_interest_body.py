# SCE-MAP: codec_zenoh_interest_body:56

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, SceCursor, SceSink
from .codec_zenoh_wireexpr import CodecZenohWireexpr

from dataclasses import dataclass
from typing import Optional


@dataclass
class CodecZenohInterestBody:
    header: int = 0
    keyexpr: Optional[CodecZenohWireexpr] = None

    @classmethod
    def decode(cls, cursor: SceCursor) -> Optional[CodecZenohInterestBody]:
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
            header = raw[0]
            cursor.advance(1)
            if (header & 0x10) != 0:
                keyexpr = CodecZenohWireexpr.decode(cursor, ((header >> 5) & 0x1))
                if keyexpr is None:
                    return None
            else:
                keyexpr = None
        except CodecError:
            return None
        return cls(
            header=header,
            keyexpr=keyexpr,
        )

    # RFC §synth-5-B flags primitive: per-bit-range accessors over
    # the carrier field. Single-bit (width=1) reads as bool; multi-bit
    # (width>=2) reads as ``int`` (Python ints are unbounded, so a single
    # ``int`` covers every result-type width). Setters mask + shift on
    # the way in so out-of-range callers can't corrupt sibling bits.
    # Plain methods (rather than @property) for API symmetry with
    # Rust / Cpp / Kotlin / Go / C11. Wire layout is unchanged.
    def keyexprs(self) -> bool:
        return (self.header & 0x01) != 0

    def set_keyexprs(self, v: bool) -> None:
        if v:
            self.header = (self.header | 0x01) & 0xFF
        else:
            self.header = self.header & (0xFF ^ 0x01)

    def subscribers(self) -> bool:
        return (self.header & 0x02) != 0

    def set_subscribers(self, v: bool) -> None:
        if v:
            self.header = (self.header | 0x02) & 0xFF
        else:
            self.header = self.header & (0xFF ^ 0x02)

    def queryables(self) -> bool:
        return (self.header & 0x04) != 0

    def set_queryables(self, v: bool) -> None:
        if v:
            self.header = (self.header | 0x04) & 0xFF
        else:
            self.header = self.header & (0xFF ^ 0x04)

    def tokens(self) -> bool:
        return (self.header & 0x08) != 0

    def set_tokens(self, v: bool) -> None:
        if v:
            self.header = (self.header | 0x08) & 0xFF
        else:
            self.header = self.header & (0xFF ^ 0x08)

    def restricted(self) -> bool:
        return (self.header & 0x10) != 0

    def set_restricted(self, v: bool) -> None:
        if v:
            self.header = (self.header | 0x10) & 0xFF
        else:
            self.header = self.header & (0xFF ^ 0x10)

    def n(self) -> bool:
        return (self.header & 0x20) != 0

    def set_n(self, v: bool) -> None:
        if v:
            self.header = (self.header | 0x20) & 0xFF
        else:
            self.header = self.header & (0xFF ^ 0x20)

    def m(self) -> bool:
        return (self.header & 0x40) != 0

    def set_m(self, v: bool) -> None:
        if v:
            self.header = (self.header | 0x40) & 0xFF
        else:
            self.header = self.header & (0xFF ^ 0x40)

    def aggregate(self) -> bool:
        return (self.header & 0x80) != 0

    def set_aggregate(self, v: bool) -> None:
        if v:
            self.header = (self.header | 0x80) & 0xFF
        else:
            self.header = self.header & (0xFF ^ 0x80)

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
        w.write_u8(self.header & 0xFF)
        if self.keyexpr is not None:
            self.keyexpr.encode(w, ((self.header >> 5) & 0x1))

    def encode_to_bytes(self) -> bytes:
        """Heap-backed convenience facade. Runs :meth:`encode` over a
        :class:`BytearraySink` and returns the freshly-encoded bytes.
        Callers targeting zero-alloc hot paths should call :meth:`encode`
        directly against a caller-owned sink."""
        _dst = bytearray()
        self.encode(BytearraySink(_dst))
        return bytes(_dst)
