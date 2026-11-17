# SCE-MAP: codec_variant_peek_basic:29 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, SceCursor, SceSink
from .codec_peek_arm_a import CodecPeekArmA
from .codec_peek_arm_b import CodecPeekArmB

from dataclasses import dataclass, field
from typing import Optional


@dataclass
class CodecVariantPeekBasicVariant:
    """RFC §synth-5-B variant primitive: discriminated-union body for
    the codec's tag-field suffix. ``kind`` selects the active arm; the
    matching ``Optional`` field carries the decoded body. ``default_tag``
    preserves the runtime tag value when the default arm fires so encode
    can round-trip it back onto the wire."""
    # RFC variant-default-uniformity (Python): pick the declared
    # default arm (``<sce:arm default="true"/>``) when present so a
    # freshly-constructed envelope round-trips byte-exactly through
    # ``encode() -> decode()``. The corresponding arm body field uses a
    # default_factory so ``Variant()`` actually populates it (rather
    # than leaving every arm field ``None`` while ``kind`` names one of
    # them, which is the latent inconsistency this RFC closes).
    kind: str = "CodecPeekArmA"
    codec_peek_arm_a: Optional[CodecPeekArmA] = field(default_factory=CodecPeekArmA)
    codec_peek_arm_b: Optional[CodecPeekArmB] = None


@dataclass
class CodecVariantPeekBasic:
    body: CodecVariantPeekBasicVariant = field(default_factory=CodecVariantPeekBasicVariant)

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
        body = CodecVariantPeekBasicVariant()
        if ((_peek >> 0) & 0x01) == 0:
            body.kind = "CodecPeekArmA"
            body.codec_peek_arm_a = CodecPeekArmA.decode(cursor)
        elif ((_peek >> 0) & 0x01) == 1:
            body.kind = "CodecPeekArmB"
            body.codec_peek_arm_b = CodecPeekArmB.decode(cursor)
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
        # RFC §synth-5-B peek-byte / streaming-prefix:
        # streaming prefix encode.
        # Append the active arm body's encoded bytes via the same sink.
        if self.body.kind == "CodecPeekArmA":
            self.body.codec_peek_arm_a.encode(w)
        elif self.body.kind == "CodecPeekArmB":
            self.body.codec_peek_arm_b.encode(w)

    def encode_to_bytes(self) -> bytes:
        """Heap-backed convenience facade. Runs :meth:`encode` over a
        :class:`BytearraySink` and returns the freshly-encoded bytes.
        Callers targeting zero-alloc hot paths should call :meth:`encode`
        directly against a caller-owned sink."""
        _dst = bytearray()
        self.encode(BytearraySink(_dst))
        return bytes(_dst)
