# SCE-MAP: codec_zenoh_decl_ext_keyexpr_inner:65 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, LengthOutOfRange, NeedMoreBytes, SceCursor, SceSink
from sce_forge_runtime.bounded import BoundedBytes

from dataclasses import dataclass
from typing import Optional, Protocol


class CodecZenohDeclExtKeyexprInnerSuffixBytes(BoundedBytes):
    """The ``suffix`` element of :class:`CodecZenohDeclExtKeyexprInner`, 0 to 65535 bytes,
    held to that bound where a value of it is made (docs/adr/0077 §3)."""

    LEAST = 0
    MOST = 65535


class CodecZenohDeclExtKeyexprInnerSource(Protocol):
    """The elements a value of :class:`CodecZenohDeclExtKeyexprInner` presents, which its encode
    reads (docs/adr/0081 §2): any object that presents them is a source of it.
    The encode derives every other element of the frame."""

    @property
    def inner_header(self) -> int: ...

    @property
    def id(self) -> int: ...

    @property
    def suffix(self) -> Optional[CodecZenohDeclExtKeyexprInnerSuffixBytes]: ...


def encode_source(src: CodecZenohDeclExtKeyexprInnerSource, w: SceSink) -> None:
    """Write any source of :class:`CodecZenohDeclExtKeyexprInner` into ``w``, each element read
    off ``src`` and written where the codec's encode plan lays it out
    (docs/adr/0081). It refuses no value; a bounded sink raises
    :class:`BufferOverflow` where it has no room left."""
    _c_inner_header = src.inner_header
    _c_inner_header = (_c_inner_header & ~0x1) | (0x1 if src.suffix is not None else 0)
    w.write_u8(_c_inner_header & 0xFF)
    w.write_vle_u64(src.id)
    if src.suffix is not None:
        w.write_bytes(src.suffix)


@dataclass
class CodecZenohDeclExtKeyexprInner:
    inner_header: int = 0
    id: int = 0
    suffix: Optional[CodecZenohDeclExtKeyexprInnerSuffixBytes] = None

    def __setattr__(self, name: str, value: object) -> None:
        # Each byte string and text is held to its bound wherever it is
        # set, where the value is made and after (docs/adr/0081 §2),
        # whatever it was handed.
        if name == "suffix":
            value = None if value is None else CodecZenohDeclExtKeyexprInnerSuffixBytes(value)
        object.__setattr__(self, name, value)

    @classmethod
    def decode(cls, cursor: SceCursor) -> Optional[CodecZenohDeclExtKeyexprInner]:
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
            f_inner_header = raw[0]
            cursor.advance(1)
            f_id = cursor.read_vle_u64()
            if (f_inner_header & 0x01) != 0:
                _n = cursor.remaining()
                if _n > 65535:
                    raise LengthOutOfRange()
                raw = cursor.peek_slice(_n)
                _v = bytes(raw)
                cursor.advance(_n)
                f_suffix = _v
            else:
                f_suffix = None
        except CodecError:
            return None
        return cls(
            inner_header=f_inner_header,
            id=f_id,
            suffix=f_suffix,
        )

    # RFC §synth-5-B flags primitive: per-bit-range accessors over
    # the carrier field. Single-bit (width=1) reads as bool; multi-bit
    # (width>=2) reads as ``int`` (Python ints are unbounded, so a single
    # ``int`` covers every result-type width). Setters mask + shift on
    # the way in so out-of-range callers can't corrupt sibling bits.
    # Plain methods (rather than @property) for API symmetry with
    # Rust / Cpp / Kotlin / Go / C11. Wire layout is unchanged.
    def n(self) -> bool:
        return (self.inner_header & 0x01) != 0

    def set_n(self, v: bool) -> None:
        if v:
            self.inner_header = (self.inner_header | 0x01) & 0xFF
        else:
            self.inner_header = self.inner_header & (0xFF ^ 0x01)

    def m(self) -> bool:
        return (self.inner_header & 0x02) != 0

    def set_m(self, v: bool) -> None:
        if v:
            self.inner_header = (self.inner_header | 0x02) & 0xFF
        else:
            self.inner_header = self.inner_header & (0xFF ^ 0x02)

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
