# SCE-MAP: codec_zenoh_decl_ext_keyexpr:89 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, SceCursor, SceSink
from sce_forge_runtime.bounded import BoundedInt
from .codec_zenoh_decl_ext_keyexpr_inner import CodecZenohDeclExtKeyexprInner
from . import codec_zenoh_decl_ext_keyexpr_inner as sce_codec_zenoh_decl_ext_keyexpr_inner

from dataclasses import dataclass, field
from typing import Optional, Protocol


class CodecZenohDeclExtKeyexprOuterHeaderInt(BoundedInt):
    """The ``outer_header`` element of :class:`CodecZenohDeclExtKeyexpr`, an unsigned integer of
    8 bits, held to them where a value of it is made (docs/adr/0081 §2)."""

    BITS = 8
    SIGNED = False


class CodecZenohDeclExtKeyexprSource(Protocol):
    """The elements a value of :class:`CodecZenohDeclExtKeyexpr` presents, which its encode
    reads (docs/adr/0081 §2): any object that presents them is a source of it.
    The encode derives every other element of the frame."""

    @property
    def outer_header(self) -> CodecZenohDeclExtKeyexprOuterHeaderInt: ...

    @property
    def inner(self) -> sce_codec_zenoh_decl_ext_keyexpr_inner.CodecZenohDeclExtKeyexprInnerSource: ...


def encode_source(src: CodecZenohDeclExtKeyexprSource, w: SceSink, *, override_outer_header_m: Optional[int] = None, override_outer_header_z: Optional[int] = None) -> None:
    """Write any source of :class:`CodecZenohDeclExtKeyexpr` into ``w``, each element read
    off ``src`` and written where the codec's encode plan lays it out
    (docs/adr/0081). It derives ``total_length`` from what each measures, and never reads it off
    the source. A chain that holds it
    sets the flag its entries end on by its keyword, in place of the value's. It refuses no value; a bounded sink raises
    :class:`BufferOverflow` where it has no room left."""
    _c_outer_header = src.outer_header
    if override_outer_header_m is not None:
        _c_outer_header = (_c_outer_header & ~0x10) | ((override_outer_header_m & 0x1) << 4)
    if override_outer_header_z is not None:
        _c_outer_header = (_c_outer_header & ~0x80) | ((override_outer_header_z & 0x1) << 7)
    _b_inner = bytearray()
    sce_codec_zenoh_decl_ext_keyexpr_inner.encode_source(src.inner, BytearraySink(_b_inner))
    _d_total_length = len(_b_inner)
    w.write_u8(_c_outer_header & 0xFF)
    w.write_vle_u64(_d_total_length)
    w.write_bytes(_b_inner)


@dataclass
class CodecZenohDeclExtKeyexpr:
    outer_header: CodecZenohDeclExtKeyexprOuterHeaderInt = 0
    inner: CodecZenohDeclExtKeyexprInner = field(default_factory=CodecZenohDeclExtKeyexprInner)

    def __setattr__(self, name: str, value: object) -> None:
        # Each integer, byte string and text is held to its bound
        # wherever it is set, where the value is made and after
        # (docs/adr/0081 §2), whatever it was handed.
        if name == "outer_header":
            value = CodecZenohDeclExtKeyexprOuterHeaderInt(value)
        object.__setattr__(self, name, value)

    @classmethod
    def decode(cls, cursor: SceCursor) -> CodecZenohDeclExtKeyexpr:
        """Decode the next frame from ``cursor``; on success the cursor
        advances past the consumed bytes. A frame the codec refuses raises
        the ``CodecError`` subclass that names the refusal (docs/adr/0099,
        amendment of 2026-11-16): ``NeedMoreBytes`` when the cursor's tail
        is shorter than the frame (RFC §synth-5-B L494-519), which the
        caller resumes after appending bytes, and for a frame no more bytes
        make whole ``VleWidthOverflow``, ``InvalidUtf8``,
        ``UndeclaredEnumValue``, or a length or a count past its bound
        (``LengthOutOfRange``, ``TooManyElements``, docs/adr/0077)."""
        # Streaming cursor decode (SSOT selection: `needs_streaming`).
        # The positional `raw[byte_off]` path is valid only when every
        # field's absolute offset is fixed at codegen time; this branch
        # handles every codec where it is not — present-if-gated fields
        # (runtime presence), VLE / repeat / TLV-chain / embed fields
        # (runtime width), string fields (UTF-8 decode), and a fixed field
        # after a variable-length payload (offset depends on the payload
        # length). Each field reads its own bytes and advances past what it
        # consumed; a refusal raises the `CodecError` that names it.
        raw = cursor.peek_slice(1)
        f_outer_header = raw[0]
        cursor.advance(1)
        f_total_length = cursor.read_vle_u64()
        _len = int(f_total_length)
        _inner = SceCursor(bytes(cursor.peek_slice(_len)))
        f_inner = CodecZenohDeclExtKeyexprInner.decode(_inner)
        cursor.advance(_len)
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
