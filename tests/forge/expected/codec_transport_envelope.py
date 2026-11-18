# SCE-MAP: codec_transport_envelope:68 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, SceCursor, SceSink
from sce_forge_runtime.bounded import BoundedInt, UnknownTag
from .codec_zenoh_init_body import CodecZenohInitBody
from .codec_zenoh_open_body import CodecZenohOpenBody
from .codec_zenoh_close import CodecZenohClose
from .codec_zenoh_keep_alive import CodecZenohKeepAlive
from .codec_zenoh_frame import CodecZenohFrame
from .codec_zenoh_fragment import CodecZenohFragment
from .codec_zenoh_join import CodecZenohJoin
from . import codec_zenoh_close as sce_codec_zenoh_close
from . import codec_zenoh_fragment as sce_codec_zenoh_fragment
from . import codec_zenoh_frame as sce_codec_zenoh_frame
from . import codec_zenoh_init_body as sce_codec_zenoh_init_body
from . import codec_zenoh_join as sce_codec_zenoh_join
from . import codec_zenoh_keep_alive as sce_codec_zenoh_keep_alive
from . import codec_zenoh_open_body as sce_codec_zenoh_open_body

from dataclasses import dataclass, field
from typing import Optional, Protocol, Union


class CodecTransportEnvelopeHeaderInt(BoundedInt):
    """The ``header`` element of :class:`CodecTransportEnvelope`, an unsigned integer of
    8 bits, held to them where a value of it is made (docs/adr/0081 §2)."""

    BITS = 8
    SIGNED = False


class CodecTransportEnvelopeUnknownTag(UnknownTag):
    """The tag the default arm of :class:`CodecTransportEnvelope`'s variant keeps: at
    most 31, and none an arm names (docs/adr/0065 §6 R3)."""

    MOST = 31
    KNOWN = frozenset({1, 2, 3, 4, 5, 6, 7})


@dataclass
class CodecTransportEnvelopeDefault:
    """The default arm of :class:`CodecTransportEnvelope`'s variant: the tag no arm
    names, which it keeps, and its body."""

    tag: CodecTransportEnvelopeUnknownTag = CodecTransportEnvelopeUnknownTag(0)
    body: sce_codec_zenoh_close.CodecZenohClose = field(default_factory=sce_codec_zenoh_close.CodecZenohClose)

    def __setattr__(self, name: str, value: object) -> None:
        # The tag is held to the tags no arm names wherever it is set,
        # where the arm is made and after (docs/adr/0065 §6 R3).
        if name == "tag":
            value = CodecTransportEnvelopeUnknownTag(value)
        object.__setattr__(self, name, value)


CodecTransportEnvelopeVariant = Union[sce_codec_zenoh_init_body.CodecZenohInitBody, sce_codec_zenoh_open_body.CodecZenohOpenBody, sce_codec_zenoh_close.CodecZenohClose, sce_codec_zenoh_keep_alive.CodecZenohKeepAlive, sce_codec_zenoh_frame.CodecZenohFrame, sce_codec_zenoh_fragment.CodecZenohFragment, sce_codec_zenoh_join.CodecZenohJoin, CodecTransportEnvelopeDefault]
"""The arm a value of :class:`CodecTransportEnvelope` holds: a value of the arm's codec,
or the default arm."""


ARM_CLASSES = {0x1: sce_codec_zenoh_init_body.CodecZenohInitBody, 0x2: sce_codec_zenoh_open_body.CodecZenohOpenBody, 0x3: sce_codec_zenoh_close.CodecZenohClose, 0x4: sce_codec_zenoh_keep_alive.CodecZenohKeepAlive, 0x5: sce_codec_zenoh_frame.CodecZenohFrame, 0x6: sce_codec_zenoh_fragment.CodecZenohFragment, 0x7: sce_codec_zenoh_join.CodecZenohJoin}
"""The class of the arm of :class:`CodecTransportEnvelope`'s variant each tag names."""


class CodecTransportEnvelopeSource(Protocol):
    """The elements a value of :class:`CodecTransportEnvelope` presents, which its encode
    reads (docs/adr/0081 §2): any object that presents them is a source of it.
    The encode derives every other element of the frame."""

    @property
    def header(self) -> CodecTransportEnvelopeHeaderInt: ...

    @property
    def body(self) -> CodecTransportEnvelopeVariant: ...


def encode_source(src: CodecTransportEnvelopeSource, w: SceSink, *, override_header_z: Optional[int] = None) -> None:
    """Write any source of :class:`CodecTransportEnvelope` into ``w``, each element read
    off ``src`` and written where the codec's encode plan lays it out
    (docs/adr/0081). A chain that holds it
    sets the flag its entries end on by its keyword, in place of the value's. It refuses no value; a bounded sink raises
    :class:`BufferOverflow` where it has no room left."""
    _v_body = src.body
    if not isinstance(_v_body, (sce_codec_zenoh_init_body.CodecZenohInitBody, sce_codec_zenoh_open_body.CodecZenohOpenBody, sce_codec_zenoh_close.CodecZenohClose, sce_codec_zenoh_keep_alive.CodecZenohKeepAlive, sce_codec_zenoh_frame.CodecZenohFrame, sce_codec_zenoh_fragment.CodecZenohFragment, sce_codec_zenoh_join.CodecZenohJoin, CodecTransportEnvelopeDefault,)):
        raise TypeError(f"{type(_v_body).__name__} is no arm of CodecTransportEnvelope's variant")
    _t_body = (1 if isinstance(_v_body, sce_codec_zenoh_init_body.CodecZenohInitBody) else 2 if isinstance(_v_body, sce_codec_zenoh_open_body.CodecZenohOpenBody) else 3 if isinstance(_v_body, sce_codec_zenoh_close.CodecZenohClose) else 4 if isinstance(_v_body, sce_codec_zenoh_keep_alive.CodecZenohKeepAlive) else 5 if isinstance(_v_body, sce_codec_zenoh_frame.CodecZenohFrame) else 6 if isinstance(_v_body, sce_codec_zenoh_fragment.CodecZenohFragment) else 7 if isinstance(_v_body, sce_codec_zenoh_join.CodecZenohJoin) else _v_body.tag)
    _c_header = src.header
    _c_header = (_c_header & ~0x1F) | ((_t_body & 0x1F) << 0)
    if isinstance(_v_body, (sce_codec_zenoh_init_body.CodecZenohInitBody, sce_codec_zenoh_open_body.CodecZenohOpenBody,)):
        _c_header = (_c_header & ~0x20) | (((sce_codec_zenoh_init_body.input_a(_v_body) if isinstance(_v_body, sce_codec_zenoh_init_body.CodecZenohInitBody) else sce_codec_zenoh_open_body.input_a(_v_body) if isinstance(_v_body, sce_codec_zenoh_open_body.CodecZenohOpenBody) else 0)) << 5)
    if isinstance(_v_body, (sce_codec_zenoh_init_body.CodecZenohInitBody, sce_codec_zenoh_join.CodecZenohJoin,)):
        _c_header = (_c_header & ~0x40) | (((sce_codec_zenoh_init_body.input_s(_v_body) if isinstance(_v_body, sce_codec_zenoh_init_body.CodecZenohInitBody) else sce_codec_zenoh_join.input_s(_v_body) if isinstance(_v_body, sce_codec_zenoh_join.CodecZenohJoin) else 0)) << 6)
    if override_header_z is not None:
        _c_header = (_c_header & ~0x80) | ((override_header_z & 0x1) << 7)
    w.write_u8(_c_header & 0xFF)
    if isinstance(_v_body, sce_codec_zenoh_init_body.CodecZenohInitBody):
        sce_codec_zenoh_init_body.encode_source(_v_body, w)
    elif isinstance(_v_body, sce_codec_zenoh_open_body.CodecZenohOpenBody):
        sce_codec_zenoh_open_body.encode_source(_v_body, w)
    elif isinstance(_v_body, sce_codec_zenoh_close.CodecZenohClose):
        sce_codec_zenoh_close.encode_source(_v_body, w)
    elif isinstance(_v_body, sce_codec_zenoh_keep_alive.CodecZenohKeepAlive):
        sce_codec_zenoh_keep_alive.encode_source(_v_body, w)
    elif isinstance(_v_body, sce_codec_zenoh_frame.CodecZenohFrame):
        sce_codec_zenoh_frame.encode_source(_v_body, w)
    elif isinstance(_v_body, sce_codec_zenoh_fragment.CodecZenohFragment):
        sce_codec_zenoh_fragment.encode_source(_v_body, w)
    elif isinstance(_v_body, sce_codec_zenoh_join.CodecZenohJoin):
        sce_codec_zenoh_join.encode_source(_v_body, w)
    else:
        sce_codec_zenoh_close.encode_source(_v_body.body, w)


@dataclass
class CodecTransportEnvelope:
    header: CodecTransportEnvelopeHeaderInt = 0
    body: CodecTransportEnvelopeVariant = field(default_factory=sce_codec_zenoh_close.CodecZenohClose)

    def __setattr__(self, name: str, value: object) -> None:
        # Each integer, byte string and text is held to its bound
        # wherever it is set, where the value is made and after
        # (docs/adr/0081 §2), whatever it was handed.
        if name == "header":
            value = CodecTransportEnvelopeHeaderInt(value)
        object.__setattr__(self, name, value)

    @classmethod
    def decode(cls, cursor: SceCursor) -> CodecTransportEnvelope:
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
        if ((f_header >> 0) & 0x1F) == 1:
            _arm = CodecZenohInitBody.decode(cursor, ((f_header >> 6) & 0x1), ((f_header >> 5) & 0x1))
            body = _arm
        elif ((f_header >> 0) & 0x1F) == 2:
            _arm = CodecZenohOpenBody.decode(cursor, ((f_header >> 5) & 0x1))
            body = _arm
        elif ((f_header >> 0) & 0x1F) == 3:
            _arm = CodecZenohClose.decode(cursor)
            body = _arm
        elif ((f_header >> 0) & 0x1F) == 4:
            _arm = CodecZenohKeepAlive.decode(cursor)
            body = _arm
        elif ((f_header >> 0) & 0x1F) == 5:
            _arm = CodecZenohFrame.decode(cursor)
            body = _arm
        elif ((f_header >> 0) & 0x1F) == 6:
            _arm = CodecZenohFragment.decode(cursor)
            body = _arm
        elif ((f_header >> 0) & 0x1F) == 7:
            _arm = CodecZenohJoin.decode(cursor, ((f_header >> 6) & 0x1))
            body = _arm
        else:
            _arm = CodecZenohClose.decode(cursor)
            body = CodecTransportEnvelopeDefault(tag=((f_header >> 0) & 0x1F), body=_arm)
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
    def mid(self) -> int:
        return (self.header >> 0) & 0x1F

    def set_mid(self, v: int) -> None:
        _shifted_mask = 0x1F << 0
        _val = (v & 0x1F) << 0
        self.header = ((self.header & (0xFF ^ _shifted_mask)) | _val) & 0xFF

    def a(self) -> bool:
        return (self.header & 0x20) != 0

    def set_a(self, v: bool) -> None:
        if v:
            self.header = (self.header | 0x20) & 0xFF
        else:
            self.header = self.header & (0xFF ^ 0x20)

    def s(self) -> bool:
        return (self.header & 0x40) != 0

    def set_s(self, v: bool) -> None:
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
