# SCE-MAP: codec_variant_dispatch:8 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, SceCursor, SceSink
from sce_forge_runtime.bounded import UnknownTag
from .codec_variant_session_open import CodecVariantSessionOpen
from .codec_variant_session_close import CodecVariantSessionClose
from . import codec_variant_session_close as sce_codec_variant_session_close
from . import codec_variant_session_open as sce_codec_variant_session_open

from dataclasses import dataclass, field
from typing import Optional, Protocol, Union


class CodecVariantDispatchUnknownTag(UnknownTag):
    """The tag the default arm of :class:`CodecVariantDispatch`'s variant keeps: at
    most 255, and none an arm names (docs/adr/0065 §6 R3)."""

    MOST = 255
    KNOWN = frozenset({1, 2})


@dataclass
class CodecVariantDispatchDefault:
    """The default arm of :class:`CodecVariantDispatch`'s variant: the tag no arm
    names, which it keeps, and its body."""

    tag: CodecVariantDispatchUnknownTag = CodecVariantDispatchUnknownTag(0)
    body: sce_codec_variant_session_close.CodecVariantSessionClose = field(default_factory=lambda: sce_codec_variant_session_close.CodecVariantSessionClose())

    def __setattr__(self, name: str, value: object) -> None:
        # The tag is held to the tags no arm names wherever it is set,
        # where the arm is made and after (docs/adr/0065 §6 R3).
        if name == "tag":
            value = CodecVariantDispatchUnknownTag(value)
        object.__setattr__(self, name, value)


CodecVariantDispatchVariant = Union[sce_codec_variant_session_open.CodecVariantSessionOpen, sce_codec_variant_session_close.CodecVariantSessionClose, CodecVariantDispatchDefault]
"""The arm a value of :class:`CodecVariantDispatch` holds: a value of the arm's codec,
or the default arm."""


ARM_CLASSES = {0x1: sce_codec_variant_session_open.CodecVariantSessionOpen, 0x2: sce_codec_variant_session_close.CodecVariantSessionClose}
"""The class of the arm of :class:`CodecVariantDispatch`'s variant each tag names."""


class CodecVariantDispatchSource(Protocol):
    """The elements a value of :class:`CodecVariantDispatch` presents, which its encode
    reads (docs/adr/0081 §2): any object that presents them is a source of it.
    The encode derives every other element of the frame."""

    @property
    def body(self) -> CodecVariantDispatchVariant: ...


def encode_source(src: CodecVariantDispatchSource, w: SceSink) -> None:
    """Write any source of :class:`CodecVariantDispatch` into ``w``, each element read
    off ``src`` and written where the codec's encode plan lays it out
    (docs/adr/0081). It refuses no value; a bounded sink raises
    :class:`BufferOverflow` where it has no room left."""
    _v_body = src.body
    if not isinstance(_v_body, (sce_codec_variant_session_open.CodecVariantSessionOpen, sce_codec_variant_session_close.CodecVariantSessionClose, CodecVariantDispatchDefault,)):
        raise TypeError(f"{type(_v_body).__name__} is no arm of CodecVariantDispatch's variant")
    _t_body = (1 if isinstance(_v_body, sce_codec_variant_session_open.CodecVariantSessionOpen) else 2 if isinstance(_v_body, sce_codec_variant_session_close.CodecVariantSessionClose) else _v_body.tag)
    _d_msg_id = _t_body
    w.write_u8(_d_msg_id & 0xFF)
    if isinstance(_v_body, sce_codec_variant_session_open.CodecVariantSessionOpen):
        sce_codec_variant_session_open.encode_source(_v_body, w)
    elif isinstance(_v_body, sce_codec_variant_session_close.CodecVariantSessionClose):
        sce_codec_variant_session_close.encode_source(_v_body, w)
    else:
        sce_codec_variant_session_close.encode_source(_v_body.body, w)


@dataclass
class CodecVariantDispatch:
    body: CodecVariantDispatchVariant = field(default_factory=lambda: sce_codec_variant_session_close.CodecVariantSessionClose())

    @classmethod
    def decode(cls, cursor: SceCursor) -> CodecVariantDispatch:
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
        f_msg_id = raw[0]
        cursor.advance(1)
        # Dispatch on the tag field; each arm decodes its body codec
        # from the cursor. The default arm (when declared) carries the
        # runtime tag value so encode can round-trip it back onto the
        # wire.
        if f_msg_id == 1:
            _arm = CodecVariantSessionOpen.decode(cursor)
            body = _arm
        elif f_msg_id == 2:
            _arm = CodecVariantSessionClose.decode(cursor)
            body = _arm
        else:
            _arm = CodecVariantSessionClose.decode(cursor)
            body = CodecVariantDispatchDefault(tag=f_msg_id, body=_arm)
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
