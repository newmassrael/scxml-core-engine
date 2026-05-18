# SCE-MAP: codec_zenoh_network_envelope:60

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import CodecError, NeedMoreBytes, SceCursor
from .codec_zenoh_interest import CodecZenohInterest
from .codec_zenoh_response_final import CodecZenohResponseFinal
from .codec_zenoh_response import CodecZenohResponse
from .codec_zenoh_request import CodecZenohRequest
from .codec_zenoh_push import CodecZenohPush
from .codec_zenoh_declare import CodecZenohDeclare
from .codec_zenoh_oam import CodecZenohOam

from dataclasses import dataclass, field
from typing import Optional


@dataclass
class CodecZenohNetworkEnvelopeVariant:
    """RFC §5.B variant primitive (B1-β): discriminated-union body for
    the codec's tag-field suffix. ``kind`` selects the active arm; the
    matching ``Optional`` field carries the decoded body. ``default_tag``
    preserves the runtime tag value when the default arm fires so encode
    can round-trip it back onto the wire."""
    # RFC variant-default-uniformity Atomic β-python: pick the declared
    # default arm (``<sce:arm default="true"/>``) when present so a
    # freshly-constructed envelope round-trips byte-exactly through
    # ``encode() -> decode()``. The corresponding arm body field uses a
    # default_factory so ``Variant()`` actually populates it (rather
    # than leaving every arm field ``None`` while ``kind`` names one of
    # them, which is the latent inconsistency this RFC closes).
    kind: str = "CodecZenohOam"
    codec_zenoh_interest: Optional[CodecZenohInterest] = None
    codec_zenoh_response_final: Optional[CodecZenohResponseFinal] = None
    codec_zenoh_response: Optional[CodecZenohResponse] = None
    codec_zenoh_request: Optional[CodecZenohRequest] = None
    codec_zenoh_push: Optional[CodecZenohPush] = None
    codec_zenoh_declare: Optional[CodecZenohDeclare] = None
    codec_zenoh_oam: Optional[CodecZenohOam] = field(default_factory=CodecZenohOam)
    default_body: Optional[CodecZenohOam] = None
    default_tag: int = 0


@dataclass
class CodecZenohNetworkEnvelope:
    body: CodecZenohNetworkEnvelopeVariant = field(default_factory=CodecZenohNetworkEnvelopeVariant)

    @classmethod
    def decode(cls, cursor: SceCursor) -> Optional[CodecZenohNetworkEnvelope]:
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
        body = CodecZenohNetworkEnvelopeVariant()
        if ((_peek >> 0) & 0x1F) == 25:
            body.kind = "CodecZenohInterest"
            _arm = CodecZenohInterest.decode(cursor)
            if _arm is None:
                return None
            body.codec_zenoh_interest = _arm
        elif ((_peek >> 0) & 0x1F) == 26:
            body.kind = "CodecZenohResponseFinal"
            _arm = CodecZenohResponseFinal.decode(cursor)
            if _arm is None:
                return None
            body.codec_zenoh_response_final = _arm
        elif ((_peek >> 0) & 0x1F) == 27:
            body.kind = "CodecZenohResponse"
            _arm = CodecZenohResponse.decode(cursor)
            if _arm is None:
                return None
            body.codec_zenoh_response = _arm
        elif ((_peek >> 0) & 0x1F) == 28:
            body.kind = "CodecZenohRequest"
            _arm = CodecZenohRequest.decode(cursor)
            if _arm is None:
                return None
            body.codec_zenoh_request = _arm
        elif ((_peek >> 0) & 0x1F) == 29:
            body.kind = "CodecZenohPush"
            _arm = CodecZenohPush.decode(cursor)
            if _arm is None:
                return None
            body.codec_zenoh_push = _arm
        elif ((_peek >> 0) & 0x1F) == 30:
            body.kind = "CodecZenohDeclare"
            _arm = CodecZenohDeclare.decode(cursor)
            if _arm is None:
                return None
            body.codec_zenoh_declare = _arm
        elif ((_peek >> 0) & 0x1F) == 31:
            body.kind = "CodecZenohOam"
            _arm = CodecZenohOam.decode(cursor)
            if _arm is None:
                return None
            body.codec_zenoh_oam = _arm
        else:
            body.kind = "Default"
            body.default_tag = ((_peek >> 0) & 0x1F)
            _arm = CodecZenohOam.decode(cursor)
            if _arm is None:
                return None
            body.default_body = _arm
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
        if self.body.kind == "CodecZenohInterest":
            r.extend(self.body.codec_zenoh_interest.encode())
        elif self.body.kind == "CodecZenohResponseFinal":
            r.extend(self.body.codec_zenoh_response_final.encode())
        elif self.body.kind == "CodecZenohResponse":
            r.extend(self.body.codec_zenoh_response.encode())
        elif self.body.kind == "CodecZenohRequest":
            r.extend(self.body.codec_zenoh_request.encode())
        elif self.body.kind == "CodecZenohPush":
            r.extend(self.body.codec_zenoh_push.encode())
        elif self.body.kind == "CodecZenohDeclare":
            r.extend(self.body.codec_zenoh_declare.encode())
        elif self.body.kind == "CodecZenohOam":
            r.extend(self.body.codec_zenoh_oam.encode())
        elif self.body.kind == "Default":
            r.extend(self.body.default_body.encode())
        return bytes(r)
