# SCE-MAP: codec_zenoh_push:45

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, SceCursor, SceSink
from .codec_zenoh_push_body import CodecZenohPushBody

from dataclasses import dataclass, field
from typing import Optional


@dataclass
class CodecZenohPushVariant:
    """RFC §5.B variant primitive: discriminated-union body for
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
    kind: str = "CodecZenohPushBody"
    codec_zenoh_push_body: Optional[CodecZenohPushBody] = field(default_factory=CodecZenohPushBody)
    default_body: Optional[CodecZenohPushBody] = None
    default_tag: int = 0


@dataclass
class CodecZenohPush:
    header: int = 0x1d
    body: CodecZenohPushVariant = field(default_factory=CodecZenohPushVariant)

    @classmethod
    def decode(cls, cursor: SceCursor) -> Optional[CodecZenohPush]:
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
        body = CodecZenohPushVariant()
        if ((header >> 0) & 0x1F) == 29:
            body.kind = "CodecZenohPushBody"
            _arm = CodecZenohPushBody.decode(cursor)
            if _arm is None:
                return None
            body.codec_zenoh_push_body = _arm
        else:
            body.kind = "Default"
            body.default_tag = ((header >> 0) & 0x1F)
            _arm = CodecZenohPushBody.decode(cursor)
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

    def rest(self) -> int:
        return (self.header >> 5) & 0x07

    def set_rest(self, v: int) -> None:
        _shifted_mask = 0x07 << 5
        _val = (v & 0x07) << 5
        self.header = ((self.header & (0xFF ^ _shifted_mask)) | _val) & 0xFF

    def encode(self, w: SceSink) -> None:
        """RFC §5.B encode-side primary: write ``self`` into the
        caller-owned ``w`` sink. Returns ``None`` on success; raises
        :class:`BufferOverflow` from a bounded sink when the destination
        has insufficient remaining capacity; growable sinks (e.g.
        :class:`BytearraySink`) are effectively infallible."""
        # Encode fixed prefix (tag field bytes are part of the prefix).
        w.write_u8(self.header & 0xFF)
        # Append the active arm body's encoded bytes via the same sink.
        if self.body.kind == "CodecZenohPushBody":
            self.body.codec_zenoh_push_body.encode(w)
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
