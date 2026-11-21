# SCE-MAP: codec_zenoh_network_envelope:60 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, SceCursor, SceSink
from sce_forge_runtime.bounded import UnknownTag
from .codec_zenoh_interest import CodecZenohInterest
from .codec_zenoh_response_final import CodecZenohResponseFinal
from .codec_zenoh_response import CodecZenohResponse
from .codec_zenoh_request import CodecZenohRequest
from .codec_zenoh_push import CodecZenohPush
from .codec_zenoh_declare import CodecZenohDeclare
from .codec_zenoh_oam import CodecZenohOam
from . import codec_zenoh_declare as sce_codec_zenoh_declare
from . import codec_zenoh_interest as sce_codec_zenoh_interest
from . import codec_zenoh_oam as sce_codec_zenoh_oam
from . import codec_zenoh_push as sce_codec_zenoh_push
from . import codec_zenoh_request as sce_codec_zenoh_request
from . import codec_zenoh_response as sce_codec_zenoh_response
from . import codec_zenoh_response_final as sce_codec_zenoh_response_final

from dataclasses import dataclass, field
from typing import Optional, Protocol, Union


class CodecZenohNetworkEnvelopeUnknownTag(UnknownTag):
    """The tag the default arm of :class:`CodecZenohNetworkEnvelope`'s variant keeps: at
    most 31, and none an arm names (docs/adr/0065 §6 R3)."""

    MOST = 31
    KNOWN = frozenset({25, 26, 27, 28, 29, 30, 31})


@dataclass
class CodecZenohNetworkEnvelopeDefault:
    """The default arm of :class:`CodecZenohNetworkEnvelope`'s variant: the tag no arm
    names, which it keeps, and its body."""

    tag: CodecZenohNetworkEnvelopeUnknownTag = CodecZenohNetworkEnvelopeUnknownTag(0)
    body: sce_codec_zenoh_oam.CodecZenohOam = field(default_factory=lambda: sce_codec_zenoh_oam.CodecZenohOam())

    def __setattr__(self, name: str, value: object) -> None:
        # The tag is held to the tags no arm names wherever it is set,
        # where the arm is made and after (docs/adr/0065 §6 R3).
        if name == "tag":
            value = CodecZenohNetworkEnvelopeUnknownTag(value)
        object.__setattr__(self, name, value)


CodecZenohNetworkEnvelopeVariant = Union[sce_codec_zenoh_interest.CodecZenohInterest, sce_codec_zenoh_response_final.CodecZenohResponseFinal, sce_codec_zenoh_response.CodecZenohResponse, sce_codec_zenoh_request.CodecZenohRequest, sce_codec_zenoh_push.CodecZenohPush, sce_codec_zenoh_declare.CodecZenohDeclare, sce_codec_zenoh_oam.CodecZenohOam, CodecZenohNetworkEnvelopeDefault]
"""The arm a value of :class:`CodecZenohNetworkEnvelope` holds: a value of the arm's codec,
or the default arm."""


ARM_CLASSES = {0x19: sce_codec_zenoh_interest.CodecZenohInterest, 0x1A: sce_codec_zenoh_response_final.CodecZenohResponseFinal, 0x1B: sce_codec_zenoh_response.CodecZenohResponse, 0x1C: sce_codec_zenoh_request.CodecZenohRequest, 0x1D: sce_codec_zenoh_push.CodecZenohPush, 0x1E: sce_codec_zenoh_declare.CodecZenohDeclare, 0x1F: sce_codec_zenoh_oam.CodecZenohOam}
"""The class of the arm of :class:`CodecZenohNetworkEnvelope`'s variant each tag names."""


class CodecZenohNetworkEnvelopeSource(Protocol):
    """The elements a value of :class:`CodecZenohNetworkEnvelope` presents, which its encode
    reads (docs/adr/0081 §2): any object that presents them is a source of it.
    The encode derives every other element of the frame."""

    @property
    def body(self) -> CodecZenohNetworkEnvelopeVariant: ...


def encode_source(src: CodecZenohNetworkEnvelopeSource, w: SceSink) -> None:
    """Write any source of :class:`CodecZenohNetworkEnvelope` into ``w``, each element read
    off ``src`` and written where the codec's encode plan lays it out
    (docs/adr/0081). It refuses no value; a bounded sink raises
    :class:`BufferOverflow` where it has no room left."""
    _v_body = src.body
    if not isinstance(_v_body, (sce_codec_zenoh_interest.CodecZenohInterest, sce_codec_zenoh_response_final.CodecZenohResponseFinal, sce_codec_zenoh_response.CodecZenohResponse, sce_codec_zenoh_request.CodecZenohRequest, sce_codec_zenoh_push.CodecZenohPush, sce_codec_zenoh_declare.CodecZenohDeclare, sce_codec_zenoh_oam.CodecZenohOam, CodecZenohNetworkEnvelopeDefault,)):
        raise TypeError(f"{type(_v_body).__name__} is no arm of CodecZenohNetworkEnvelope's variant")
    if isinstance(_v_body, sce_codec_zenoh_interest.CodecZenohInterest):
        sce_codec_zenoh_interest.encode_source(_v_body, w)
    elif isinstance(_v_body, sce_codec_zenoh_response_final.CodecZenohResponseFinal):
        sce_codec_zenoh_response_final.encode_source(_v_body, w)
    elif isinstance(_v_body, sce_codec_zenoh_response.CodecZenohResponse):
        sce_codec_zenoh_response.encode_source(_v_body, w)
    elif isinstance(_v_body, sce_codec_zenoh_request.CodecZenohRequest):
        sce_codec_zenoh_request.encode_source(_v_body, w)
    elif isinstance(_v_body, sce_codec_zenoh_push.CodecZenohPush):
        sce_codec_zenoh_push.encode_source(_v_body, w)
    elif isinstance(_v_body, sce_codec_zenoh_declare.CodecZenohDeclare):
        sce_codec_zenoh_declare.encode_source(_v_body, w)
    elif isinstance(_v_body, sce_codec_zenoh_oam.CodecZenohOam):
        sce_codec_zenoh_oam.encode_source(_v_body, w)
    else:
        sce_codec_zenoh_oam.encode_source(_v_body.body, w)


@dataclass
class CodecZenohNetworkEnvelope:
    body: CodecZenohNetworkEnvelopeVariant = field(default_factory=lambda: sce_codec_zenoh_oam.CodecZenohOam())

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
        if ((_peek >> 0) & 0x1F) == 25:
            _arm = CodecZenohInterest.decode(cursor)
            body = _arm
        elif ((_peek >> 0) & 0x1F) == 26:
            _arm = CodecZenohResponseFinal.decode(cursor)
            body = _arm
        elif ((_peek >> 0) & 0x1F) == 27:
            _arm = CodecZenohResponse.decode(cursor)
            body = _arm
        elif ((_peek >> 0) & 0x1F) == 28:
            _arm = CodecZenohRequest.decode(cursor)
            body = _arm
        elif ((_peek >> 0) & 0x1F) == 29:
            _arm = CodecZenohPush.decode(cursor)
            body = _arm
        elif ((_peek >> 0) & 0x1F) == 30:
            _arm = CodecZenohDeclare.decode(cursor)
            body = _arm
        elif ((_peek >> 0) & 0x1F) == 31:
            _arm = CodecZenohOam.decode(cursor)
            body = _arm
        else:
            _arm = CodecZenohOam.decode(cursor)
            body = CodecZenohNetworkEnvelopeDefault(tag=((_peek >> 0) & 0x1F), body=_arm)
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
