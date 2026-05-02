# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import CodecError, NeedMoreBytes, SceCursor
from .codec_init_syn_body import CodecInitSynBody

from dataclasses import dataclass, field
from typing import Optional


@dataclass
class CodecInitSynEnvelopeBody:
    """RFC §5.B variant primitive (B1-β): discriminated-union body for
    the codec's tag-field suffix. ``kind`` selects the active arm; the
    matching ``Optional`` field carries the decoded body. ``default_tag``
    preserves the runtime tag value when the default arm fires so encode
    can round-trip it back onto the wire."""
    # Default to the first declared arm (or "Default" when arms is empty)
    # so a freshly-constructed envelope round-trips through encode without
    # needing the caller to populate the body explicitly.
    kind: str = "CodecInitSynBody"
    codec_init_syn_body: Optional[CodecInitSynBody] = None
    default_body: Optional[CodecInitSynBody] = None
    default_tag: int = 0


@dataclass
class CodecInitSynEnvelope:
    header: int = 0
    body: CodecInitSynEnvelopeBody = field(default_factory=CodecInitSynEnvelopeBody)

    @classmethod
    def decode(cls, cursor: SceCursor) -> Optional[CodecInitSynEnvelope]:
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
        header = raw[0]
        try:
            cursor.advance(1)
        except NeedMoreBytes:
            return None
        # Dispatch on the tag field; each arm decodes its body codec
        # from the cursor. The default arm (when declared) carries the
        # runtime tag value so encode can round-trip it back onto the
        # wire.
        body = CodecInitSynEnvelopeBody()
        if ((header >> 0) & 0x1F) == 1:
            body.kind = "CodecInitSynBody"
            _arm = CodecInitSynBody.decode(cursor, header)
            if _arm is None:
                return None
            body.codec_init_syn_body = _arm
        else:
            body.kind = "Default"
            body.default_tag = ((header >> 0) & 0x1F)
            _arm = CodecInitSynBody.decode(cursor, header)
            if _arm is None:
                return None
            body.default_body = _arm
        return cls(
            header=header,
            body=body,
        )

    # RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors over
    # the carrier field. Single-bit (width=1) reads as bool; multi-bit
    # (width>=2) reads as ``int`` (Python ints are unbounded, so a single
    # ``int`` covers every result-type width). Setters mask + shift on
    # the way in so out-of-range callers can't corrupt sibling bits.
    # Plain methods (rather than @property) for API symmetry with
    # Rust / Cpp / Kotlin / Go / C11. Wire layout is unchanged.
    def mid(self) -> int:
        return (self.header >> 0) & 0x1F

    def set_mid(self, v: int) -> None:
        _shifted_mask = 0x1F << 0
        _val = (v & 0x1F) << 0
        self.header = ((self.header & (0xFF ^ _shifted_mask)) | _val) & 0xFF

    def s(self) -> bool:
        return (self.header & 0x40) != 0

    def set_s(self, v: bool) -> None:
        if v:
            self.header = (self.header | 0x40) & 0xFF
        else:
            self.header = self.header & (0xFF ^ 0x40)

    def encode(self) -> bytes:
        # Encode fixed prefix (tag field bytes are part of the prefix).
        # The tag value is read from the struct field, NOT derived from
        # the body discriminant — keeping author-set tag / body in sync
        # is the caller's responsibility (v1 keeps the layout simple).
        r = bytearray()
        r.append(self.header & 0xFF)
        # Append the active arm body's encoded bytes.
        if self.body.kind == "CodecInitSynBody":
            r.extend(self.body.codec_init_syn_body.encode(self.header))
        elif self.body.kind == "Default":
            r.extend(self.body.default_body.encode(self.header))
        return bytes(r)
