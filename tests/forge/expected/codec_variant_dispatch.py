# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import CodecError, NeedMoreBytes, SceCursor
from .codec_variant_session_open import CodecVariantSessionOpen
from .codec_variant_session_close import CodecVariantSessionClose

from dataclasses import dataclass, field
from typing import Optional


@dataclass
class CodecVariantDispatchVariant:
    """RFC §5.B variant primitive (B1-β): discriminated-union body for
    the codec's tag-field suffix. ``kind`` selects the active arm; the
    matching ``Optional`` field carries the decoded body. ``default_tag``
    preserves the runtime tag value when the default arm fires so encode
    can round-trip it back onto the wire."""
    # Default to the first declared arm (or "Default" when arms is empty)
    # so a freshly-constructed envelope round-trips through encode without
    # needing the caller to populate the body explicitly.
    kind: str = "CodecVariantSessionOpen"
    codec_variant_session_open: Optional[CodecVariantSessionOpen] = None
    codec_variant_session_close: Optional[CodecVariantSessionClose] = None
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
        (RFC §5.B L494-519); on success the cursor advances past the
        consumed bytes. VLE codecs also return ``None`` on
        ``VleWidthOverflow``."""
        # Decode fixed prefix (RFC §5.B variant B1-β: fields before tag suffix).
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

    def encode(self) -> bytes:
        # Encode fixed prefix (tag field bytes are part of the prefix).
        # The tag value is read from the struct field, NOT derived from
        # the body discriminant — keeping author-set tag / body in sync
        # is the caller's responsibility (v1 keeps the layout simple).
        r = bytearray()
        r.append(self.msg_id & 0xFF)
        # Append the active arm body's encoded bytes.
        if self.body.kind == "CodecVariantSessionOpen":
            r.extend(self.body.codec_variant_session_open.encode())
        elif self.body.kind == "CodecVariantSessionClose":
            r.extend(self.body.codec_variant_session_close.encode())
        elif self.body.kind == "Default":
            r.extend(self.body.default_body.encode())
        return bytes(r)
