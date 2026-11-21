# SCE-MAP: codec_zenoh_request:73 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, SceCursor, SceSink, TooManyElements
from sce_forge_runtime.bounded import BoundedInt, BoundedTuple, UnknownTag
from .codec_zenoh_wireexpr import CodecZenohWireexpr
from .codec_zenoh_ext_entry import CodecZenohExtEntry
from .codec_zenoh_msg_put import CodecZenohMsgPut
from .codec_zenoh_msg_del import CodecZenohMsgDel
from .codec_zenoh_query import CodecZenohQuery
from . import codec_zenoh_ext_entry as sce_codec_zenoh_ext_entry
from . import codec_zenoh_msg_del as sce_codec_zenoh_msg_del
from . import codec_zenoh_msg_put as sce_codec_zenoh_msg_put
from . import codec_zenoh_query as sce_codec_zenoh_query
from . import codec_zenoh_wireexpr as sce_codec_zenoh_wireexpr

from dataclasses import dataclass, field
from typing import Optional, List, Protocol, Union


class CodecZenohRequestHeaderInt(BoundedInt):
    """The ``header`` element of :class:`CodecZenohRequest`, an unsigned integer of
    8 bits, held to them where a value of it is made (docs/adr/0081 §2)."""

    BITS = 8
    SIGNED = False


class CodecZenohRequestRidInt(BoundedInt):
    """The ``rid`` element of :class:`CodecZenohRequest`, an unsigned integer of
    64 bits, held to them where a value of it is made (docs/adr/0081 §2)."""

    BITS = 64
    SIGNED = False


class CodecZenohRequestExtensionsList(BoundedTuple):
    """The ``extensions`` element of :class:`CodecZenohRequest`, 1 or more entries, the protocol stating no most,
    held to that bound where a value of it is made (docs/adr/0077 §3)."""

    LEAST = 1
    MOST = None


class CodecZenohRequestUnknownTag(UnknownTag):
    """The tag the default arm of :class:`CodecZenohRequest`'s variant keeps: at
    most 31, and none an arm names (docs/adr/0065 §6 R3)."""

    MOST = 31
    KNOWN = frozenset({1, 2, 3})


@dataclass
class CodecZenohRequestDefault:
    """The default arm of :class:`CodecZenohRequest`'s variant: the tag no arm
    names, which it keeps, and its body."""

    tag: CodecZenohRequestUnknownTag = CodecZenohRequestUnknownTag(0)
    body: sce_codec_zenoh_query.CodecZenohQuery = field(default_factory=lambda: sce_codec_zenoh_query.CodecZenohQuery())

    def __setattr__(self, name: str, value: object) -> None:
        # The tag is held to the tags no arm names wherever it is set,
        # where the arm is made and after (docs/adr/0065 §6 R3).
        if name == "tag":
            value = CodecZenohRequestUnknownTag(value)
        object.__setattr__(self, name, value)


CodecZenohRequestVariant = Union[sce_codec_zenoh_msg_put.CodecZenohMsgPut, sce_codec_zenoh_msg_del.CodecZenohMsgDel, sce_codec_zenoh_query.CodecZenohQuery, CodecZenohRequestDefault]
"""The arm a value of :class:`CodecZenohRequest` holds: a value of the arm's codec,
or the default arm."""


ARM_CLASSES = {0x1: sce_codec_zenoh_msg_put.CodecZenohMsgPut, 0x2: sce_codec_zenoh_msg_del.CodecZenohMsgDel, 0x3: sce_codec_zenoh_query.CodecZenohQuery}
"""The class of the arm of :class:`CodecZenohRequest`'s variant each tag names."""


class CodecZenohRequestSource(Protocol):
    """The elements a value of :class:`CodecZenohRequest` presents, which its encode
    reads (docs/adr/0081 §2): any object that presents them is a source of it.
    The encode derives every other element of the frame."""

    @property
    def header(self) -> CodecZenohRequestHeaderInt: ...

    @property
    def rid(self) -> CodecZenohRequestRidInt: ...

    @property
    def keyexpr(self) -> sce_codec_zenoh_wireexpr.CodecZenohWireexprSource: ...

    @property
    def extensions(self) -> Optional[CodecZenohRequestExtensionsList]: ...

    @property
    def body(self) -> CodecZenohRequestVariant: ...


def encode_source(src: CodecZenohRequestSource, w: SceSink, *, override_header_m: Optional[int] = None) -> None:
    """Write any source of :class:`CodecZenohRequest` into ``w``, each element read
    off ``src`` and written where the codec's encode plan lays it out
    (docs/adr/0081). A chain that holds it
    sets the flag its entries end on by its keyword, in place of the value's. It refuses no value; a bounded sink raises
    :class:`BufferOverflow` where it has no room left."""
    _v_body = src.body
    if not isinstance(_v_body, (sce_codec_zenoh_msg_put.CodecZenohMsgPut, sce_codec_zenoh_msg_del.CodecZenohMsgDel, sce_codec_zenoh_query.CodecZenohQuery, CodecZenohRequestDefault,)):
        raise TypeError(f"{type(_v_body).__name__} is no arm of CodecZenohRequest's variant")
    _c_header = src.header
    _c_header = (_c_header & ~0x20) | ((sce_codec_zenoh_wireexpr.input_n(src.keyexpr)) << 5)
    _c_header = (_c_header & ~0x80) | (0x80 if src.extensions is not None else 0)
    if override_header_m is not None:
        _c_header = (_c_header & ~0x40) | ((override_header_m & 0x1) << 6)
    w.write_u8(_c_header & 0xFF)
    w.write_vle_u64(src.rid)
    sce_codec_zenoh_wireexpr.encode_source(src.keyexpr, w)
    if src.extensions is not None:
        _left_extensions = len(src.extensions)
        for _entry in src.extensions:
            _left_extensions -= 1
            sce_codec_zenoh_ext_entry.encode_source(_entry, w, override_header_z=1 if _left_extensions != 0 else 0)
    if isinstance(_v_body, sce_codec_zenoh_msg_put.CodecZenohMsgPut):
        sce_codec_zenoh_msg_put.encode_source(_v_body, w)
    elif isinstance(_v_body, sce_codec_zenoh_msg_del.CodecZenohMsgDel):
        sce_codec_zenoh_msg_del.encode_source(_v_body, w)
    elif isinstance(_v_body, sce_codec_zenoh_query.CodecZenohQuery):
        sce_codec_zenoh_query.encode_source(_v_body, w)
    else:
        sce_codec_zenoh_query.encode_source(_v_body.body, w)


@dataclass
class CodecZenohRequest:
    header: CodecZenohRequestHeaderInt = 0x1c
    rid: CodecZenohRequestRidInt = 0
    keyexpr: CodecZenohWireexpr = field(default_factory=CodecZenohWireexpr)
    extensions: Optional[CodecZenohRequestExtensionsList] = None
    body: CodecZenohRequestVariant = field(default_factory=lambda: sce_codec_zenoh_msg_put.CodecZenohMsgPut())

    def __setattr__(self, name: str, value: object) -> None:
        # Each integer, byte string and text is held to its bound
        # wherever it is set, where the value is made and after
        # (docs/adr/0081 §2), whatever it was handed.
        if name == "header":
            value = CodecZenohRequestHeaderInt(value)
        elif name == "rid":
            value = CodecZenohRequestRidInt(value)
        elif name == "extensions":
            value = None if value is None else CodecZenohRequestExtensionsList(value)
        object.__setattr__(self, name, value)

    @classmethod
    def decode(cls, cursor: SceCursor) -> CodecZenohRequest:
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
        f_rid = cursor.read_vle_u64()
        f_keyexpr = CodecZenohWireexpr.decode(cursor, ((f_header >> 5) & 0x1))
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
        _peek = cursor.peek_slice(1)[0]
        # Dispatch on the tag field; each arm decodes its body codec
        # from the cursor. The default arm (when declared) carries the
        # runtime tag value so encode can round-trip it back onto the
        # wire.
        if ((_peek >> 0) & 0x1F) == 1:
            _arm = CodecZenohMsgPut.decode(cursor)
            body = _arm
        elif ((_peek >> 0) & 0x1F) == 2:
            _arm = CodecZenohMsgDel.decode(cursor)
            body = _arm
        elif ((_peek >> 0) & 0x1F) == 3:
            _arm = CodecZenohQuery.decode(cursor)
            body = _arm
        else:
            _arm = CodecZenohQuery.decode(cursor)
            body = CodecZenohRequestDefault(tag=((_peek >> 0) & 0x1F), body=_arm)
        return cls(
            header=f_header,
            rid=f_rid,
            keyexpr=f_keyexpr,
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

    def n(self) -> bool:
        return (self.header & 0x20) != 0

    def set_n(self, v: bool) -> None:
        if v:
            self.header = (self.header | 0x20) & 0xFF
        else:
            self.header = self.header & (0xFF ^ 0x20)

    def m(self) -> bool:
        return (self.header & 0x40) != 0

    def set_m(self, v: bool) -> None:
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
