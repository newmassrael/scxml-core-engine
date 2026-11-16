# SCE-MAP: codec_nested_parent:22 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, SceCursor, SceSink, TooManyElements
from sce_forge_runtime.bounded import BoundedTuple
from .codec_nested_body import CodecNestedBody
from . import codec_nested_body as sce_codec_nested_body

from dataclasses import dataclass, field
from typing import Optional, List, Protocol


class CodecNestedParentBodyListList(BoundedTuple):
    """The ``body_list`` element of :class:`CodecNestedParent`, 0 to 4 entries,
    held to that bound where a value of it is made (docs/adr/0077 §3)."""

    LEAST = 0
    MOST = 4


class CodecNestedParentSource(Protocol):
    """The elements a value of :class:`CodecNestedParent` presents, which its encode
    reads (docs/adr/0081 §2): any object that presents them is a source of it.
    The encode derives every other element of the frame."""

    @property
    def hdr(self) -> int: ...

    @property
    def required_body(self) -> sce_codec_nested_body.CodecNestedBodySource: ...

    @property
    def optional_body(self) -> Optional[sce_codec_nested_body.CodecNestedBodySource]: ...

    @property
    def body_list(self) -> CodecNestedParentBodyListList: ...


def encode_source(src: CodecNestedParentSource, w: SceSink) -> None:
    """Write any source of :class:`CodecNestedParent` into ``w``, each element read
    off ``src`` and written where the codec's encode plan lays it out
    (docs/adr/0081). It derives ``m`` from what each measures, and never reads it off
    the source. It refuses no value; a bounded sink raises
    :class:`BufferOverflow` where it has no room left."""
    _d_m = len(src.body_list)
    _c_hdr = src.hdr
    _c_hdr = (_c_hdr & ~0x1) | (0x1 if src.optional_body is not None else 0)
    w.write_u8(_c_hdr & 0xFF)
    w.write_u8(_d_m & 0xFF)
    sce_codec_nested_body.encode_source(src.required_body, w)
    if src.optional_body is not None:
        sce_codec_nested_body.encode_source(src.optional_body, w)
    for _entry in src.body_list:
        sce_codec_nested_body.encode_source(_entry, w)


@dataclass
class CodecNestedParent:
    hdr: int = 0
    required_body: CodecNestedBody = field(default_factory=CodecNestedBody)
    optional_body: Optional[CodecNestedBody] = None
    body_list: CodecNestedParentBodyListList = CodecNestedParentBodyListList()

    def __setattr__(self, name: str, value: object) -> None:
        # Each byte string and text is held to its bound wherever it is
        # set, where the value is made and after (docs/adr/0081 §2),
        # whatever it was handed.
        if name == "body_list":
            value = CodecNestedParentBodyListList(value)
        object.__setattr__(self, name, value)

    @classmethod
    def decode(cls, cursor: SceCursor) -> Optional[CodecNestedParent]:
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
            f_hdr = raw[0]
            cursor.advance(1)
            raw = cursor.peek_slice(1)
            f_m = raw[0]
            cursor.advance(1)
            f_required_body = CodecNestedBody.decode(cursor)
            if f_required_body is None:
                return None
            if (f_hdr & 0x01) != 0:
                f_optional_body = CodecNestedBody.decode(cursor)
                if f_optional_body is None:
                    return None
            else:
                f_optional_body = None
            if f_m > 4:
                raise TooManyElements()
            f_body_list = []
            for _ in range(f_m):
                _elem = CodecNestedBody.decode(cursor)
                if _elem is None:
                    return None
                f_body_list.append(_elem)
        except CodecError:
            return None
        return cls(
            hdr=f_hdr,
            required_body=f_required_body,
            optional_body=f_optional_body,
            body_list=f_body_list,
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
        encode_source(self, w)

    def encode_to_bytes(self) -> bytes:
        """Heap-backed convenience facade. Runs :meth:`encode` over a
        :class:`BytearraySink` and returns the freshly-encoded bytes.
        Callers targeting zero-alloc hot paths should call :meth:`encode`
        directly against a caller-owned sink."""
        _dst = bytearray()
        self.encode(BytearraySink(_dst))
        return bytes(_dst)
