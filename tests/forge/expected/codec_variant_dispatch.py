# SCE-MAP: codec_variant_dispatch:8

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, SceCursor, SceSink
from .codec_variant_session_open import CodecVariantSessionOpen
from .codec_variant_session_close import CodecVariantSessionClose

from dataclasses import dataclass, field
from typing import Optional


@dataclass
class CodecVariantDispatchVariant:
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
    kind: str = "CodecVariantSessionClose"
    codec_variant_session_open: Optional[CodecVariantSessionOpen] = None
    codec_variant_session_close: Optional[CodecVariantSessionClose] = field(default_factory=CodecVariantSessionClose)
    default_body: Optional[CodecVariantSessionClose] = None
    default_tag: int = 0


@dataclass
class CodecVariantDispatch:
    msg_id: int = 0
    body: CodecVariantDispatchVariant = field(default_factory=CodecVariantDispatchVariant)

    @classmethod
    def decode(cls, cursor: SceCursor) -> Optional[CodecVariantDispatch]:
        """Decode the next frame from ``cursor``. Returns ``None`` when
        the cursor's tail is shorter than the declared minimum frame
        (RFC §synth-5-B L494-519); on success the cursor advances past the
        consumed bytes. VLE codecs also return ``None`` on
        ``VleWidthOverflow``."""
        # Decode fixed prefix (RFC §synth-5-B variant: fields before tag suffix).
        try:
            raw = cursor.peek_slice(1)
        except NeedMoreBytes:
            return None
        msg_id = raw[0]
        try:
            cursor.advance(1)
        except NeedMoreBytes:
            return None
        # Dispatch on the tag field; each arm decodes its body codec
        # from the cursor. The default arm (when declared) carries the
        # runtime tag value so encode can round-trip it back onto the
        # wire.
        body = CodecVariantDispatchVariant()
        if msg_id == 1:
            body.kind = "CodecVariantSessionOpen"
            _arm = CodecVariantSessionOpen.decode(cursor)
            if _arm is None:
                return None
            body.codec_variant_session_open = _arm
        elif msg_id == 2:
            body.kind = "CodecVariantSessionClose"
            _arm = CodecVariantSessionClose.decode(cursor)
            if _arm is None:
                return None
            body.codec_variant_session_close = _arm
        else:
            body.kind = "Default"
            body.default_tag = msg_id
            _arm = CodecVariantSessionClose.decode(cursor)
            if _arm is None:
                return None
            body.default_body = _arm
        return cls(
            msg_id=msg_id,
            body=body,
        )

    def encode(self, w: SceSink) -> None:
        """RFC §synth-5-B encode-side primary: write ``self`` into the
        caller-owned ``w`` sink. Returns ``None`` on success; raises
        :class:`BufferOverflow` from a bounded sink when the destination
        has insufficient remaining capacity; growable sinks (e.g.
        :class:`BytearraySink`) are effectively infallible."""
        # Encode fixed prefix (tag field bytes are part of the prefix).
        w.write_u8(self.msg_id & 0xFF)
        # Append the active arm body's encoded bytes via the same sink.
        if self.body.kind == "CodecVariantSessionOpen":
            self.body.codec_variant_session_open.encode(w)
        elif self.body.kind == "CodecVariantSessionClose":
            self.body.codec_variant_session_close.encode(w)
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
