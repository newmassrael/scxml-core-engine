# SCE-MAP: codec_zenoh_oam:56 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, SceCursor, SceSink, TooManyElements
from sce_forge_runtime.bounded import BoundedInt, BoundedTuple, UnknownTag
from .codec_zenoh_ext_entry import CodecZenohExtEntry
from .codec_zenoh_ext_unit import CodecZenohExtUnit
from .codec_zenoh_ext_zint import CodecZenohExtZint
from .codec_zenoh_ext_zbuf import CodecZenohExtZbuf
from . import codec_zenoh_ext_entry as sce_codec_zenoh_ext_entry
from . import codec_zenoh_ext_unit as sce_codec_zenoh_ext_unit
from . import codec_zenoh_ext_zbuf as sce_codec_zenoh_ext_zbuf
from . import codec_zenoh_ext_zint as sce_codec_zenoh_ext_zint

from dataclasses import dataclass, field
from typing import Optional, List, Protocol, Union


class CodecZenohOamHeaderInt(BoundedInt):
    """The ``header`` element of :class:`CodecZenohOam`, an unsigned integer of
    8 bits, held to them where a value of it is made (docs/adr/0081 §2)."""

    BITS = 8
    SIGNED = False


class CodecZenohOamIdInt(BoundedInt):
    """The ``id`` element of :class:`CodecZenohOam`, an unsigned integer of
    16 bits, held to them where a value of it is made (docs/adr/0081 §2)."""

    BITS = 16
    SIGNED = False


class CodecZenohOamExtensionsList(BoundedTuple):
    """The ``extensions`` element of :class:`CodecZenohOam`, 1 or more entries, the protocol stating no most,
    held to that bound where a value of it is made (docs/adr/0077 §3)."""

    LEAST = 1
    MOST = None


class CodecZenohOamUnknownTag(UnknownTag):
    """The tag the default arm of :class:`CodecZenohOam`'s variant keeps: at
    most 3, and none an arm names (docs/adr/0065 §6 R3)."""

    MOST = 3
    KNOWN = frozenset({0, 1, 2})


@dataclass
class CodecZenohOamDefault:
    """The default arm of :class:`CodecZenohOam`'s variant: the tag no arm
    names, which it keeps, and its body."""

    tag: CodecZenohOamUnknownTag = CodecZenohOamUnknownTag(3)
    body: sce_codec_zenoh_ext_unit.CodecZenohExtUnit = field(default_factory=sce_codec_zenoh_ext_unit.CodecZenohExtUnit)

    def __setattr__(self, name: str, value: object) -> None:
        # The tag is held to the tags no arm names wherever it is set,
        # where the arm is made and after (docs/adr/0065 §6 R3).
        if name == "tag":
            value = CodecZenohOamUnknownTag(value)
        object.__setattr__(self, name, value)


CodecZenohOamVariant = Union[sce_codec_zenoh_ext_unit.CodecZenohExtUnit, sce_codec_zenoh_ext_zint.CodecZenohExtZint, sce_codec_zenoh_ext_zbuf.CodecZenohExtZbuf, CodecZenohOamDefault]
"""The arm a value of :class:`CodecZenohOam` holds: a value of the arm's codec,
or the default arm."""


ARM_CLASSES = {0x0: sce_codec_zenoh_ext_unit.CodecZenohExtUnit, 0x1: sce_codec_zenoh_ext_zint.CodecZenohExtZint, 0x2: sce_codec_zenoh_ext_zbuf.CodecZenohExtZbuf}
"""The class of the arm of :class:`CodecZenohOam`'s variant each tag names."""


class CodecZenohOamSource(Protocol):
    """The elements a value of :class:`CodecZenohOam` presents, which its encode
    reads (docs/adr/0081 §2): any object that presents them is a source of it.
    The encode derives every other element of the frame."""

    @property
    def header(self) -> CodecZenohOamHeaderInt: ...

    @property
    def id(self) -> CodecZenohOamIdInt: ...

    @property
    def extensions(self) -> Optional[CodecZenohOamExtensionsList]: ...

    @property
    def body(self) -> CodecZenohOamVariant: ...


def encode_source(src: CodecZenohOamSource, w: SceSink) -> None:
    """Write any source of :class:`CodecZenohOam` into ``w``, each element read
    off ``src`` and written where the codec's encode plan lays it out
    (docs/adr/0081). It refuses no value; a bounded sink raises
    :class:`BufferOverflow` where it has no room left."""
    _v_body = src.body
    if not isinstance(_v_body, (sce_codec_zenoh_ext_unit.CodecZenohExtUnit, sce_codec_zenoh_ext_zint.CodecZenohExtZint, sce_codec_zenoh_ext_zbuf.CodecZenohExtZbuf, CodecZenohOamDefault,)):
        raise TypeError(f"{type(_v_body).__name__} is no arm of CodecZenohOam's variant")
    _t_body = (0 if isinstance(_v_body, sce_codec_zenoh_ext_unit.CodecZenohExtUnit) else 1 if isinstance(_v_body, sce_codec_zenoh_ext_zint.CodecZenohExtZint) else 2 if isinstance(_v_body, sce_codec_zenoh_ext_zbuf.CodecZenohExtZbuf) else _v_body.tag)
    _c_header = src.header
    _c_header = (_c_header & ~0x60) | ((_t_body & 0x3) << 5)
    _c_header = (_c_header & ~0x80) | (0x80 if src.extensions is not None else 0)
    w.write_u8(_c_header & 0xFF)
    w.write_vle_u16(src.id)
    if src.extensions is not None:
        _left_extensions = len(src.extensions)
        for _entry in src.extensions:
            _left_extensions -= 1
            sce_codec_zenoh_ext_entry.encode_source(_entry, w, override_header_z=1 if _left_extensions != 0 else 0)
    if isinstance(_v_body, sce_codec_zenoh_ext_unit.CodecZenohExtUnit):
        sce_codec_zenoh_ext_unit.encode_source(_v_body, w)
    elif isinstance(_v_body, sce_codec_zenoh_ext_zint.CodecZenohExtZint):
        sce_codec_zenoh_ext_zint.encode_source(_v_body, w)
    elif isinstance(_v_body, sce_codec_zenoh_ext_zbuf.CodecZenohExtZbuf):
        sce_codec_zenoh_ext_zbuf.encode_source(_v_body, w)
    else:
        sce_codec_zenoh_ext_unit.encode_source(_v_body.body, w)


@dataclass
class CodecZenohOam:
    header: CodecZenohOamHeaderInt = 0x1f
    id: CodecZenohOamIdInt = 0
    extensions: Optional[CodecZenohOamExtensionsList] = None
    body: CodecZenohOamVariant = field(default_factory=sce_codec_zenoh_ext_unit.CodecZenohExtUnit)

    def __setattr__(self, name: str, value: object) -> None:
        # Each integer, byte string and text is held to its bound
        # wherever it is set, where the value is made and after
        # (docs/adr/0081 §2), whatever it was handed.
        if name == "header":
            value = CodecZenohOamHeaderInt(value)
        elif name == "id":
            value = CodecZenohOamIdInt(value)
        elif name == "extensions":
            value = None if value is None else CodecZenohOamExtensionsList(value)
        object.__setattr__(self, name, value)

    @classmethod
    def decode(cls, cursor: SceCursor) -> CodecZenohOam:
        """Decode the next frame from ``cursor``; on success the cursor
        advances past the consumed bytes. A frame the codec refuses raises
        the ``CodecError`` subclass that names the refusal (docs/adr/0099,
        amendment of 2026-11-16): ``NeedMoreBytes`` when the cursor's tail
        is shorter than the frame (RFC §synth-5-B L494-519), which the
        caller resumes after appending bytes, and for a frame no more bytes
        make whole ``VleWidthOverflow``, ``InvalidUtf8``,
        ``UndeclaredEnumValue``, or a length or a count past its bound
        (``LengthOutOfRange``, ``TooManyElements``, docs/adr/0077)."""
        # RFC §synth-5-B peek-byte / streaming-prefix:
        # streaming prefix decode (variable-length fields supported via
        # per-field present_if/tlv-chain/embed/repeat helpers). Peek-byte
        # mode additionally peeks the cursor's next byte for variant tag
        # without advancing — arm body decoder reads it as own header.
        raw = cursor.peek_slice(1)
        f_header = raw[0]
        cursor.advance(1)
        f_id = cursor.read_vle_u16()
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
        # Dispatch on the tag field; each arm decodes its body codec
        # from the cursor. The default arm (when declared) carries the
        # runtime tag value so encode can round-trip it back onto the
        # wire.
        if ((f_header >> 5) & 0x03) == 0:
            _arm = CodecZenohExtUnit.decode(cursor)
            body = _arm
        elif ((f_header >> 5) & 0x03) == 1:
            _arm = CodecZenohExtZint.decode(cursor)
            body = _arm
        elif ((f_header >> 5) & 0x03) == 2:
            _arm = CodecZenohExtZbuf.decode(cursor)
            body = _arm
        else:
            _arm = CodecZenohExtUnit.decode(cursor)
            body = CodecZenohOamDefault(tag=((f_header >> 5) & 0x03), body=_arm)
        return cls(
            header=f_header,
            id=f_id,
            extensions=f_extensions,
            body=body,
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

    def enc(self) -> int:
        return (self.header >> 5) & 0x03

    def set_enc(self, v: int) -> None:
        _shifted_mask = 0x03 << 5
        _val = (v & 0x03) << 5
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
