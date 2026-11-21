# SCE-MAP: codec_zenoh_response:76 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, InvalidUtf8, LengthOutOfRange, NeedMoreBytes, SceCursor, SceSink, TooManyElements
from sce_forge_runtime.bounded import BoundedInt, BoundedText, BoundedTuple, UnknownTag
from .codec_zenoh_ext_entry import CodecZenohExtEntry
from .codec_zenoh_reply import CodecZenohReply
from .codec_zenoh_err import CodecZenohErr
from . import codec_zenoh_err as sce_codec_zenoh_err
from . import codec_zenoh_ext_entry as sce_codec_zenoh_ext_entry
from . import codec_zenoh_reply as sce_codec_zenoh_reply

from dataclasses import dataclass, field
from typing import Optional, List, Protocol, Union


class CodecZenohResponseHeaderInt(BoundedInt):
    """The ``header`` element of :class:`CodecZenohResponse`, an unsigned integer of
    8 bits, held to them where a value of it is made (docs/adr/0081 §2)."""

    BITS = 8
    SIGNED = False


class CodecZenohResponseRequestIdInt(BoundedInt):
    """The ``request_id`` element of :class:`CodecZenohResponse`, an unsigned integer of
    64 bits, held to them where a value of it is made (docs/adr/0081 §2)."""

    BITS = 64
    SIGNED = False


class CodecZenohResponseKeyIdInt(BoundedInt):
    """The ``key_id`` element of :class:`CodecZenohResponse`, an unsigned integer of
    32 bits, held to them where a value of it is made (docs/adr/0081 §2)."""

    BITS = 32
    SIGNED = False


class CodecZenohResponseSuffixText(BoundedText):
    """The ``suffix`` element of :class:`CodecZenohResponse`, 0 to 65535 UTF-8 bytes,
    held to that bound where a value of it is made (docs/adr/0077 §3)."""

    LEAST = 0
    MOST = 65535


class CodecZenohResponseExtensionsList(BoundedTuple):
    """The ``extensions`` element of :class:`CodecZenohResponse`, 1 or more entries, the protocol stating no most,
    held to that bound where a value of it is made (docs/adr/0077 §3)."""

    LEAST = 1
    MOST = None


class CodecZenohResponseUnknownTag(UnknownTag):
    """The tag the default arm of :class:`CodecZenohResponse`'s variant keeps: at
    most 31, and none an arm names (docs/adr/0065 §6 R3)."""

    MOST = 31
    KNOWN = frozenset({4, 5})


@dataclass
class CodecZenohResponseDefault:
    """The default arm of :class:`CodecZenohResponse`'s variant: the tag no arm
    names, which it keeps, and its body."""

    tag: CodecZenohResponseUnknownTag = CodecZenohResponseUnknownTag(0)
    body: sce_codec_zenoh_reply.CodecZenohReply = field(default_factory=lambda: sce_codec_zenoh_reply.CodecZenohReply())

    def __setattr__(self, name: str, value: object) -> None:
        # The tag is held to the tags no arm names wherever it is set,
        # where the arm is made and after (docs/adr/0065 §6 R3).
        if name == "tag":
            value = CodecZenohResponseUnknownTag(value)
        object.__setattr__(self, name, value)


CodecZenohResponseVariant = Union[sce_codec_zenoh_reply.CodecZenohReply, sce_codec_zenoh_err.CodecZenohErr, CodecZenohResponseDefault]
"""The arm a value of :class:`CodecZenohResponse` holds: a value of the arm's codec,
or the default arm."""


ARM_CLASSES = {0x4: sce_codec_zenoh_reply.CodecZenohReply, 0x5: sce_codec_zenoh_err.CodecZenohErr}
"""The class of the arm of :class:`CodecZenohResponse`'s variant each tag names."""


class CodecZenohResponseSource(Protocol):
    """The elements a value of :class:`CodecZenohResponse` presents, which its encode
    reads (docs/adr/0081 §2): any object that presents them is a source of it.
    The encode derives every other element of the frame."""

    @property
    def header(self) -> CodecZenohResponseHeaderInt: ...

    @property
    def request_id(self) -> CodecZenohResponseRequestIdInt: ...

    @property
    def key_id(self) -> CodecZenohResponseKeyIdInt: ...

    @property
    def suffix(self) -> Optional[CodecZenohResponseSuffixText]: ...

    @property
    def extensions(self) -> Optional[CodecZenohResponseExtensionsList]: ...

    @property
    def body(self) -> CodecZenohResponseVariant: ...


def encode_source(src: CodecZenohResponseSource, w: SceSink, *, override_header_m: Optional[int] = None) -> None:
    """Write any source of :class:`CodecZenohResponse` into ``w``, each element read
    off ``src`` and written where the codec's encode plan lays it out
    (docs/adr/0081). It derives ``suffix_len`` from what each measures, and never reads it off
    the source. A chain that holds it
    sets the flag its entries end on by its keyword, in place of the value's. It refuses no value; a bounded sink raises
    :class:`BufferOverflow` where it has no room left."""
    _v_body = src.body
    if not isinstance(_v_body, (sce_codec_zenoh_reply.CodecZenohReply, sce_codec_zenoh_err.CodecZenohErr, CodecZenohResponseDefault,)):
        raise TypeError(f"{type(_v_body).__name__} is no arm of CodecZenohResponse's variant")
    _d_suffix_len = (len(src.suffix.encode("utf-8"))) if src.suffix is not None else 0
    _c_header = src.header
    _c_header = (_c_header & ~0x20) | (0x20 if src.suffix is not None else 0)
    _c_header = (_c_header & ~0x80) | (0x80 if src.extensions is not None else 0)
    if override_header_m is not None:
        _c_header = (_c_header & ~0x40) | ((override_header_m & 0x1) << 6)
    w.write_u8(_c_header & 0xFF)
    w.write_vle_u64(src.request_id)
    w.write_vle_u32(src.key_id)
    if ((_c_header >> 5) & 1) != 0:
        w.write_vle_u64(_d_suffix_len)
    if src.suffix is not None:
        w.write_bytes(src.suffix.encode("utf-8"))
    if src.extensions is not None:
        _left_extensions = len(src.extensions)
        for _entry in src.extensions:
            _left_extensions -= 1
            sce_codec_zenoh_ext_entry.encode_source(_entry, w, override_header_z=1 if _left_extensions != 0 else 0)
    if isinstance(_v_body, sce_codec_zenoh_reply.CodecZenohReply):
        sce_codec_zenoh_reply.encode_source(_v_body, w)
    elif isinstance(_v_body, sce_codec_zenoh_err.CodecZenohErr):
        sce_codec_zenoh_err.encode_source(_v_body, w)
    else:
        sce_codec_zenoh_reply.encode_source(_v_body.body, w)


@dataclass
class CodecZenohResponse:
    header: CodecZenohResponseHeaderInt = 0x1b
    request_id: CodecZenohResponseRequestIdInt = 0
    key_id: CodecZenohResponseKeyIdInt = 0
    suffix: Optional[CodecZenohResponseSuffixText] = None
    extensions: Optional[CodecZenohResponseExtensionsList] = None
    body: CodecZenohResponseVariant = field(default_factory=lambda: sce_codec_zenoh_reply.CodecZenohReply())

    def __setattr__(self, name: str, value: object) -> None:
        # Each integer, byte string and text is held to its bound
        # wherever it is set, where the value is made and after
        # (docs/adr/0081 §2), whatever it was handed.
        if name == "header":
            value = CodecZenohResponseHeaderInt(value)
        elif name == "request_id":
            value = CodecZenohResponseRequestIdInt(value)
        elif name == "key_id":
            value = CodecZenohResponseKeyIdInt(value)
        elif name == "suffix":
            value = None if value is None else CodecZenohResponseSuffixText(value)
        elif name == "extensions":
            value = None if value is None else CodecZenohResponseExtensionsList(value)
        object.__setattr__(self, name, value)

    @classmethod
    def decode(cls, cursor: SceCursor) -> CodecZenohResponse:
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
        f_request_id = cursor.read_vle_u64()
        f_key_id = cursor.read_vle_u32()
        if (f_header & 0x20) != 0:
            _v = cursor.read_vle_u64()
            f_suffix_len = _v
        else:
            f_suffix_len = None
        if (f_header & 0x20) != 0:
            _len = f_suffix_len
            if _len > 65535:
                raise LengthOutOfRange()
            _n = _len
            raw = cursor.peek_slice(_n)
            try:
                _v = bytes(raw).decode('utf-8')
            except UnicodeDecodeError as exc:
                raise InvalidUtf8() from exc
            cursor.advance(_n)
            f_suffix = _v
        else:
            f_suffix = None
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
        if ((_peek >> 0) & 0x1F) == 4:
            _arm = CodecZenohReply.decode(cursor)
            body = _arm
        elif ((_peek >> 0) & 0x1F) == 5:
            _arm = CodecZenohErr.decode(cursor)
            body = _arm
        else:
            _arm = CodecZenohReply.decode(cursor)
            body = CodecZenohResponseDefault(tag=((_peek >> 0) & 0x1F), body=_arm)
        return cls(
            header=f_header,
            request_id=f_request_id,
            key_id=f_key_id,
            suffix=f_suffix,
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
