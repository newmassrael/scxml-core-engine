# SCE-MAP: codec_origin_envelope:19 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, InvalidUtf8, LengthOutOfRange, NeedMoreBytes, SceCursor, SceSink, TooManyElements
from sce_forge_runtime.bounded import BoundedText, BoundedTuple
from .codec_origin_leaf import CodecOriginLeaf
from . import codec_origin_leaf as sce_codec_origin_leaf

from dataclasses import dataclass, field
from typing import Optional, List, Protocol


class CodecOriginEnvelopeNoteText(BoundedText):
    """The ``note`` element of :class:`CodecOriginEnvelope`, 0 to 16 UTF-8 bytes,
    held to that bound where a value of it is made (docs/adr/0077 §3)."""

    LEAST = 0
    MOST = 16


class CodecOriginEnvelopeItemsList(BoundedTuple):
    """The ``items`` element of :class:`CodecOriginEnvelope`, 0 to 3 entries,
    held to that bound where a value of it is made (docs/adr/0077 §3)."""

    LEAST = 0
    MOST = 3


class CodecOriginEnvelopeSource(Protocol):
    """The elements a value of :class:`CodecOriginEnvelope` presents, which its encode
    reads (docs/adr/0081 §2): any object that presents them is a source of it.
    The encode derives every other element of the frame."""

    @property
    def hdr(self) -> int: ...

    @property
    def note(self) -> CodecOriginEnvelopeNoteText: ...

    @property
    def required(self) -> sce_codec_origin_leaf.CodecOriginLeafSource: ...

    @property
    def optional(self) -> Optional[sce_codec_origin_leaf.CodecOriginLeafSource]: ...

    @property
    def items(self) -> CodecOriginEnvelopeItemsList: ...


def encode_source(src: CodecOriginEnvelopeSource, w: SceSink) -> None:
    """Write any source of :class:`CodecOriginEnvelope` into ``w``, each element read
    off ``src`` and written where the codec's encode plan lays it out
    (docs/adr/0081). It derives ``m``, ``note_len`` from what each measures, and never reads it off
    the source. It refuses no value; a bounded sink raises
    :class:`BufferOverflow` where it has no room left."""
    _d_note_len = len(src.note.encode("utf-8"))
    _d_m = len(src.items)
    _c_hdr = src.hdr
    _c_hdr = (_c_hdr & ~0x1) | (0x1 if src.optional is not None else 0)
    w.write_u8(_c_hdr & 0xFF)
    w.write_u8(_d_note_len & 0xFF)
    w.write_bytes(src.note.encode("utf-8"))
    w.write_u8(_d_m & 0xFF)
    sce_codec_origin_leaf.encode_source(src.required, w)
    if src.optional is not None:
        sce_codec_origin_leaf.encode_source(src.optional, w)
    for _entry in src.items:
        sce_codec_origin_leaf.encode_source(_entry, w)


@dataclass
class CodecOriginEnvelope:
    hdr: int = 0
    note: CodecOriginEnvelopeNoteText = CodecOriginEnvelopeNoteText()
    required: CodecOriginLeaf = field(default_factory=CodecOriginLeaf)
    optional: Optional[CodecOriginLeaf] = None
    items: CodecOriginEnvelopeItemsList = CodecOriginEnvelopeItemsList()

    def __setattr__(self, name: str, value: object) -> None:
        # Each byte string and text is held to its bound wherever it is
        # set, where the value is made and after (docs/adr/0081 §2),
        # whatever it was handed.
        if name == "note":
            value = CodecOriginEnvelopeNoteText(value)
        elif name == "items":
            value = CodecOriginEnvelopeItemsList(value)
        object.__setattr__(self, name, value)

    @classmethod
    def decode(cls, cursor: SceCursor) -> Optional[CodecOriginEnvelope]:
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
            f_note_len = raw[0]
            cursor.advance(1)
            _len = f_note_len
            if _len > 16:
                raise LengthOutOfRange()
            _n = _len
            raw = cursor.peek_slice(_n)
            try:
                f_note = bytes(raw).decode('utf-8')
            except UnicodeDecodeError as exc:
                raise InvalidUtf8() from exc
            cursor.advance(_n)
            raw = cursor.peek_slice(1)
            f_m = raw[0]
            cursor.advance(1)
            f_required = CodecOriginLeaf.decode(cursor)
            if f_required is None:
                return None
            if (f_hdr & 0x01) != 0:
                f_optional = CodecOriginLeaf.decode(cursor)
                if f_optional is None:
                    return None
            else:
                f_optional = None
            if f_m > 3:
                raise TooManyElements()
            f_items = []
            for _ in range(f_m):
                _elem = CodecOriginLeaf.decode(cursor)
                if _elem is None:
                    return None
                f_items.append(_elem)
        except CodecError:
            return None
        return cls(
            hdr=f_hdr,
            note=f_note,
            required=f_required,
            optional=f_optional,
            items=f_items,
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
