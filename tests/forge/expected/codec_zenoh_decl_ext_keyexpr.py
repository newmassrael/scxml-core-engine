# SCE-MAP: codec_zenoh_decl_ext_keyexpr:89 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, SceCursor, SceSink
from .codec_zenoh_decl_ext_keyexpr_inner import CodecZenohDeclExtKeyexprInner
from . import codec_zenoh_decl_ext_keyexpr_inner as sce_codec_zenoh_decl_ext_keyexpr_inner

from dataclasses import dataclass, field
from typing import Optional, Protocol


class CodecZenohDeclExtKeyexprSource(Protocol):
    """The elements a value of :class:`CodecZenohDeclExtKeyexpr` presents, which its encode
    reads (docs/adr/0081 §2): any object that presents them is a source of it.
    The encode derives every other element of the frame."""

    @property
    def outer_header(self) -> int: ...

    @property
    def inner(self) -> sce_codec_zenoh_decl_ext_keyexpr_inner.CodecZenohDeclExtKeyexprInnerSource: ...


def encode_source(src: CodecZenohDeclExtKeyexprSource, w: SceSink) -> None:
    """Write any source of :class:`CodecZenohDeclExtKeyexpr` into ``w``, each element read
    off ``src`` and written where the codec's encode plan lays it out
    (docs/adr/0081). It derives ``total_length`` from what each measures, and never reads it off
    the source. It refuses no value; a bounded sink raises
    :class:`BufferOverflow` where it has no room left."""
    _b_inner = bytearray()
    sce_codec_zenoh_decl_ext_keyexpr_inner.encode_source(src.inner, BytearraySink(_b_inner))
    _d_total_length = len(_b_inner)
    w.write_u8(src.outer_header & 0xFF)
    w.write_vle_u64(_d_total_length)
    w.write_bytes(_b_inner)


@dataclass
class CodecZenohDeclExtKeyexpr:
    outer_header: int = 0
    inner: CodecZenohDeclExtKeyexprInner = field(default_factory=CodecZenohDeclExtKeyexprInner)

    @classmethod
    def decode(cls, cursor: SceCursor) -> Optional[CodecZenohDeclExtKeyexpr]:
        """Decode the next frame from ``cursor``. Returns ``None`` when
        the cursor's tail is shorter than the declared minimum frame
        (RFC §synth-5-B L494-519); on success the cursor advances past the
        consumed bytes. VLE codecs also return ``None`` on
        ``VleWidthOverflow``, and every codec on a field's refusal: a length
        or a count past its bound (``LengthOutOfRange``, ``TooManyElements``,
        docs/adr/0077), an undeclared enum value."""
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
            f_outer_header = raw[0]
            cursor.advance(1)
            f_total_length = cursor.read_vle_u64()
            _len = int(f_total_length)
            _raw = cursor.peek_slice(_len)
            if _raw is None:
                return None
            _inner = SceCursor(bytes(_raw))
            f_inner = CodecZenohDeclExtKeyexprInner.decode(_inner)
            if f_inner is None:
                return None
            cursor.advance(_len)
        except CodecError:
            return None
        return cls(
            outer_header=f_outer_header,
            inner=f_inner,
        )

    # RFC §synth-5-B flags primitive: per-bit-range accessors over
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
        """RFC §synth-5-B encode-side primary: write ``self`` into the
        caller-owned ``w`` sink. Returns ``None`` on success; raises
        :class:`BufferOverflow` from a bounded sink when the destination
        has insufficient remaining capacity; growable sinks (e.g.
        :class:`BytearraySink`) are effectively infallible."""
        encode_source(self, w)

    def encode_to_bytes(self) -> bytes:
        """Heap-backed convenience facade. Runs :meth:`encode` over a
        :class:`BytearraySink` and returns the freshly-encoded bytes.
        Callers targeting zero-alloc hot paths should call :meth:`encode`
        directly against a caller-owned sink."""
        _dst = bytearray()
        self.encode(BytearraySink(_dst))
        return bytes(_dst)
