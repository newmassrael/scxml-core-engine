# SCE-MAP: codec_chain_prev_entry:19 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, SceCursor, SceSink
from sce_forge_runtime.bounded import BoundedInt

from dataclasses import dataclass, field
from typing import Optional, Protocol, Union


class CodecChainPrevEntryHeaderInt(BoundedInt):
    """The ``header`` element of :class:`CodecChainPrevEntry`, an unsigned integer of
    8 bits, held to them where a value of it is made (docs/adr/0081 §2)."""

    BITS = 8
    SIGNED = False


class CodecChainPrevEntryTagInt(BoundedInt):
    """The ``tag`` element of :class:`CodecChainPrevEntry`, an unsigned integer of
    8 bits, held to them where a value of it is made (docs/adr/0081 §2)."""

    BITS = 8
    SIGNED = False


class CodecChainPrevEntryLongValueInt(BoundedInt):
    """The ``long_value`` element of :class:`CodecChainPrevEntry`, an unsigned integer of
    16 bits, held to them where a value of it is made (docs/adr/0081 §2)."""

    BITS = 16
    SIGNED = False


class CodecChainPrevEntryShortValueInt(BoundedInt):
    """The ``short_value`` element of :class:`CodecChainPrevEntry`, an unsigned integer of
    8 bits, held to them where a value of it is made (docs/adr/0081 §2)."""

    BITS = 8
    SIGNED = False


@dataclass
class CodecChainPrevEntryAfterMarkerAfterMarker:
    """The ``after_marker`` group of :class:`CodecChainPrevEntry` where ``after_marker`` holds
    (docs/adr/0065 §6 R4): the members present then."""

    long_value: CodecChainPrevEntryLongValueInt = 0

    def __setattr__(self, name: str, value: object) -> None:
        # Each integer, byte string and text is held to its bound
        # wherever it is set, where the value is made and after
        # (docs/adr/0081 §2), whatever it was handed.
        if name == "long_value":
            value = CodecChainPrevEntryLongValueInt(value)
        object.__setattr__(self, name, value)


@dataclass
class CodecChainPrevEntryAfterMarkerNotAfterMarker:
    """The ``after_marker`` group of :class:`CodecChainPrevEntry` where ``after_marker`` does not hold
    (docs/adr/0065 §6 R4): the members present then."""

    short_value: CodecChainPrevEntryShortValueInt = 0

    def __setattr__(self, name: str, value: object) -> None:
        # Each integer, byte string and text is held to its bound
        # wherever it is set, where the value is made and after
        # (docs/adr/0081 §2), whatever it was handed.
        if name == "short_value":
            value = CodecChainPrevEntryShortValueInt(value)
        object.__setattr__(self, name, value)


CodecChainPrevEntryAfterMarker = Union[CodecChainPrevEntryAfterMarkerAfterMarker, CodecChainPrevEntryAfterMarkerNotAfterMarker]
"""The ``after_marker`` group of :class:`CodecChainPrevEntry`: the arm the value holds."""


class CodecChainPrevEntrySource(Protocol):
    """The elements a value of :class:`CodecChainPrevEntry` presents, which its encode
    reads (docs/adr/0081 §2): any object that presents them is a source of it.
    The encode derives every other element of the frame."""

    @property
    def header(self) -> CodecChainPrevEntryHeaderInt: ...

    @property
    def tag(self) -> Optional[CodecChainPrevEntryTagInt]: ...

    @property
    def after_marker(self) -> CodecChainPrevEntryAfterMarker: ...


def encode_source(src: CodecChainPrevEntrySource, w: SceSink, *, override_header_z: Optional[int] = None) -> None:
    """Write any source of :class:`CodecChainPrevEntry` into ``w``, each element read
    off ``src`` and written where the codec's encode plan lays it out
    (docs/adr/0081). A chain that holds it
    sets the flag its entries end on by its keyword, in place of the value's. It refuses no value; a bounded sink raises
    :class:`BufferOverflow` where it has no room left."""
    _g_after_marker = src.after_marker
    _e_long_value = _g_after_marker.long_value if isinstance(_g_after_marker, CodecChainPrevEntryAfterMarkerAfterMarker) else None
    _e_short_value = _g_after_marker.short_value if isinstance(_g_after_marker, CodecChainPrevEntryAfterMarkerNotAfterMarker) else None
    _c_header = src.header
    if override_header_z is not None:
        _c_header = (_c_header & ~0x80) | ((override_header_z & 0x1) << 7)
    w.write_u8(_c_header & 0xFF)
    if _e_short_value is not None:
        w.write_u8(_e_short_value & 0xFF)
    if _e_long_value is not None:
        w.write_u8((_e_long_value >> 8) & 0xFF)
        w.write_u8(_e_long_value & 0xFF)
    if src.tag is not None:
        w.write_u8(src.tag & 0xFF)


def input_after_marker(src: CodecChainPrevEntrySource) -> int:
    """The flag input ``after_marker`` a source of :class:`CodecChainPrevEntry` says, which
    its encode is not handed (docs/adr/0065 §6 R4a)."""
    return (1 if isinstance(src.after_marker, CodecChainPrevEntryAfterMarkerAfterMarker) else 0)



def input_tagged(src: CodecChainPrevEntrySource) -> int:
    """The flag input ``tagged`` a source of :class:`CodecChainPrevEntry` says, which
    its encode is not handed (docs/adr/0065 §6 R4a)."""
    return (1 if src.tag is not None else 0)


@dataclass
class CodecChainPrevEntry:
    header: CodecChainPrevEntryHeaderInt = 0
    tag: Optional[CodecChainPrevEntryTagInt] = None
    after_marker: CodecChainPrevEntryAfterMarker = field(default_factory=CodecChainPrevEntryAfterMarkerNotAfterMarker)

    def __setattr__(self, name: str, value: object) -> None:
        # Each integer, byte string and text is held to its bound
        # wherever it is set, where the value is made and after
        # (docs/adr/0081 §2), whatever it was handed.
        if name == "header":
            value = CodecChainPrevEntryHeaderInt(value)
        elif name == "tag":
            value = None if value is None else CodecChainPrevEntryTagInt(value)
        object.__setattr__(self, name, value)

    @classmethod
    def decode(cls, cursor: SceCursor, after_marker: int, tagged: int) -> CodecChainPrevEntry:
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
        if (after_marker & 0x01) == 0:
            raw = cursor.peek_slice(1)
            _v = raw[0]
            cursor.advance(1)
            f_short_value = _v
        else:
            f_short_value = None
        if (after_marker & 0x01) != 0:
            raw = cursor.peek_slice(2)
            _v = (raw[0] << 8) | raw[1]
            cursor.advance(2)
            f_long_value = _v
        else:
            f_long_value = None
        if (tagged & 0x01) != 0:
            raw = cursor.peek_slice(1)
            _v = raw[0]
            cursor.advance(1)
            f_tag = _v
        else:
            f_tag = None
        return cls(
            header=f_header,
            tag=f_tag,
            after_marker=CodecChainPrevEntryAfterMarkerAfterMarker(long_value=f_long_value) if (after_marker != 0) else CodecChainPrevEntryAfterMarkerNotAfterMarker(short_value=f_short_value),
        )

    # RFC §synth-5-B flags primitive: per-bit-range accessors over
    # the carrier field. Single-bit (width=1) reads as bool; multi-bit
    # (width>=2) reads as ``int`` (Python ints are unbounded, so a single
    # ``int`` covers every result-type width). Setters mask + shift on
    # the way in so out-of-range callers can't corrupt sibling bits.
    # Plain methods (rather than @property) for API symmetry with
    # Rust / Cpp / Kotlin / Go / C11. Wire layout is unchanged.
    def ext_id(self) -> int:
        return (self.header >> 0) & 0x0F

    def set_ext_id(self, v: int) -> None:
        _shifted_mask = 0x0F << 0
        _val = (v & 0x0F) << 0
        self.header = ((self.header & (0xFF ^ _shifted_mask)) | _val) & 0xFF

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
