# SCE-MAP: codec_origin_choice:9 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, SceCursor, SceSink
from sce_forge_runtime.bounded import UnknownTag
from .codec_origin_leaf import CodecOriginLeaf
from .codec_origin_scalar import CodecOriginScalar
from . import codec_origin_leaf as sce_codec_origin_leaf
from . import codec_origin_scalar as sce_codec_origin_scalar

from dataclasses import dataclass, field
from typing import Optional, Protocol, Union


class CodecOriginChoiceUnknownTag(UnknownTag):
    """The tag the default arm of :class:`CodecOriginChoice`'s variant keeps: at
    most 255, and none an arm names (docs/adr/0065 §6 R3)."""

    MOST = 255
    KNOWN = frozenset({1, 2})


@dataclass
class CodecOriginChoiceDefault:
    """The default arm of :class:`CodecOriginChoice`'s variant: the tag no arm
    names, which it keeps, and its body."""

    tag: CodecOriginChoiceUnknownTag = CodecOriginChoiceUnknownTag(0)
    body: sce_codec_origin_scalar.CodecOriginScalar = field(default_factory=sce_codec_origin_scalar.CodecOriginScalar)

    def __setattr__(self, name: str, value: object) -> None:
        # The tag is held to the tags no arm names wherever it is set,
        # where the arm is made and after (docs/adr/0065 §6 R3).
        if name == "tag":
            value = CodecOriginChoiceUnknownTag(value)
        object.__setattr__(self, name, value)


CodecOriginChoiceVariant = Union[sce_codec_origin_leaf.CodecOriginLeaf, sce_codec_origin_scalar.CodecOriginScalar, CodecOriginChoiceDefault]
"""The arm a value of :class:`CodecOriginChoice` holds: a value of the arm's codec,
or the default arm."""


ARM_CLASSES = {0x1: sce_codec_origin_leaf.CodecOriginLeaf, 0x2: sce_codec_origin_scalar.CodecOriginScalar}
"""The class of the arm of :class:`CodecOriginChoice`'s variant each tag names."""


class CodecOriginChoiceSource(Protocol):
    """The elements a value of :class:`CodecOriginChoice` presents, which its encode
    reads (docs/adr/0081 §2): any object that presents them is a source of it.
    The encode derives every other element of the frame."""

    @property
    def body(self) -> CodecOriginChoiceVariant: ...


def encode_source(src: CodecOriginChoiceSource, w: SceSink) -> None:
    """Write any source of :class:`CodecOriginChoice` into ``w``, each element read
    off ``src`` and written where the codec's encode plan lays it out
    (docs/adr/0081). It refuses no value; a bounded sink raises
    :class:`BufferOverflow` where it has no room left."""
    _v_body = src.body
    if not isinstance(_v_body, (sce_codec_origin_leaf.CodecOriginLeaf, sce_codec_origin_scalar.CodecOriginScalar, CodecOriginChoiceDefault,)):
        raise TypeError(f"{type(_v_body).__name__} is no arm of CodecOriginChoice's variant")
    _t_body = (1 if isinstance(_v_body, sce_codec_origin_leaf.CodecOriginLeaf) else 2 if isinstance(_v_body, sce_codec_origin_scalar.CodecOriginScalar) else _v_body.tag)
    _d_tag = _t_body
    w.write_u8(_d_tag & 0xFF)
    if isinstance(_v_body, sce_codec_origin_leaf.CodecOriginLeaf):
        sce_codec_origin_leaf.encode_source(_v_body, w)
    elif isinstance(_v_body, sce_codec_origin_scalar.CodecOriginScalar):
        sce_codec_origin_scalar.encode_source(_v_body, w)
    else:
        sce_codec_origin_scalar.encode_source(_v_body.body, w)


@dataclass
class CodecOriginChoice:
    body: CodecOriginChoiceVariant = field(default_factory=sce_codec_origin_scalar.CodecOriginScalar)

    @classmethod
    def decode(cls, cursor: SceCursor) -> CodecOriginChoice:
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
        f_tag = raw[0]
        cursor.advance(1)
        # Dispatch on the tag field; each arm decodes its body codec
        # from the cursor. The default arm (when declared) carries the
        # runtime tag value so encode can round-trip it back onto the
        # wire.
        if f_tag == 1:
            _arm = CodecOriginLeaf.decode(cursor)
            body = _arm
        elif f_tag == 2:
            _arm = CodecOriginScalar.decode(cursor)
            body = _arm
        else:
            _arm = CodecOriginScalar.decode(cursor)
            body = CodecOriginChoiceDefault(tag=f_tag, body=_arm)
        return cls(
            body=body,
        )

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
