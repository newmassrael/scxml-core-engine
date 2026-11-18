# SCE-MAP: codec_zenoh_err:57 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, LengthOutOfRange, NeedMoreBytes, SceCursor, SceSink, TooManyElements
from sce_forge_runtime.bounded import BoundedBytes, BoundedInt, BoundedTuple
from .codec_zenoh_encoding import CodecZenohEncoding
from .codec_zenoh_ext_entry import CodecZenohExtEntry
from . import codec_zenoh_encoding as sce_codec_zenoh_encoding
from . import codec_zenoh_ext_entry as sce_codec_zenoh_ext_entry

from dataclasses import dataclass, field
from typing import Optional, List, Protocol


class CodecZenohErrHeaderInt(BoundedInt):
    """The ``header`` element of :class:`CodecZenohErr`, an unsigned integer of
    8 bits, held to them where a value of it is made (docs/adr/0081 §2)."""

    BITS = 8
    SIGNED = False


class CodecZenohErrExtensionsList(BoundedTuple):
    """The ``extensions`` element of :class:`CodecZenohErr`, 1 or more entries, the protocol stating no most,
    held to that bound where a value of it is made (docs/adr/0077 §3)."""

    LEAST = 1
    MOST = None


class CodecZenohErrPayloadBytes(BoundedBytes):
    """The ``payload`` element of :class:`CodecZenohErr`, 0 to 4294967295 bytes,
    held to that bound where a value of it is made (docs/adr/0077 §3)."""

    LEAST = 0
    MOST = 4294967295


class CodecZenohErrSource(Protocol):
    """The elements a value of :class:`CodecZenohErr` presents, which its encode
    reads (docs/adr/0081 §2): any object that presents them is a source of it.
    The encode derives every other element of the frame."""

    @property
    def header(self) -> CodecZenohErrHeaderInt: ...

    @property
    def encoding(self) -> Optional[sce_codec_zenoh_encoding.CodecZenohEncodingSource]: ...

    @property
    def extensions(self) -> Optional[CodecZenohErrExtensionsList]: ...

    @property
    def payload(self) -> CodecZenohErrPayloadBytes: ...


def encode_source(src: CodecZenohErrSource, w: SceSink, *, override_header_x: Optional[int] = None) -> None:
    """Write any source of :class:`CodecZenohErr` into ``w``, each element read
    off ``src`` and written where the codec's encode plan lays it out
    (docs/adr/0081). It derives ``payload_len`` from what each measures, and never reads it off
    the source. A chain that holds it
    sets the flag its entries end on by its keyword, in place of the value's. It refuses no value; a bounded sink raises
    :class:`BufferOverflow` where it has no room left."""
    _d_payload_len = len(src.payload)
    _c_header = src.header
    _c_header = (_c_header & ~0x40) | (0x40 if src.encoding is not None else 0)
    _c_header = (_c_header & ~0x80) | (0x80 if src.extensions is not None else 0)
    if override_header_x is not None:
        _c_header = (_c_header & ~0x20) | ((override_header_x & 0x1) << 5)
    w.write_u8(_c_header & 0xFF)
    if src.encoding is not None:
        sce_codec_zenoh_encoding.encode_source(src.encoding, w)
    if src.extensions is not None:
        _left_extensions = len(src.extensions)
        for _entry in src.extensions:
            _left_extensions -= 1
            sce_codec_zenoh_ext_entry.encode_source(_entry, w, override_header_z=1 if _left_extensions != 0 else 0)
    w.write_vle_u64(_d_payload_len)
    w.write_bytes(src.payload)


@dataclass
class CodecZenohErr:
    header: CodecZenohErrHeaderInt = 0x05
    encoding: Optional[CodecZenohEncoding] = None
    extensions: Optional[CodecZenohErrExtensionsList] = None
    payload: CodecZenohErrPayloadBytes = CodecZenohErrPayloadBytes()

    def __setattr__(self, name: str, value: object) -> None:
        # Each integer, byte string and text is held to its bound
        # wherever it is set, where the value is made and after
        # (docs/adr/0081 §2), whatever it was handed.
        if name == "header":
            value = CodecZenohErrHeaderInt(value)
        elif name == "extensions":
            value = None if value is None else CodecZenohErrExtensionsList(value)
        elif name == "payload":
            value = CodecZenohErrPayloadBytes(value)
        object.__setattr__(self, name, value)

    @classmethod
    def decode(cls, cursor: SceCursor) -> CodecZenohErr:
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
        if (f_header & 0x40) != 0:
            f_encoding = CodecZenohEncoding.decode(cursor)
        else:
            f_encoding = None
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
        f_payload_len = cursor.read_vle_u64()
        _len = f_payload_len
        if _len > 4294967295:
            raise LengthOutOfRange()
        _n = _len
        raw = cursor.peek_slice(_n)
        f_payload = bytes(raw)
        cursor.advance(_n)
        return cls(
            header=f_header,
            encoding=f_encoding,
            extensions=f_extensions,
            payload=f_payload,
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

    def x(self) -> bool:
        return (self.header & 0x20) != 0

    def set_x(self, v: bool) -> None:
        if v:
            self.header = (self.header | 0x20) & 0xFF
        else:
            self.header = self.header & (0xFF ^ 0x20)

    def e(self) -> bool:
        return (self.header & 0x40) != 0

    def set_e(self, v: bool) -> None:
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
