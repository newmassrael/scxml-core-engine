# SCE-MAP: codec_zenoh_network_envelope:60 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, SceCursor, SceSink
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
    def decode(cls, cursor: SceCursor) -> CodecZenohNetworkEnvelope:
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
        body = CodecZenohNetworkEnvelopeVariant()
        if ((_peek >> 0) & 0x1F) == 25:
            body.kind = "CodecZenohInterest"
            body.codec_zenoh_interest = CodecZenohInterest.decode(cursor)
        elif ((_peek >> 0) & 0x1F) == 26:
            body.kind = "CodecZenohResponseFinal"
            body.codec_zenoh_response_final = CodecZenohResponseFinal.decode(cursor)
        elif ((_peek >> 0) & 0x1F) == 27:
            body.kind = "CodecZenohResponse"
            body.codec_zenoh_response = CodecZenohResponse.decode(cursor)
        elif ((_peek >> 0) & 0x1F) == 28:
            body.kind = "CodecZenohRequest"
            body.codec_zenoh_request = CodecZenohRequest.decode(cursor)
        elif ((_peek >> 0) & 0x1F) == 29:
            body.kind = "CodecZenohPush"
            body.codec_zenoh_push = CodecZenohPush.decode(cursor)
        elif ((_peek >> 0) & 0x1F) == 30:
            body.kind = "CodecZenohDeclare"
            body.codec_zenoh_declare = CodecZenohDeclare.decode(cursor)
        elif ((_peek >> 0) & 0x1F) == 31:
            body.kind = "CodecZenohOam"
            body.codec_zenoh_oam = CodecZenohOam.decode(cursor)
        else:
            body.kind = "Default"
            body.default_tag = ((_peek >> 0) & 0x1F)
            body.default_body = CodecZenohOam.decode(cursor)
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
        if self.body.kind == "CodecZenohInterest":
            self.body.codec_zenoh_interest.encode(w)
        elif self.body.kind == "CodecZenohResponseFinal":
            self.body.codec_zenoh_response_final.encode(w)
        elif self.body.kind == "CodecZenohResponse":
            self.body.codec_zenoh_response.encode(w)
        elif self.body.kind == "CodecZenohRequest":
            self.body.codec_zenoh_request.encode(w)
        elif self.body.kind == "CodecZenohPush":
            self.body.codec_zenoh_push.encode(w)
        elif self.body.kind == "CodecZenohDeclare":
            self.body.codec_zenoh_declare.encode(w)
        elif self.body.kind == "CodecZenohOam":
            self.body.codec_zenoh_oam.encode(w)
        elif self.body.kind == "Default":
            self.body.default_body.encode(w)

    def encode_to_bytes(self) -> bytes:
        """Heap-backed convenience facade. Runs :meth:`encode` over a
        :class:`BytearraySink` and returns the freshly-encoded bytes.
        Callers targeting zero-alloc hot paths should call :meth:`encode`
        directly against a caller-owned sink."""
        _dst = bytearray()
        self.encode(BytearraySink(_dst))
        return bytes(_dst)
