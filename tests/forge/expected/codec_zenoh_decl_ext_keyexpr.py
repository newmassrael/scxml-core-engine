# SCE-MAP: codec_zenoh_decl_ext_keyexpr:89

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, SceCursor, SceSink
from .codec_zenoh_decl_ext_keyexpr_inner import CodecZenohDeclExtKeyexprInner

from dataclasses import dataclass
from typing import Optional


@dataclass
class CodecZenohDeclExtKeyexpr:
    outer_header: int = 0
    total_length: int = 0
    inner: CodecZenohDeclExtKeyexprInner = b""

    @classmethod
    def decode(cls, cursor: SceCursor) -> Optional[CodecZenohDeclExtKeyexpr]:
        """Decode the next frame from ``cursor``. Returns ``None`` when
        the cursor's tail is shorter than the declared minimum frame
        (RFC §5.B L494-519); on success the cursor advances past the
        consumed bytes. VLE codecs also return ``None`` on
        ``VleWidthOverflow``."""
        # RFC §5.B B4: per-field bit-size dispatch routes Fixed /
        # LengthRef siblings of VLE fields through
        # `present_if_decode_stmt` (predicate=None arms). Pure-VLE
        # codecs stay byte-stable.
        try:
            raw = cursor.peek_slice(1)
            outer_header = raw[0]
            cursor.advance(1)
            total_length = cursor.read_vle_u64()
            _len = int(total_length)
            _raw = cursor.peek_slice(_len)
            if _raw is None:
                return None
            _inner = SceCursor(bytes(_raw))
            inner = CodecZenohDeclExtKeyexprInner.decode(_inner)
            if inner is None:
                return None
            cursor.advance(_len)
        except CodecError:
            return None
        return cls(
            outer_header=outer_header,
            total_length=total_length,
            inner=inner,
        )

    # RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors over
    # the carrier field. Single-bit (width=1) reads as bool; multi-bit
    # (width>=2) reads as ``int`` (Python ints are unbounded, so a single
    # ``int`` covers every result-type width). Setters mask + shift on
    # the way in so out-of-range callers can't corrupt sibling bits.
    # Plain methods (rather than @property) for API symmetry with
    # Rust / Cpp / Kotlin / Go / C11. Wire layout is unchanged.
    def ext_id(self) -> int:
        return (self.outer_header >> 0) & 0x0F

    def set_ext_id(self, v: int) -> None:
        _shifted_mask = 0x0F << 0
        _val = (v & 0x0F) << 0
        self.outer_header = ((self.outer_header & (0xFF ^ _shifted_mask)) | _val) & 0xFF

    def m(self) -> bool:
        return (self.outer_header & 0x10) != 0

    def set_m(self, v: bool) -> None:
        if v:
            self.outer_header = (self.outer_header | 0x10) & 0xFF
        else:
            self.outer_header = self.outer_header & (0xFF ^ 0x10)

    def enc(self) -> int:
        return (self.outer_header >> 5) & 0x03

    def set_enc(self, v: int) -> None:
        _shifted_mask = 0x03 << 5
        _val = (v & 0x03) << 5
        self.outer_header = ((self.outer_header & (0xFF ^ _shifted_mask)) | _val) & 0xFF

    def z(self) -> bool:
        return (self.outer_header & 0x80) != 0

    def set_z(self, v: bool) -> None:
        if v:
            self.outer_header = (self.outer_header | 0x80) & 0xFF
        else:
            self.outer_header = self.outer_header & (0xFF ^ 0x80)

    def encode(self, w: SceSink) -> None:
        """RFC §5.B B1-α encode-side primary: write ``self`` into the
        caller-owned ``w`` sink. Returns ``None`` on success; raises
        :class:`BufferOverflow` from a bounded sink when the destination
        has insufficient remaining capacity; growable sinks (e.g.
        :class:`BytearraySink`) are effectively infallible."""
        # RFC §5.B B4: per-field bit-size dispatch.
        w.write_u8(self.outer_header & 0xFF)
        _vle = int(self.total_length)
        while _vle >= 0x80:
            w.write_u8((_vle & 0x7F) | 0x80)
            _vle >>= 7
        w.write_u8(_vle)
        self.inner.encode(w)

    def encode_to_bytes(self) -> bytes:
        """Heap-backed convenience facade. Runs :meth:`encode` over a
        :class:`BytearraySink` and returns the freshly-encoded bytes.
        Callers targeting zero-alloc hot paths should call :meth:`encode`
        directly against a caller-owned sink."""
        _dst = bytearray()
        self.encode(BytearraySink(_dst))
        return bytes(_dst)
