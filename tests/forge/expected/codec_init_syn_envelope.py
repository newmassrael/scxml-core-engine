# SCE-MAP: codec_init_syn_envelope:24

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, SceCursor, SceSink
from .codec_init_syn_body import CodecInitSynBody

from dataclasses import dataclass, field
from typing import Optional


@dataclass
class CodecInitSynEnvelopeVariant:
    """RFC §5.B variant primitive: discriminated-union body for
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
    kind: str = "CodecInitSynBody"
    codec_init_syn_body: Optional[CodecInitSynBody] = field(default_factory=CodecInitSynBody)
    default_body: Optional[CodecInitSynBody] = None
    default_tag: int = 0


@dataclass
class CodecInitSynEnvelope:
    header: int = 0
    body: CodecInitSynEnvelopeVariant = field(default_factory=CodecInitSynEnvelopeVariant)

    @classmethod
    def decode(cls, cursor: SceCursor) -> Optional[CodecInitSynEnvelope]:
        """Decode the next frame from ``cursor``. Returns ``None`` when
        the cursor's tail is shorter than the declared minimum frame
        (RFC §5.B L494-519); on success the cursor advances past the
        consumed bytes. VLE codecs also return ``None`` on
        ``VleWidthOverflow``."""
        # Decode fixed prefix (RFC §5.B variant: fields before tag suffix).
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
        body = CodecInitSynEnvelopeVariant()
        if ((header >> 0) & 0x1F) == 1:
            body.kind = "CodecInitSynBody"
            _arm = CodecInitSynBody.decode(cursor, ((header >> 6) & 0x1))
            if _arm is None:
                return None
            body.codec_init_syn_body = _arm
        else:
            body.kind = "Default"
            body.default_tag = ((header >> 0) & 0x1F)
            _arm = CodecInitSynBody.decode(cursor, ((header >> 6) & 0x1))
            if _arm is None:
                return None
            body.default_body = _arm
        return cls(
            header=header,
            body=body,
        )

    # RFC §5.B flags primitive: per-bit-range accessors over
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

    def encode(self, w: SceSink) -> None:
        """RFC §5.B encode-side primary: write ``self`` into the
        caller-owned ``w`` sink. Returns ``None`` on success; raises
        :class:`BufferOverflow` from a bounded sink when the destination
        has insufficient remaining capacity; growable sinks (e.g.
        :class:`BytearraySink`) are effectively infallible."""
        # Encode fixed prefix (tag field bytes are part of the prefix).
        w.write_u8(self.header & 0xFF)
        # Append the active arm body's encoded bytes via the same sink.
        if self.body.kind == "CodecInitSynBody":
            self.body.codec_init_syn_body.encode(w, ((self.header >> 6) & 0x1))
        elif self.body.kind == "Default":
            self.body.default_body.encode(w, ((self.header >> 6) & 0x1))

    def encode_to_bytes(self) -> bytes:
        """Heap-backed convenience facade. Runs :meth:`encode` over a
        :class:`BytearraySink` and returns the freshly-encoded bytes.
        Callers targeting zero-alloc hot paths should call :meth:`encode`
        directly against a caller-owned sink."""
        _dst = bytearray()
        self.encode(BytearraySink(_dst))
        return bytes(_dst)
