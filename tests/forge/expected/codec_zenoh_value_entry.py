# SCE-MAP: codec_zenoh_value_entry:14 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, SceCursor, SceSink
from sce_forge_runtime.bounded import BoundedInt, UnknownTag
from .codec_zenoh_ext_unit import CodecZenohExtUnit
from .codec_zenoh_ext_zint import CodecZenohExtZint
from .codec_zenoh_value_zbuf import CodecZenohValueZbuf
from . import codec_zenoh_ext_unit as sce_codec_zenoh_ext_unit
from . import codec_zenoh_ext_zint as sce_codec_zenoh_ext_zint
from . import codec_zenoh_value_zbuf as sce_codec_zenoh_value_zbuf

from dataclasses import dataclass, field
from typing import Optional, Protocol, Union


class CodecZenohValueEntryHeaderInt(BoundedInt):
    """The ``header`` element of :class:`CodecZenohValueEntry`, an unsigned integer of
    8 bits, held to them where a value of it is made (docs/adr/0081 §2)."""

    BITS = 8
    SIGNED = False


class CodecZenohValueEntryUnknownTag(UnknownTag):
    """The tag the default arm of :class:`CodecZenohValueEntry`'s variant keeps: at
    most 3, and none an arm names (docs/adr/0065 §6 R3)."""

    MOST = 3
    KNOWN = frozenset({0, 1, 2})


@dataclass
class CodecZenohValueEntryDefault:
    """The default arm of :class:`CodecZenohValueEntry`'s variant: the tag no arm
    names, which it keeps, and its body."""

    tag: CodecZenohValueEntryUnknownTag = CodecZenohValueEntryUnknownTag(3)
    body: sce_codec_zenoh_ext_unit.CodecZenohExtUnit = field(default_factory=sce_codec_zenoh_ext_unit.CodecZenohExtUnit)

    def __setattr__(self, name: str, value: object) -> None:
        # The tag is held to the tags no arm names wherever it is set,
        # where the arm is made and after (docs/adr/0065 §6 R3).
        if name == "tag":
            value = CodecZenohValueEntryUnknownTag(value)
        object.__setattr__(self, name, value)


CodecZenohValueEntryVariant = Union[sce_codec_zenoh_ext_unit.CodecZenohExtUnit, sce_codec_zenoh_ext_zint.CodecZenohExtZint, sce_codec_zenoh_value_zbuf.CodecZenohValueZbuf, CodecZenohValueEntryDefault]
"""The arm a value of :class:`CodecZenohValueEntry` holds: a value of the arm's codec,
or the default arm."""


ARM_CLASSES = {0x0: sce_codec_zenoh_ext_unit.CodecZenohExtUnit, 0x1: sce_codec_zenoh_ext_zint.CodecZenohExtZint, 0x2: sce_codec_zenoh_value_zbuf.CodecZenohValueZbuf}
"""The class of the arm of :class:`CodecZenohValueEntry`'s variant each tag names."""


class CodecZenohValueEntrySource(Protocol):
    """The elements a value of :class:`CodecZenohValueEntry` presents, which its encode
    reads (docs/adr/0081 §2): any object that presents them is a source of it.
    The encode derives every other element of the frame."""

    @property
    def header(self) -> CodecZenohValueEntryHeaderInt: ...

    @property
    def body(self) -> CodecZenohValueEntryVariant: ...


def encode_source(src: CodecZenohValueEntrySource, w: SceSink, *, override_header_m: Optional[int] = None, override_header_z: Optional[int] = None) -> None:
    """Write any source of :class:`CodecZenohValueEntry` into ``w``, each element read
    off ``src`` and written where the codec's encode plan lays it out
    (docs/adr/0081). A chain that holds it
    sets the flag its entries end on by its keyword, in place of the value's. It refuses no value; a bounded sink raises
    :class:`BufferOverflow` where it has no room left."""
    _v_body = src.body
    if not isinstance(_v_body, (sce_codec_zenoh_ext_unit.CodecZenohExtUnit, sce_codec_zenoh_ext_zint.CodecZenohExtZint, sce_codec_zenoh_value_zbuf.CodecZenohValueZbuf, CodecZenohValueEntryDefault,)):
        raise TypeError(f"{type(_v_body).__name__} is no arm of CodecZenohValueEntry's variant")
    _t_body = (0 if isinstance(_v_body, sce_codec_zenoh_ext_unit.CodecZenohExtUnit) else 1 if isinstance(_v_body, sce_codec_zenoh_ext_zint.CodecZenohExtZint) else 2 if isinstance(_v_body, sce_codec_zenoh_value_zbuf.CodecZenohValueZbuf) else _v_body.tag)
    _c_header = src.header
    _c_header = (_c_header & ~0x60) | ((_t_body & 0x3) << 5)
    if override_header_m is not None:
        _c_header = (_c_header & ~0x10) | ((override_header_m & 0x1) << 4)
    if override_header_z is not None:
        _c_header = (_c_header & ~0x80) | ((override_header_z & 0x1) << 7)
    w.write_u8(_c_header & 0xFF)
    if isinstance(_v_body, sce_codec_zenoh_ext_unit.CodecZenohExtUnit):
        sce_codec_zenoh_ext_unit.encode_source(_v_body, w)
    elif isinstance(_v_body, sce_codec_zenoh_ext_zint.CodecZenohExtZint):
        sce_codec_zenoh_ext_zint.encode_source(_v_body, w)
    elif isinstance(_v_body, sce_codec_zenoh_value_zbuf.CodecZenohValueZbuf):
        sce_codec_zenoh_value_zbuf.encode_source(_v_body, w)
    else:
        sce_codec_zenoh_ext_unit.encode_source(_v_body.body, w)


def input_after_shm(src: CodecZenohValueEntrySource) -> Optional[int]:
    """The flag input ``after_shm`` a source of :class:`CodecZenohValueEntry` says, which
    its encode is not handed (docs/adr/0065 §6 R4a),
    or ``None`` where the value says nothing of it."""
    return (sce_codec_zenoh_value_zbuf.input_after_shm(src.body) if isinstance(src.body, sce_codec_zenoh_value_zbuf.CodecZenohValueZbuf) else None)


@dataclass
class CodecZenohValueEntry:
    header: CodecZenohValueEntryHeaderInt = 0
    body: CodecZenohValueEntryVariant = field(default_factory=sce_codec_zenoh_ext_unit.CodecZenohExtUnit)

    def __setattr__(self, name: str, value: object) -> None:
        # Each integer, byte string and text is held to its bound
        # wherever it is set, where the value is made and after
        # (docs/adr/0081 §2), whatever it was handed.
        if name == "header":
            value = CodecZenohValueEntryHeaderInt(value)
        object.__setattr__(self, name, value)

    @classmethod
    def decode(cls, cursor: SceCursor, after_shm: int) -> CodecZenohValueEntry:
        """Decode the next frame from ``cursor``; on success the cursor
        advances past the consumed bytes. A frame the codec refuses raises
        the ``CodecError`` subclass that names the refusal (docs/adr/0099,
        amendment of 2026-11-16): ``NeedMoreBytes`` when the cursor's tail
        is shorter than the frame (RFC §synth-5-B L494-519), which the
        caller resumes after appending bytes, and for a frame no more bytes
        make whole ``VleWidthOverflow``, ``InvalidUtf8``,
        ``UndeclaredEnumValue``, or a length or a count past its bound
        (``LengthOutOfRange``, ``TooManyElements``, docs/adr/0077)."""
        # Decode fixed prefix (RFC §synth-5-B variant: fields before tag suffix).
        raw = cursor.peek_slice(1)
        f_header = raw[0]
        cursor.advance(1)
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
            _arm = CodecZenohValueZbuf.decode(cursor, after_shm)
            body = _arm
        else:
            _arm = CodecZenohExtUnit.decode(cursor)
            body = CodecZenohValueEntryDefault(tag=((f_header >> 5) & 0x03), body=_arm)
        return cls(
            header=f_header,
            body=body,
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

    def m(self) -> bool:
        return (self.header & 0x10) != 0

    def set_m(self, v: bool) -> None:
        if v:
            self.header = (self.header | 0x10) & 0xFF
        else:
            self.header = self.header & (0xFF ^ 0x10)

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
