# SCE-MAP: codec_transport_envelope:68 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, SceCursor, SceSink
from .codec_zenoh_init_body import CodecZenohInitBody
from .codec_zenoh_open_body import CodecZenohOpenBody
from .codec_zenoh_close import CodecZenohClose
from .codec_zenoh_keep_alive import CodecZenohKeepAlive
from .codec_zenoh_frame import CodecZenohFrame
from .codec_zenoh_fragment import CodecZenohFragment
from .codec_zenoh_join import CodecZenohJoin

from dataclasses import dataclass, field
from typing import Optional


@dataclass
class CodecTransportEnvelopeVariant:
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
    kind: str = "CodecZenohClose"
    codec_zenoh_init_body: Optional[CodecZenohInitBody] = None
    codec_zenoh_open_body: Optional[CodecZenohOpenBody] = None
    codec_zenoh_close: Optional[CodecZenohClose] = field(default_factory=CodecZenohClose)
    codec_zenoh_keep_alive: Optional[CodecZenohKeepAlive] = None
    codec_zenoh_frame: Optional[CodecZenohFrame] = None
    codec_zenoh_fragment: Optional[CodecZenohFragment] = None
    codec_zenoh_join: Optional[CodecZenohJoin] = None
    default_body: Optional[CodecZenohClose] = None
    default_tag: int = 0


@dataclass
class CodecTransportEnvelope:
    header: int = 0
    body: CodecTransportEnvelopeVariant = field(default_factory=CodecTransportEnvelopeVariant)

    @classmethod
    def decode(cls, cursor: SceCursor) -> CodecTransportEnvelope:
        """Decode the next frame from ``cursor``; on success the cursor
        advances past the consumed bytes. A frame the codec refuses raises
        the ``CodecError`` subclass that names the refusal (docs/adr/0099,
        amendment of 2026-11-16): ``NeedMoreBytes`` when the cursor's tail
        is shorter than the frame (RFC §synth-5-B L494-519), which the
        caller resumes after appending bytes, and for a frame no more bytes
        make whole ``VleWidthOverflow``, ``InvalidUtf8``,
        ``UndeclaredEnumValue``, or a length or a count past its bound
        (``LengthOutOfRange``, ``TooManyElements``, docs/adr/0077)."""
        # Decode fixed prefix (RFC §synth-5-B variant: fields before tag suffix).
        raw = cursor.peek_slice(1)
        f_header = raw[0]
        cursor.advance(1)
        # Dispatch on the tag field; each arm decodes its body codec
        # from the cursor. The default arm (when declared) carries the
        # runtime tag value so encode can round-trip it back onto the
        # wire.
        body = CodecTransportEnvelopeVariant()
        if ((f_header >> 0) & 0x1F) == 1:
            body.kind = "CodecZenohInitBody"
            body.codec_zenoh_init_body = CodecZenohInitBody.decode(cursor, ((f_header >> 6) & 0x1), ((f_header >> 5) & 0x1))
        elif ((f_header >> 0) & 0x1F) == 2:
            body.kind = "CodecZenohOpenBody"
            body.codec_zenoh_open_body = CodecZenohOpenBody.decode(cursor, ((f_header >> 5) & 0x1))
        elif ((f_header >> 0) & 0x1F) == 3:
            body.kind = "CodecZenohClose"
            body.codec_zenoh_close = CodecZenohClose.decode(cursor)
        elif ((f_header >> 0) & 0x1F) == 4:
            body.kind = "CodecZenohKeepAlive"
            body.codec_zenoh_keep_alive = CodecZenohKeepAlive.decode(cursor)
        elif ((f_header >> 0) & 0x1F) == 5:
            body.kind = "CodecZenohFrame"
            body.codec_zenoh_frame = CodecZenohFrame.decode(cursor)
        elif ((f_header >> 0) & 0x1F) == 6:
            body.kind = "CodecZenohFragment"
            body.codec_zenoh_fragment = CodecZenohFragment.decode(cursor)
        elif ((f_header >> 0) & 0x1F) == 7:
            body.kind = "CodecZenohJoin"
            body.codec_zenoh_join = CodecZenohJoin.decode(cursor, ((f_header >> 6) & 0x1))
        else:
            body.kind = "Default"
            body.default_tag = ((f_header >> 0) & 0x1F)
            body.default_body = CodecZenohClose.decode(cursor)
        return cls(
            header=f_header,
            body=body,
        )

    # RFC §synth-5-B flags primitive: per-bit-range accessors over
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

    def a(self) -> bool:
        return (self.header & 0x20) != 0

    def set_a(self, v: bool) -> None:
        if v:
            self.header = (self.header | 0x20) & 0xFF
        else:
            self.header = self.header & (0xFF ^ 0x20)

    def s(self) -> bool:
        return (self.header & 0x40) != 0

    def set_s(self, v: bool) -> None:
        if v:
            self.header = (self.header | 0x40) & 0xFF
        else:
            self.header = self.header & (0xFF ^ 0x40)

    def z(self) -> bool:
        return (self.header & 0x80) != 0

    def set_z(self, v: bool) -> None:
        if v:
            self.header = (self.header | 0x80) & 0xFF
        else:
            self.header = self.header & (0xFF ^ 0x80)

    def encode(self, w: SceSink) -> None:
        """RFC §synth-5-B encode-side primary: write ``self`` into the
        caller-owned ``w`` sink. Returns ``None`` on success; raises
        :class:`BufferOverflow` from a bounded sink when the destination
        has insufficient remaining capacity; growable sinks (e.g.
        :class:`BytearraySink`) are effectively infallible."""
        # Encode fixed prefix (tag field bytes are part of the prefix).
        w.write_u8(self.header & 0xFF)
        # Append the active arm body's encoded bytes via the same sink.
        if self.body.kind == "CodecZenohInitBody":
            self.body.codec_zenoh_init_body.encode(w, ((self.header >> 6) & 0x1), ((self.header >> 5) & 0x1))
        elif self.body.kind == "CodecZenohOpenBody":
            self.body.codec_zenoh_open_body.encode(w, ((self.header >> 5) & 0x1))
        elif self.body.kind == "CodecZenohClose":
            self.body.codec_zenoh_close.encode(w)
        elif self.body.kind == "CodecZenohKeepAlive":
            self.body.codec_zenoh_keep_alive.encode(w)
        elif self.body.kind == "CodecZenohFrame":
            self.body.codec_zenoh_frame.encode(w)
        elif self.body.kind == "CodecZenohFragment":
            self.body.codec_zenoh_fragment.encode(w)
        elif self.body.kind == "CodecZenohJoin":
            self.body.codec_zenoh_join.encode(w, ((self.header >> 6) & 0x1))
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
