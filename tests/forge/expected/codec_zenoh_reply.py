# SCE-MAP: codec_zenoh_reply:54 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, SceCursor, SceSink, TooManyElements
from sce_forge_runtime.bounded import BoundedInt, BoundedTuple
from .codec_zenoh_ext_entry import CodecZenohExtEntry
from .codec_zenoh_push_body import CodecZenohPushBody
from . import codec_zenoh_ext_entry as sce_codec_zenoh_ext_entry
from . import codec_zenoh_push_body as sce_codec_zenoh_push_body

from dataclasses import dataclass, field
from typing import Optional, List, Protocol


class CodecZenohReplyHeaderInt(BoundedInt):
    """The ``header`` element of :class:`CodecZenohReply`, an unsigned integer of
    8 bits, held to them where a value of it is made (docs/adr/0081 §2)."""

    BITS = 8
    SIGNED = False


class CodecZenohReplyConsolidationInt(BoundedInt):
    """The ``consolidation`` element of :class:`CodecZenohReply`, an unsigned integer of
    8 bits, held to them where a value of it is made (docs/adr/0081 §2)."""

    BITS = 8
    SIGNED = False


class CodecZenohReplyExtensionsList(BoundedTuple):
    """The ``extensions`` element of :class:`CodecZenohReply`, 1 or more entries, the protocol stating no most,
    held to that bound where a value of it is made (docs/adr/0077 §3)."""

    LEAST = 1
    MOST = None


class CodecZenohReplySource(Protocol):
    """The elements a value of :class:`CodecZenohReply` presents, which its encode
    reads (docs/adr/0081 §2): any object that presents them is a source of it.
    The encode derives every other element of the frame."""

    @property
    def header(self) -> CodecZenohReplyHeaderInt: ...

    @property
    def consolidation(self) -> Optional[CodecZenohReplyConsolidationInt]: ...

    @property
    def extensions(self) -> Optional[CodecZenohReplyExtensionsList]: ...

    @property
    def body(self) -> sce_codec_zenoh_push_body.CodecZenohPushBodySource: ...


def encode_source(src: CodecZenohReplySource, w: SceSink, *, override_header_x: Optional[int] = None) -> None:
    """Write any source of :class:`CodecZenohReply` into ``w``, each element read
    off ``src`` and written where the codec's encode plan lays it out
    (docs/adr/0081). A chain that holds it
    sets the flag its entries end on by its keyword, in place of the value's. It refuses no value; a bounded sink raises
    :class:`BufferOverflow` where it has no room left."""
    _c_header = src.header
    _c_header = (_c_header & ~0x20) | (0x20 if src.consolidation is not None else 0)
    _c_header = (_c_header & ~0x80) | (0x80 if src.extensions is not None else 0)
    if override_header_x is not None:
        _c_header = (_c_header & ~0x40) | ((override_header_x & 0x1) << 6)
    w.write_u8(_c_header & 0xFF)
    if src.consolidation is not None:
        w.write_u8(src.consolidation & 0xFF)
    if src.extensions is not None:
        _left_extensions = len(src.extensions)
        for _entry in src.extensions:
            _left_extensions -= 1
            sce_codec_zenoh_ext_entry.encode_source(_entry, w, override_header_z=1 if _left_extensions != 0 else 0)
    sce_codec_zenoh_push_body.encode_source(src.body, w)


@dataclass
class CodecZenohReply:
    header: CodecZenohReplyHeaderInt = 0x04
    consolidation: Optional[CodecZenohReplyConsolidationInt] = None
    extensions: Optional[CodecZenohReplyExtensionsList] = None
    body: CodecZenohPushBody = field(default_factory=CodecZenohPushBody)

    def __setattr__(self, name: str, value: object) -> None:
        # Each integer, byte string and text is held to its bound
        # wherever it is set, where the value is made and after
        # (docs/adr/0081 §2), whatever it was handed.
        if name == "header":
            value = CodecZenohReplyHeaderInt(value)
        elif name == "consolidation":
            value = None if value is None else CodecZenohReplyConsolidationInt(value)
        elif name == "extensions":
            value = None if value is None else CodecZenohReplyExtensionsList(value)
        object.__setattr__(self, name, value)

    @classmethod
    def decode(cls, cursor: SceCursor) -> CodecZenohReply:
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
        f_header = raw[0]
        cursor.advance(1)
        if (f_header & 0x20) != 0:
            raw = cursor.peek_slice(1)
            _v = raw[0]
            cursor.advance(1)
            f_consolidation = _v
        else:
            f_consolidation = None
        if (f_header & 0x80) != 0:
            f_extensions = []
            _more = False
            while True:
                _elem = CodecZenohExtEntry.decode(cursor)
                _more = _elem.z()
                f_extensions.append(_elem)
                if not _more:
                    break
            if _more and cursor.remaining() == 0:
                raise NeedMoreBytes()
        else:
            f_extensions = None
        f_body = CodecZenohPushBody.decode(cursor)
        return cls(
            header=f_header,
            consolidation=f_consolidation,
            extensions=f_extensions,
            body=f_body,
        )

    # RFC §synth-5-B flags primitive: per-bit-range accessors over
    # the carrier field. Single-bit (width=1) reads as bool; multi-bit
    # (width>=2) reads as ``int`` (Python ints are unbounded, so a single
    # ``int`` covers every result-type width). Setters mask + shift on
    # the way in so out-of-range callers can't corrupt sibling bits.
    # Plain methods (rather than @property) for API symmetry with
    # Rust / Cpp / Kotlin / Go / C11. Wire layout is unchanged.
    def mid(self) -> int:
        return (self.header >> 0) & 0x1F

    def set_mid(self, v: int) -> None:
        _shifted_mask = 0x1F << 0
        _val = (v & 0x1F) << 0
        self.header = ((self.header & (0xFF ^ _shifted_mask)) | _val) & 0xFF

    def c(self) -> bool:
        return (self.header & 0x20) != 0

    def set_c(self, v: bool) -> None:
        if v:
            self.header = (self.header | 0x20) & 0xFF
        else:
            self.header = self.header & (0xFF ^ 0x20)

    def x(self) -> bool:
        return (self.header & 0x40) != 0

    def set_x(self, v: bool) -> None:
        if v:
            self.header = (self.header | 0x40) & 0xFF
        else:
            self.header = self.header & (0xFF ^ 0x40)

    def z(self) -> bool:
        return (self.header & 0x80) != 0

    def set_z(self, v: bool) -> None:
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
        encode_source(self, w)

    def encode_to_bytes(self) -> bytes:
        """Heap-backed convenience facade. Runs :meth:`encode` over a
        :class:`BytearraySink` and returns the freshly-encoded bytes.
        Callers targeting zero-alloc hot paths should call :meth:`encode`
        directly against a caller-owned sink."""
        _dst = bytearray()
        self.encode(BytearraySink(_dst))
        return bytes(_dst)
