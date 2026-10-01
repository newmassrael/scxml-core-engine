# SCE-MAP: codec_chain_has_tagged:11 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, PresentIfMismatch, SceCursor, SceSink, TlvChainOverflow
from .codec_chain_has_tagged_entry import CodecChainHasTaggedEntry

from dataclasses import dataclass, field
from typing import Optional, List


@dataclass
class CodecChainHasTagged:
    header: int = 0
    entries: List[CodecChainHasTaggedEntry] = field(default_factory=list)
    priority: Optional[int] = None
    checksum: Optional[int] = None

    @classmethod
    def decode(cls, cursor: SceCursor) -> Optional[CodecChainHasTagged]:
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
            entries = []
            _more = False
            for _ in range(3):
                if cursor.remaining() == 0:
                    break
                _elem = CodecChainHasTaggedEntry.decode(cursor)
                if _elem is None:
                    return None
                _more = _elem.more()
                entries.append(_elem)
                if not _more:
                    break
            if _more and cursor.remaining() == 0:
                raise NeedMoreBytes()
            if _more:
                raise TlvChainOverflow()
            _has_entries_7 = any(_e.entry_type == 7 for _e in entries)
            _has_entries_9 = any(_e.entry_type == 9 for _e in entries)
            if _has_entries_7 or (header & 0x01) != 0:
                raw = cursor.peek_slice(1)
                _v = raw[0]
                cursor.advance(1)
                priority = _v
            else:
                priority = None
            if not _has_entries_9:
                raw = cursor.peek_slice(2)
                _v = (raw[0] << 8) | raw[1]
                cursor.advance(2)
                checksum = _v
            else:
                checksum = None
        except CodecError:
            return None
        return cls(
            header=header,
            entries=entries,
            priority=priority,
            checksum=checksum,
        )

    # RFC §synth-5-B flags primitive: per-bit-range accessors over
    # the carrier field. Single-bit (width=1) reads as bool; multi-bit
    # (width>=2) reads as ``int`` (Python ints are unbounded, so a single
    # ``int`` covers every result-type width). Setters mask + shift on
    # the way in so out-of-range callers can't corrupt sibling bits.
    # Plain methods (rather than @property) for API symmetry with
    # Rust / Cpp / Kotlin / Go / C11. Wire layout is unchanged.
    def wide(self) -> bool:
        return (self.header & 0x01) != 0

    def set_wide(self, v: bool) -> None:
        if v:
            self.header = (self.header | 0x01) & 0xFF
        else:
            self.header = self.header & (0xFF ^ 0x01)

    def encode(self, w: SceSink) -> None:
        """RFC §synth-5-B encode-side primary: write ``self`` into the
        caller-owned ``w`` sink. Returns ``None`` on success; raises
        :class:`BufferOverflow` from a bounded sink when the destination
        has insufficient remaining capacity; growable sinks (e.g.
        :class:`BytearraySink`) are effectively infallible."""
        _has_entries_7 = any(_e.entry_type == 7 for _e in self.entries)
        _has_entries_9 = any(_e.entry_type == 9 for _e in self.entries)
        if _has_entries_7 or (self.header & 0x01) != 0:
            if self.priority is None:
                raise PresentIfMismatch("priority")
        elif self.priority is not None:
            raise PresentIfMismatch("priority")
        if not _has_entries_9:
            if self.checksum is None:
                raise PresentIfMismatch("checksum")
        elif self.checksum is not None:
            raise PresentIfMismatch("checksum")
        # Streaming cursor encode (SSOT selection: `needs_streaming`).
        # Mirrors the streaming decode: every field appends its own bytes
        # in declaration order through the per-field encode blocks, so a
        # gated field skips its append when absent, and a fixed field after
        # a variable-length payload lands after the payload (the positional
        # path appends variable fields last, placing it ahead on the wire).
        # Per-field `is_repeat` / `is_tlv_chain` / `is_embed` route to their
        # dedicated helpers; everything else uses `present_if_encode_block`.
        w.write_u8(self.header & 0xFF)
        for _e in self.entries:
            _e.encode(w)
        if self.priority is not None:
            w.write_u8(self.priority & 0xFF)
        if self.checksum is not None:
            w.write_u8((self.checksum >> 8) & 0xFF)
            w.write_u8(self.checksum & 0xFF)

    def encode_to_bytes(self) -> bytes:
        """Heap-backed convenience facade. Runs :meth:`encode` over a
        :class:`BytearraySink` and returns the freshly-encoded bytes.
        Callers targeting zero-alloc hot paths should call :meth:`encode`
        directly against a caller-owned sink."""
        _dst = bytearray()
        self.encode(BytearraySink(_dst))
        return bytes(_dst)
