# SCE-MAP: codec_chain_has_marker:28 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, PresentIfMismatch, SceCursor, SceSink, TlvChainOverflow
from .codec_zenoh_ext_entry import CodecZenohExtEntry
from .codec_chain_has_marker_slice import CodecChainHasMarkerSlice

from dataclasses import dataclass, field
from typing import Optional, List


@dataclass
class CodecChainHasMarker:
    header: int = 0
    extensions: Optional[List[CodecZenohExtEntry]] = b""
    payload_len: Optional[int] = None
    payload: Optional[bytes] = None
    slice_count: Optional[int] = None
    slices: Optional[List[CodecChainHasMarkerSlice]] = None

    @classmethod
    def decode(cls, cursor: SceCursor) -> Optional[CodecChainHasMarker]:
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
            if (header & 0x80) != 0:
                extensions = []
                _more = False
                for _ in range(4):
                    if cursor.remaining() == 0:
                        break
                    _elem = CodecZenohExtEntry.decode(cursor)
                    if _elem is None:
                        return None
                    _more = _elem.z()
                    extensions.append(_elem)
                    if not _more:
                        break
                if _more and cursor.remaining() == 0:
                    raise NeedMoreBytes()
                if _more:
                    raise TlvChainOverflow()
            else:
                extensions = None
            _has_extensions_18 = extensions is not None and any((_e.header & 127) == 18 for _e in extensions)
            if not _has_extensions_18:
                _v = cursor.read_vle_u64()
                payload_len = _v
            else:
                payload_len = None
            if not _has_extensions_18:
                _n = payload_len
                raw = cursor.peek_slice(_n)
                _v = bytes(raw)
                cursor.advance(_n)
                payload = _v
            else:
                payload = None
            if _has_extensions_18:
                _v = cursor.read_vle_u32()
                slice_count = _v
            else:
                slice_count = None
            if _has_extensions_18:
                slices = []
                for _ in range(slice_count):
                    _elem = CodecChainHasMarkerSlice.decode(cursor)
                    if _elem is None:
                        return None
                    slices.append(_elem)
            else:
                slices = None
        except CodecError:
            return None
        return cls(
            header=header,
            extensions=extensions,
            payload_len=payload_len,
            payload=payload,
            slice_count=slice_count,
            slices=slices,
        )

    # RFC §synth-5-B flags primitive: per-bit-range accessors over
    # the carrier field. Single-bit (width=1) reads as bool; multi-bit
    # (width>=2) reads as ``int`` (Python ints are unbounded, so a single
    # ``int`` covers every result-type width). Setters mask + shift on
    # the way in so out-of-range callers can't corrupt sibling bits.
    # Plain methods (rather than @property) for API symmetry with
    # Rust / Cpp / Kotlin / Go / C11. Wire layout is unchanged.
    def kind(self) -> int:
        return (self.header >> 0) & 0x1F

    def set_kind(self, v: int) -> None:
        _shifted_mask = 0x1F << 0
        _val = (v & 0x1F) << 0
        self.header = ((self.header & (0xFF ^ _shifted_mask)) | _val) & 0xFF

    def e(self) -> bool:
        return (self.header & 0x80) != 0

    def set_e(self, v: bool) -> None:
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
        _has_extensions_18 = self.extensions is not None and any((_e.header & 127) == 18 for _e in self.extensions)
        if not _has_extensions_18:
            if self.payload_len is None:
                raise PresentIfMismatch("payload_len")
        elif self.payload_len is not None:
            raise PresentIfMismatch("payload_len")
        if not _has_extensions_18:
            if self.payload is None:
                raise PresentIfMismatch("payload")
        elif self.payload is not None:
            raise PresentIfMismatch("payload")
        if _has_extensions_18:
            if self.slice_count is None:
                raise PresentIfMismatch("slice_count")
        elif self.slice_count is not None:
            raise PresentIfMismatch("slice_count")
        if _has_extensions_18:
            if self.slices is None:
                raise PresentIfMismatch("slices")
        elif self.slices is not None:
            raise PresentIfMismatch("slices")
        # Streaming cursor encode (SSOT selection: `needs_streaming`).
        # Mirrors the streaming decode: every field appends its own bytes
        # in declaration order through the per-field encode blocks, so a
        # gated field skips its append when absent, and a fixed field after
        # a variable-length payload lands after the payload (the positional
        # path appends variable fields last, placing it ahead on the wire).
        # Per-field `is_repeat` / `is_tlv_chain` / `is_embed` route to their
        # dedicated helpers; everything else uses `present_if_encode_block`.
        w.write_u8(self.header & 0xFF)
        if self.extensions is not None:
            for _e in self.extensions:
                _e.encode(w)
        if self.payload_len is not None:
            _vle = int(self.payload_len)
            while _vle >= 0x80:
                w.write_u8((_vle & 0x7F) | 0x80)
                _vle >>= 7
            w.write_u8(_vle)
        if self.payload is not None:
            w.write_bytes(self.payload)
        if self.slice_count is not None:
            _vle = int(self.slice_count)
            while _vle >= 0x80:
                w.write_u8((_vle & 0x7F) | 0x80)
                _vle >>= 7
            w.write_u8(_vle)
        if self.slices is not None:
            for _e in self.slices:
                _e.encode(w)

    def encode_to_bytes(self) -> bytes:
        """Heap-backed convenience facade. Runs :meth:`encode` over a
        :class:`BytearraySink` and returns the freshly-encoded bytes.
        Callers targeting zero-alloc hot paths should call :meth:`encode`
        directly against a caller-owned sink."""
        _dst = bytearray()
        self.encode(BytearraySink(_dst))
        return bytes(_dst)
