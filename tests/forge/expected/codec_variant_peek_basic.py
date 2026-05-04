# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import CodecError, NeedMoreBytes, SceCursor
from .codec_peek_arm_a import CodecPeekArmA
from .codec_peek_arm_b import CodecPeekArmB

from dataclasses import dataclass, field
from typing import Optional


@dataclass
class CodecVariantPeekBasicVariant:
    """RFC §5.B variant primitive (B1-β): discriminated-union body for
    the codec's tag-field suffix. ``kind`` selects the active arm; the
    matching ``Optional`` field carries the decoded body. ``default_tag``
    preserves the runtime tag value when the default arm fires so encode
    can round-trip it back onto the wire."""
    # Default to the first declared arm (or "Default" when arms is empty)
    # so a freshly-constructed envelope round-trips through encode without
    # needing the caller to populate the body explicitly.
    kind: str = "CodecPeekArmA"
    codec_peek_arm_a: Optional[CodecPeekArmA] = None
    codec_peek_arm_b: Optional[CodecPeekArmB] = None


@dataclass
class CodecVariantPeekBasic:
    body: CodecVariantPeekBasicVariant = field(default_factory=CodecVariantPeekBasicVariant)

    @classmethod
    def decode(cls, cursor: SceCursor) -> Optional[CodecVariantPeekBasic]:
        """Decode the next frame from ``cursor``. Returns ``None`` when
        the cursor's tail is shorter than the declared minimum frame
        (RFC §5.B L494-519); on success the cursor advances past the
        consumed bytes. VLE codecs also return ``None`` on
        ``VleWidthOverflow``."""
        # RFC §5.B Y3 atomic 2b-ii peek-byte / 2b-iv streaming-prefix:
        # streaming prefix decode (variable-length fields supported via
        # per-field present_if/tlv-chain/embed/repeat helpers). Peek-byte
        # mode additionally peeks the cursor's next byte for variant tag
        # without advancing — arm body decoder reads it as own header.
        try:
            _peek = cursor.peek_slice(1)[0]
        except NeedMoreBytes:
            return None
        # Dispatch on the tag field; each arm decodes its body codec
        # from the cursor. The default arm (when declared) carries the
        # runtime tag value so encode can round-trip it back onto the
        # wire.
        body = CodecVariantPeekBasicVariant()
        if ((_peek >> 0) & 0x01) == 0:
            body.kind = "CodecPeekArmA"
            _arm = CodecPeekArmA.decode(cursor)
            if _arm is None:
                return None
            body.codec_peek_arm_a = _arm
        elif ((_peek >> 0) & 0x01) == 1:
            body.kind = "CodecPeekArmB"
            _arm = CodecPeekArmB.decode(cursor)
            if _arm is None:
                return None
            body.codec_peek_arm_b = _arm
        else:
            # codec/variant-arm-unreachable rejected this case at parse time.
            return None
        return cls(
            body=body,
        )

    def encode(self) -> bytes:
        # RFC §5.B Y3 atomic 2b-ii peek-byte / 2b-iv streaming-prefix:
        # streaming prefix encode. Peek-byte mode: arm body's encode
        # prepends its own header byte (which the decoder peeked); no
        # separate tag byte here. Streaming-prefix mode (own-field):
        # carrier is part of the prefix fields and emits via the same
        # per-field path.
        r = bytearray()
        # Append the active arm body's encoded bytes.
        if self.body.kind == "CodecPeekArmA":
            r.extend(self.body.codec_peek_arm_a.encode())
        elif self.body.kind == "CodecPeekArmB":
            r.extend(self.body.codec_peek_arm_b.encode())
        return bytes(r)
