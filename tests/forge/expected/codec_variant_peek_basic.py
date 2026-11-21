# SCE-MAP: codec_variant_peek_basic:29 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, SceCursor, SceSink
from .codec_peek_arm_a import CodecPeekArmA
from .codec_peek_arm_b import CodecPeekArmB
from . import codec_peek_arm_a as sce_codec_peek_arm_a
from . import codec_peek_arm_b as sce_codec_peek_arm_b

from dataclasses import dataclass, field
from typing import Optional, Protocol, Union


CodecVariantPeekBasicVariant = Union[sce_codec_peek_arm_a.CodecPeekArmA, sce_codec_peek_arm_b.CodecPeekArmB]
"""The arm a value of :class:`CodecVariantPeekBasic` holds: a value of the arm's codec,
or the default arm."""


ARM_CLASSES = {0x0: sce_codec_peek_arm_a.CodecPeekArmA, 0x1: sce_codec_peek_arm_b.CodecPeekArmB}
"""The class of the arm of :class:`CodecVariantPeekBasic`'s variant each tag names."""


class CodecVariantPeekBasicSource(Protocol):
    """The elements a value of :class:`CodecVariantPeekBasic` presents, which its encode
    reads (docs/adr/0081 §2): any object that presents them is a source of it.
    The encode derives every other element of the frame."""

    @property
    def body(self) -> CodecVariantPeekBasicVariant: ...


def encode_source(src: CodecVariantPeekBasicSource, w: SceSink) -> None:
    """Write any source of :class:`CodecVariantPeekBasic` into ``w``, each element read
    off ``src`` and written where the codec's encode plan lays it out
    (docs/adr/0081). It refuses no value; a bounded sink raises
    :class:`BufferOverflow` where it has no room left."""
    _v_body = src.body
    if not isinstance(_v_body, (sce_codec_peek_arm_a.CodecPeekArmA, sce_codec_peek_arm_b.CodecPeekArmB,)):
        raise TypeError(f"{type(_v_body).__name__} is no arm of CodecVariantPeekBasic's variant")
    if isinstance(_v_body, sce_codec_peek_arm_a.CodecPeekArmA):
        sce_codec_peek_arm_a.encode_source(_v_body, w)
    else:
        sce_codec_peek_arm_b.encode_source(_v_body, w)


@dataclass
class CodecVariantPeekBasic:
    body: CodecVariantPeekBasicVariant = field(default_factory=lambda: sce_codec_peek_arm_a.CodecPeekArmA())

    @classmethod
    def decode(cls, cursor: SceCursor) -> CodecVariantPeekBasic:
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
        _peek = cursor.peek_slice(1)[0]
        # Dispatch on the tag field; each arm decodes its body codec
        # from the cursor. The default arm (when declared) carries the
        # runtime tag value so encode can round-trip it back onto the
        # wire.
        if ((_peek >> 0) & 0x01) == 0:
            _arm = CodecPeekArmA.decode(cursor)
            body = _arm
        elif ((_peek >> 0) & 0x01) == 1:
            _arm = CodecPeekArmB.decode(cursor)
            body = _arm
        else:
            # codec/variant-arm-unreachable proves every tag names an arm, so
            # no frame reaches here.
            raise AssertionError("codec/variant-arm-unreachable admitted a tag no arm names")
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
