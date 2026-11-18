# SCE-MAP: codec_chain_has_marker:28 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, LengthOutOfRange, NeedMoreBytes, SceCursor, SceSink, TooManyElements
from sce_forge_runtime.bounded import BoundedBytes, BoundedInt, BoundedTuple
from sce_forge_runtime.membership import Chain, Placed
from .codec_zenoh_ext_entry import CodecZenohExtEntry
from .codec_chain_has_marker_slice import CodecChainHasMarkerSlice
from . import codec_chain_has_marker_slice as sce_codec_chain_has_marker_slice
from . import codec_zenoh_ext_entry as sce_codec_zenoh_ext_entry

from dataclasses import dataclass, field
from typing import Optional, List, Protocol, Union


class CodecChainHasMarkerHeaderInt(BoundedInt):
    """The ``header`` element of :class:`CodecChainHasMarker`, an unsigned integer of
    8 bits, held to them where a value of it is made (docs/adr/0081 §2)."""

    BITS = 8
    SIGNED = False


class CodecChainHasMarkerSlicesList(BoundedTuple):
    """The ``slices`` element of :class:`CodecChainHasMarker`, 0 to 4 entries,
    held to that bound where a value of it is made (docs/adr/0077 §3)."""

    LEAST = 0
    MOST = 4


class CodecChainHasMarkerPayloadBytes(BoundedBytes):
    """The ``payload`` element of :class:`CodecChainHasMarker`, 0 to 256 bytes,
    held to that bound where a value of it is made (docs/adr/0077 §3)."""

    LEAST = 0
    MOST = 256


class CodecChainHasMarkerExtensionsHas0x12Key:
    """The kind of entry of ``extensions`` of :class:`CodecChainHasMarker` a gate or a
    law asks for: an entry whose identifier is 0x12 (docs/adr/0065 §6 R6)."""

    @staticmethod
    def matches(entry: sce_codec_zenoh_ext_entry.CodecZenohExtEntrySource) -> bool:
        """Whether ``entry`` is of the kind."""
        return (entry.header & 0x7F) == 0x12

    @staticmethod
    def named() -> sce_codec_zenoh_ext_entry.CodecZenohExtEntry:
        """The entry of the kind: its identifier, and nothing else to say."""
        return sce_codec_zenoh_ext_entry.CodecZenohExtEntry(header=0x12, body=sce_codec_zenoh_ext_entry.ARM_CLASSES[0x0]())


class CodecChainHasMarkerExtensionsHas0x12Has0x12Placed(Placed):
    """The ``extensions`` chain :class:`CodecChainHasMarkerExtensionsHas0x12Has0x12` holds: the entries other than
    the one of the kind CodecChainHasMarkerExtensionsHas0x12Key, which is its identifier alone, and its place
    among them (docs/adr/0065 §6 R6a)."""

    LEAST = 1
    MOST = 4
    KEY = CodecChainHasMarkerExtensionsHas0x12Key


@dataclass
class CodecChainHasMarkerExtensionsHas0x12Has0x12:
    """The ``extensions_has_0x12`` group of :class:`CodecChainHasMarker` where ``extensions.has(0x12)`` holds
    (docs/adr/0065 §6 R4): the members present then."""

    extensions: CodecChainHasMarkerExtensionsHas0x12Has0x12Placed = field(default_factory=CodecChainHasMarkerExtensionsHas0x12Has0x12Placed)
    slices: CodecChainHasMarkerSlicesList = CodecChainHasMarkerSlicesList()

    def __setattr__(self, name: str, value: object) -> None:
        # Each integer, byte string and text is held to its bound
        # wherever it is set, where the value is made and after
        # (docs/adr/0081 §2), whatever it was handed.
        if name == "extensions":
            value = CodecChainHasMarkerExtensionsHas0x12Has0x12Placed(value.rest, value.place)
        elif name == "slices":
            value = CodecChainHasMarkerSlicesList(value)
        object.__setattr__(self, name, value)


class CodecChainHasMarkerExtensionsHas0x12NotHas0x12Chain(Chain):
    """The ``extensions`` chain :class:`CodecChainHasMarkerExtensionsHas0x12NotHas0x12` holds: an entry of each kind of
    ``HOLDS`` and of none of ``LACKS`` (docs/adr/0065 §6 R6b)."""

    LEAST = 1
    MOST = 4
    HOLDS = ()
    LACKS = (CodecChainHasMarkerExtensionsHas0x12Key,)


@dataclass
class CodecChainHasMarkerExtensionsHas0x12NotHas0x12:
    """The ``extensions_has_0x12`` group of :class:`CodecChainHasMarker` where ``extensions.has(0x12)`` does not hold
    (docs/adr/0065 §6 R4): the members present then."""

    extensions: Optional[CodecChainHasMarkerExtensionsHas0x12NotHas0x12Chain] = None
    payload: CodecChainHasMarkerPayloadBytes = CodecChainHasMarkerPayloadBytes()

    def __setattr__(self, name: str, value: object) -> None:
        # Each integer, byte string and text is held to its bound
        # wherever it is set, where the value is made and after
        # (docs/adr/0081 §2), whatever it was handed.
        if name == "extensions":
            value = None if value is None else CodecChainHasMarkerExtensionsHas0x12NotHas0x12Chain(value)
        elif name == "payload":
            value = CodecChainHasMarkerPayloadBytes(value)
        object.__setattr__(self, name, value)


CodecChainHasMarkerExtensionsHas0x12 = Union[CodecChainHasMarkerExtensionsHas0x12Has0x12, CodecChainHasMarkerExtensionsHas0x12NotHas0x12]
"""The ``extensions_has_0x12`` group of :class:`CodecChainHasMarker`: the arm the value holds."""


class CodecChainHasMarkerSource(Protocol):
    """The elements a value of :class:`CodecChainHasMarker` presents, which its encode
    reads (docs/adr/0081 §2): any object that presents them is a source of it.
    The encode derives every other element of the frame."""

    @property
    def header(self) -> CodecChainHasMarkerHeaderInt: ...

    @property
    def extensions_has_0x12(self) -> CodecChainHasMarkerExtensionsHas0x12: ...


def encode_source(src: CodecChainHasMarkerSource, w: SceSink) -> None:
    """Write any source of :class:`CodecChainHasMarker` into ``w``, each element read
    off ``src`` and written where the codec's encode plan lays it out
    (docs/adr/0081). It derives ``payload_len``, ``slice_count`` from what each measures, and never reads it off
    the source. It refuses no value; a bounded sink raises
    :class:`BufferOverflow` where it has no room left."""
    _g_extensions_has_0x12 = src.extensions_has_0x12
    _e_extensions = _g_extensions_has_0x12.extensions
    _e_slices = _g_extensions_has_0x12.slices if isinstance(_g_extensions_has_0x12, CodecChainHasMarkerExtensionsHas0x12Has0x12) else None
    _e_payload = _g_extensions_has_0x12.payload if isinstance(_g_extensions_has_0x12, CodecChainHasMarkerExtensionsHas0x12NotHas0x12) else None
    _d_payload_len = (len(_e_payload)) if _e_payload is not None else 0
    _d_slice_count = (len(_e_slices)) if _e_slices is not None else 0
    _c_header = src.header
    _c_header = (_c_header & ~0x80) | (0x80 if _e_extensions is not None else 0)
    w.write_u8(_c_header & 0xFF)
    if _e_extensions is not None:
        _left_extensions = len(_e_extensions)
        for _entry in _e_extensions:
            _left_extensions -= 1
            sce_codec_zenoh_ext_entry.encode_source(_entry, w, override_header_z=1 if _left_extensions != 0 else 0)
    if not (isinstance(_g_extensions_has_0x12, CodecChainHasMarkerExtensionsHas0x12Has0x12)):
        w.write_vle_u64(_d_payload_len)
    if _e_payload is not None:
        w.write_bytes(_e_payload)
    if (isinstance(_g_extensions_has_0x12, CodecChainHasMarkerExtensionsHas0x12Has0x12)):
        w.write_vle_u32(_d_slice_count)
    if _e_slices is not None:
        for _entry in _e_slices:
            sce_codec_chain_has_marker_slice.encode_source(_entry, w)


@dataclass
class CodecChainHasMarker:
    header: CodecChainHasMarkerHeaderInt = 0
    extensions_has_0x12: CodecChainHasMarkerExtensionsHas0x12 = field(default_factory=CodecChainHasMarkerExtensionsHas0x12NotHas0x12)

    def __setattr__(self, name: str, value: object) -> None:
        # Each integer, byte string and text is held to its bound
        # wherever it is set, where the value is made and after
        # (docs/adr/0081 §2), whatever it was handed.
        if name == "header":
            value = CodecChainHasMarkerHeaderInt(value)
        object.__setattr__(self, name, value)

    @classmethod
    def decode(cls, cursor: SceCursor) -> CodecChainHasMarker:
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
        if (f_header & 0x80) != 0:
            f_extensions = []
            _more = False
            for _ in range(4):
                _elem = CodecZenohExtEntry.decode(cursor)
                _more = _elem.z()
                f_extensions.append(_elem)
                if not _more:
                    break
            if _more and cursor.remaining() == 0:
                raise NeedMoreBytes()
            if _more:
                raise TooManyElements()
        else:
            f_extensions = None
        _has_extensions_18 = f_extensions is not None and any((_e.header & 127) == 18 for _e in f_extensions)
        if not _has_extensions_18:
            _v = cursor.read_vle_u64()
            f_payload_len = _v
        else:
            f_payload_len = None
        if not _has_extensions_18:
            _len = f_payload_len
            if _len > 256:
                raise LengthOutOfRange()
            _n = _len
            raw = cursor.peek_slice(_n)
            _v = bytes(raw)
            cursor.advance(_n)
            f_payload = _v
        else:
            f_payload = None
        if _has_extensions_18:
            _v = cursor.read_vle_u32()
            f_slice_count = _v
        else:
            f_slice_count = None
        if _has_extensions_18:
            if f_slice_count > 4:
                raise TooManyElements()
            f_slices = []
            for _ in range(f_slice_count):
                f_slices.append(CodecChainHasMarkerSlice.decode(cursor))
        else:
            f_slices = None
        return cls(
            header=f_header,
            extensions_has_0x12=CodecChainHasMarkerExtensionsHas0x12Has0x12(extensions=CodecChainHasMarkerExtensionsHas0x12Has0x12Placed.place_first(f_extensions), slices=f_slices) if ((f_extensions is not None and any((_x.header & 0x7F) == 0x12 for _x in f_extensions))) else CodecChainHasMarkerExtensionsHas0x12NotHas0x12(extensions=None if f_extensions is None else CodecChainHasMarkerExtensionsHas0x12NotHas0x12Chain(f_extensions), payload=f_payload),
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
        encode_source(self, w)

    def encode_to_bytes(self) -> bytes:
        """Heap-backed convenience facade. Runs :meth:`encode` over a
        :class:`BytearraySink` and returns the freshly-encoded bytes.
        Callers targeting zero-alloc hot paths should call :meth:`encode`
        directly against a caller-owned sink."""
        _dst = bytearray()
        self.encode(BytearraySink(_dst))
        return bytes(_dst)
