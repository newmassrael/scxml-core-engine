# SCE-MAP: codec_zenoh_declaration:54 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, SceCursor, SceSink
from sce_forge_runtime.bounded import BoundedInt, UnknownTag
from .codec_zenoh_decl_kexpr import CodecZenohDeclKexpr
from .codec_zenoh_undecl_kexpr import CodecZenohUndeclKexpr
from .codec_zenoh_decl_subscriber import CodecZenohDeclSubscriber
from .codec_zenoh_undecl_subscriber import CodecZenohUndeclSubscriber
from .codec_zenoh_decl_queryable import CodecZenohDeclQueryable
from .codec_zenoh_undecl_queryable import CodecZenohUndeclQueryable
from .codec_zenoh_decl_token import CodecZenohDeclToken
from .codec_zenoh_undecl_token import CodecZenohUndeclToken
from .codec_zenoh_decl_final import CodecZenohDeclFinal
from . import codec_zenoh_decl_final as sce_codec_zenoh_decl_final
from . import codec_zenoh_decl_kexpr as sce_codec_zenoh_decl_kexpr
from . import codec_zenoh_decl_queryable as sce_codec_zenoh_decl_queryable
from . import codec_zenoh_decl_subscriber as sce_codec_zenoh_decl_subscriber
from . import codec_zenoh_decl_token as sce_codec_zenoh_decl_token
from . import codec_zenoh_undecl_kexpr as sce_codec_zenoh_undecl_kexpr
from . import codec_zenoh_undecl_queryable as sce_codec_zenoh_undecl_queryable
from . import codec_zenoh_undecl_subscriber as sce_codec_zenoh_undecl_subscriber
from . import codec_zenoh_undecl_token as sce_codec_zenoh_undecl_token

from dataclasses import dataclass, field
from typing import Optional, Protocol, Union


class CodecZenohDeclarationHeaderInt(BoundedInt):
    """The ``header`` element of :class:`CodecZenohDeclaration`, an unsigned integer of
    8 bits, held to them where a value of it is made (docs/adr/0081 §2)."""

    BITS = 8
    SIGNED = False


class CodecZenohDeclarationUnknownTag(UnknownTag):
    """The tag the default arm of :class:`CodecZenohDeclaration`'s variant keeps: at
    most 31, and none an arm names (docs/adr/0065 §6 R3)."""

    MOST = 31
    KNOWN = frozenset({0, 1, 2, 3, 4, 5, 6, 7, 26})


@dataclass
class CodecZenohDeclarationDefault:
    """The default arm of :class:`CodecZenohDeclaration`'s variant: the tag no arm
    names, which it keeps, and its body."""

    tag: CodecZenohDeclarationUnknownTag = CodecZenohDeclarationUnknownTag(8)
    body: sce_codec_zenoh_decl_final.CodecZenohDeclFinal = field(default_factory=sce_codec_zenoh_decl_final.CodecZenohDeclFinal)

    def __setattr__(self, name: str, value: object) -> None:
        # The tag is held to the tags no arm names wherever it is set,
        # where the arm is made and after (docs/adr/0065 §6 R3).
        if name == "tag":
            value = CodecZenohDeclarationUnknownTag(value)
        object.__setattr__(self, name, value)


CodecZenohDeclarationVariant = Union[sce_codec_zenoh_decl_kexpr.CodecZenohDeclKexpr, sce_codec_zenoh_undecl_kexpr.CodecZenohUndeclKexpr, sce_codec_zenoh_decl_subscriber.CodecZenohDeclSubscriber, sce_codec_zenoh_undecl_subscriber.CodecZenohUndeclSubscriber, sce_codec_zenoh_decl_queryable.CodecZenohDeclQueryable, sce_codec_zenoh_undecl_queryable.CodecZenohUndeclQueryable, sce_codec_zenoh_decl_token.CodecZenohDeclToken, sce_codec_zenoh_undecl_token.CodecZenohUndeclToken, sce_codec_zenoh_decl_final.CodecZenohDeclFinal, CodecZenohDeclarationDefault]
"""The arm a value of :class:`CodecZenohDeclaration` holds: a value of the arm's codec,
or the default arm."""


ARM_CLASSES = {0x0: sce_codec_zenoh_decl_kexpr.CodecZenohDeclKexpr, 0x1: sce_codec_zenoh_undecl_kexpr.CodecZenohUndeclKexpr, 0x2: sce_codec_zenoh_decl_subscriber.CodecZenohDeclSubscriber, 0x3: sce_codec_zenoh_undecl_subscriber.CodecZenohUndeclSubscriber, 0x4: sce_codec_zenoh_decl_queryable.CodecZenohDeclQueryable, 0x5: sce_codec_zenoh_undecl_queryable.CodecZenohUndeclQueryable, 0x6: sce_codec_zenoh_decl_token.CodecZenohDeclToken, 0x7: sce_codec_zenoh_undecl_token.CodecZenohUndeclToken, 0x1A: sce_codec_zenoh_decl_final.CodecZenohDeclFinal}
"""The class of the arm of :class:`CodecZenohDeclaration`'s variant each tag names."""


class CodecZenohDeclarationSource(Protocol):
    """The elements a value of :class:`CodecZenohDeclaration` presents, which its encode
    reads (docs/adr/0081 §2): any object that presents them is a source of it.
    The encode derives every other element of the frame."""

    @property
    def header(self) -> CodecZenohDeclarationHeaderInt: ...

    @property
    def body(self) -> CodecZenohDeclarationVariant: ...


def encode_source(src: CodecZenohDeclarationSource, w: SceSink, *, override_header_m: Optional[int] = None) -> None:
    """Write any source of :class:`CodecZenohDeclaration` into ``w``, each element read
    off ``src`` and written where the codec's encode plan lays it out
    (docs/adr/0081). A chain that holds it
    sets the flag its entries end on by its keyword, in place of the value's. It refuses no value; a bounded sink raises
    :class:`BufferOverflow` where it has no room left."""
    _v_body = src.body
    if not isinstance(_v_body, (sce_codec_zenoh_decl_kexpr.CodecZenohDeclKexpr, sce_codec_zenoh_undecl_kexpr.CodecZenohUndeclKexpr, sce_codec_zenoh_decl_subscriber.CodecZenohDeclSubscriber, sce_codec_zenoh_undecl_subscriber.CodecZenohUndeclSubscriber, sce_codec_zenoh_decl_queryable.CodecZenohDeclQueryable, sce_codec_zenoh_undecl_queryable.CodecZenohUndeclQueryable, sce_codec_zenoh_decl_token.CodecZenohDeclToken, sce_codec_zenoh_undecl_token.CodecZenohUndeclToken, sce_codec_zenoh_decl_final.CodecZenohDeclFinal, CodecZenohDeclarationDefault,)):
        raise TypeError(f"{type(_v_body).__name__} is no arm of CodecZenohDeclaration's variant")
    _t_body = (0 if isinstance(_v_body, sce_codec_zenoh_decl_kexpr.CodecZenohDeclKexpr) else 1 if isinstance(_v_body, sce_codec_zenoh_undecl_kexpr.CodecZenohUndeclKexpr) else 2 if isinstance(_v_body, sce_codec_zenoh_decl_subscriber.CodecZenohDeclSubscriber) else 3 if isinstance(_v_body, sce_codec_zenoh_undecl_subscriber.CodecZenohUndeclSubscriber) else 4 if isinstance(_v_body, sce_codec_zenoh_decl_queryable.CodecZenohDeclQueryable) else 5 if isinstance(_v_body, sce_codec_zenoh_undecl_queryable.CodecZenohUndeclQueryable) else 6 if isinstance(_v_body, sce_codec_zenoh_decl_token.CodecZenohDeclToken) else 7 if isinstance(_v_body, sce_codec_zenoh_undecl_token.CodecZenohUndeclToken) else 26 if isinstance(_v_body, sce_codec_zenoh_decl_final.CodecZenohDeclFinal) else _v_body.tag)
    _c_header = src.header
    _c_header = (_c_header & ~0x1F) | ((_t_body & 0x1F) << 0)
    if isinstance(_v_body, (sce_codec_zenoh_decl_kexpr.CodecZenohDeclKexpr, sce_codec_zenoh_decl_subscriber.CodecZenohDeclSubscriber, sce_codec_zenoh_decl_queryable.CodecZenohDeclQueryable, sce_codec_zenoh_decl_token.CodecZenohDeclToken,)):
        _c_header = (_c_header & ~0x20) | (((sce_codec_zenoh_decl_kexpr.input_n(_v_body) if isinstance(_v_body, sce_codec_zenoh_decl_kexpr.CodecZenohDeclKexpr) else sce_codec_zenoh_decl_subscriber.input_n(_v_body) if isinstance(_v_body, sce_codec_zenoh_decl_subscriber.CodecZenohDeclSubscriber) else sce_codec_zenoh_decl_queryable.input_n(_v_body) if isinstance(_v_body, sce_codec_zenoh_decl_queryable.CodecZenohDeclQueryable) else sce_codec_zenoh_decl_token.input_n(_v_body) if isinstance(_v_body, sce_codec_zenoh_decl_token.CodecZenohDeclToken) else 0)) << 5)
    if isinstance(_v_body, (sce_codec_zenoh_undecl_subscriber.CodecZenohUndeclSubscriber, sce_codec_zenoh_decl_queryable.CodecZenohDeclQueryable, sce_codec_zenoh_undecl_queryable.CodecZenohUndeclQueryable, sce_codec_zenoh_undecl_token.CodecZenohUndeclToken,)):
        _c_header = (_c_header & ~0x80) | (((sce_codec_zenoh_undecl_subscriber.input_z(_v_body) if isinstance(_v_body, sce_codec_zenoh_undecl_subscriber.CodecZenohUndeclSubscriber) else sce_codec_zenoh_decl_queryable.input_z(_v_body) if isinstance(_v_body, sce_codec_zenoh_decl_queryable.CodecZenohDeclQueryable) else sce_codec_zenoh_undecl_queryable.input_z(_v_body) if isinstance(_v_body, sce_codec_zenoh_undecl_queryable.CodecZenohUndeclQueryable) else sce_codec_zenoh_undecl_token.input_z(_v_body) if isinstance(_v_body, sce_codec_zenoh_undecl_token.CodecZenohUndeclToken) else 0)) << 7)
    if override_header_m is not None:
        _c_header = (_c_header & ~0x40) | ((override_header_m & 0x1) << 6)
    w.write_u8(_c_header & 0xFF)
    if isinstance(_v_body, sce_codec_zenoh_decl_kexpr.CodecZenohDeclKexpr):
        sce_codec_zenoh_decl_kexpr.encode_source(_v_body, w)
    elif isinstance(_v_body, sce_codec_zenoh_undecl_kexpr.CodecZenohUndeclKexpr):
        sce_codec_zenoh_undecl_kexpr.encode_source(_v_body, w)
    elif isinstance(_v_body, sce_codec_zenoh_decl_subscriber.CodecZenohDeclSubscriber):
        sce_codec_zenoh_decl_subscriber.encode_source(_v_body, w)
    elif isinstance(_v_body, sce_codec_zenoh_undecl_subscriber.CodecZenohUndeclSubscriber):
        sce_codec_zenoh_undecl_subscriber.encode_source(_v_body, w)
    elif isinstance(_v_body, sce_codec_zenoh_decl_queryable.CodecZenohDeclQueryable):
        sce_codec_zenoh_decl_queryable.encode_source(_v_body, w)
    elif isinstance(_v_body, sce_codec_zenoh_undecl_queryable.CodecZenohUndeclQueryable):
        sce_codec_zenoh_undecl_queryable.encode_source(_v_body, w)
    elif isinstance(_v_body, sce_codec_zenoh_decl_token.CodecZenohDeclToken):
        sce_codec_zenoh_decl_token.encode_source(_v_body, w)
    elif isinstance(_v_body, sce_codec_zenoh_undecl_token.CodecZenohUndeclToken):
        sce_codec_zenoh_undecl_token.encode_source(_v_body, w)
    elif isinstance(_v_body, sce_codec_zenoh_decl_final.CodecZenohDeclFinal):
        sce_codec_zenoh_decl_final.encode_source(_v_body, w)
    else:
        sce_codec_zenoh_decl_final.encode_source(_v_body.body, w)


@dataclass
class CodecZenohDeclaration:
    header: CodecZenohDeclarationHeaderInt = 0
    body: CodecZenohDeclarationVariant = field(default_factory=sce_codec_zenoh_decl_final.CodecZenohDeclFinal)

    def __setattr__(self, name: str, value: object) -> None:
        # Each integer, byte string and text is held to its bound
        # wherever it is set, where the value is made and after
        # (docs/adr/0081 §2), whatever it was handed.
        if name == "header":
            value = CodecZenohDeclarationHeaderInt(value)
        object.__setattr__(self, name, value)

    @classmethod
    def decode(cls, cursor: SceCursor) -> CodecZenohDeclaration:
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
        if ((f_header >> 0) & 0x1F) == 0:
            _arm = CodecZenohDeclKexpr.decode(cursor, ((f_header >> 5) & 0x1))
            body = _arm
        elif ((f_header >> 0) & 0x1F) == 1:
            _arm = CodecZenohUndeclKexpr.decode(cursor)
            body = _arm
        elif ((f_header >> 0) & 0x1F) == 2:
            _arm = CodecZenohDeclSubscriber.decode(cursor, ((f_header >> 5) & 0x1))
            body = _arm
        elif ((f_header >> 0) & 0x1F) == 3:
            _arm = CodecZenohUndeclSubscriber.decode(cursor, ((f_header >> 7) & 0x1))
            body = _arm
        elif ((f_header >> 0) & 0x1F) == 4:
            _arm = CodecZenohDeclQueryable.decode(cursor, ((f_header >> 5) & 0x1), ((f_header >> 7) & 0x1))
            body = _arm
        elif ((f_header >> 0) & 0x1F) == 5:
            _arm = CodecZenohUndeclQueryable.decode(cursor, ((f_header >> 7) & 0x1))
            body = _arm
        elif ((f_header >> 0) & 0x1F) == 6:
            _arm = CodecZenohDeclToken.decode(cursor, ((f_header >> 5) & 0x1))
            body = _arm
        elif ((f_header >> 0) & 0x1F) == 7:
            _arm = CodecZenohUndeclToken.decode(cursor, ((f_header >> 7) & 0x1))
            body = _arm
        elif ((f_header >> 0) & 0x1F) == 26:
            _arm = CodecZenohDeclFinal.decode(cursor)
            body = _arm
        else:
            _arm = CodecZenohDeclFinal.decode(cursor)
            body = CodecZenohDeclarationDefault(tag=((f_header >> 0) & 0x1F), body=_arm)
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
