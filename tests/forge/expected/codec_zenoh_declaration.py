# SCE-MAP: codec_zenoh_declaration:54

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, SceCursor, SceSink
from .codec_zenoh_decl_kexpr import CodecZenohDeclKexpr
from .codec_zenoh_undecl_kexpr import CodecZenohUndeclKexpr
from .codec_zenoh_decl_subscriber import CodecZenohDeclSubscriber
from .codec_zenoh_undecl_subscriber import CodecZenohUndeclSubscriber
from .codec_zenoh_decl_queryable import CodecZenohDeclQueryable
from .codec_zenoh_undecl_queryable import CodecZenohUndeclQueryable
from .codec_zenoh_decl_token import CodecZenohDeclToken
from .codec_zenoh_undecl_token import CodecZenohUndeclToken
from .codec_zenoh_decl_final import CodecZenohDeclFinal

from dataclasses import dataclass, field
from typing import Optional


@dataclass
class CodecZenohDeclarationVariant:
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
    kind: str = "CodecZenohDeclFinal"
    codec_zenoh_decl_kexpr: Optional[CodecZenohDeclKexpr] = None
    codec_zenoh_undecl_kexpr: Optional[CodecZenohUndeclKexpr] = None
    codec_zenoh_decl_subscriber: Optional[CodecZenohDeclSubscriber] = None
    codec_zenoh_undecl_subscriber: Optional[CodecZenohUndeclSubscriber] = None
    codec_zenoh_decl_queryable: Optional[CodecZenohDeclQueryable] = None
    codec_zenoh_undecl_queryable: Optional[CodecZenohUndeclQueryable] = None
    codec_zenoh_decl_token: Optional[CodecZenohDeclToken] = None
    codec_zenoh_undecl_token: Optional[CodecZenohUndeclToken] = None
    codec_zenoh_decl_final: Optional[CodecZenohDeclFinal] = field(default_factory=CodecZenohDeclFinal)
    default_body: Optional[CodecZenohDeclFinal] = None
    default_tag: int = 0


@dataclass
class CodecZenohDeclaration:
    header: int = 0
    body: CodecZenohDeclarationVariant = field(default_factory=CodecZenohDeclarationVariant)

    @classmethod
    def decode(cls, cursor: SceCursor) -> Optional[CodecZenohDeclaration]:
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
        header = raw[0]
        try:
            cursor.advance(1)
        except NeedMoreBytes:
            return None
        # Dispatch on the tag field; each arm decodes its body codec
        # from the cursor. The default arm (when declared) carries the
        # runtime tag value so encode can round-trip it back onto the
        # wire.
        body = CodecZenohDeclarationVariant()
        if ((header >> 0) & 0x1F) == 0:
            body.kind = "CodecZenohDeclKexpr"
            _arm = CodecZenohDeclKexpr.decode(cursor, ((header >> 5) & 0x1))
            if _arm is None:
                return None
            body.codec_zenoh_decl_kexpr = _arm
        elif ((header >> 0) & 0x1F) == 1:
            body.kind = "CodecZenohUndeclKexpr"
            _arm = CodecZenohUndeclKexpr.decode(cursor)
            if _arm is None:
                return None
            body.codec_zenoh_undecl_kexpr = _arm
        elif ((header >> 0) & 0x1F) == 2:
            body.kind = "CodecZenohDeclSubscriber"
            _arm = CodecZenohDeclSubscriber.decode(cursor, ((header >> 5) & 0x1))
            if _arm is None:
                return None
            body.codec_zenoh_decl_subscriber = _arm
        elif ((header >> 0) & 0x1F) == 3:
            body.kind = "CodecZenohUndeclSubscriber"
            _arm = CodecZenohUndeclSubscriber.decode(cursor, ((header >> 7) & 0x1))
            if _arm is None:
                return None
            body.codec_zenoh_undecl_subscriber = _arm
        elif ((header >> 0) & 0x1F) == 4:
            body.kind = "CodecZenohDeclQueryable"
            _arm = CodecZenohDeclQueryable.decode(cursor, ((header >> 5) & 0x1), ((header >> 7) & 0x1))
            if _arm is None:
                return None
            body.codec_zenoh_decl_queryable = _arm
        elif ((header >> 0) & 0x1F) == 5:
            body.kind = "CodecZenohUndeclQueryable"
            _arm = CodecZenohUndeclQueryable.decode(cursor, ((header >> 7) & 0x1))
            if _arm is None:
                return None
            body.codec_zenoh_undecl_queryable = _arm
        elif ((header >> 0) & 0x1F) == 6:
            body.kind = "CodecZenohDeclToken"
            _arm = CodecZenohDeclToken.decode(cursor, ((header >> 5) & 0x1))
            if _arm is None:
                return None
            body.codec_zenoh_decl_token = _arm
        elif ((header >> 0) & 0x1F) == 7:
            body.kind = "CodecZenohUndeclToken"
            _arm = CodecZenohUndeclToken.decode(cursor, ((header >> 7) & 0x1))
            if _arm is None:
                return None
            body.codec_zenoh_undecl_token = _arm
        elif ((header >> 0) & 0x1F) == 26:
            body.kind = "CodecZenohDeclFinal"
            _arm = CodecZenohDeclFinal.decode(cursor)
            if _arm is None:
                return None
            body.codec_zenoh_decl_final = _arm
        else:
            body.kind = "Default"
            body.default_tag = ((header >> 0) & 0x1F)
            _arm = CodecZenohDeclFinal.decode(cursor)
            if _arm is None:
                return None
            body.default_body = _arm
        return cls(
            header=header,
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

    def n(self) -> bool:
        return (self.header & 0x20) != 0

    def set_n(self, v: bool) -> None:
        if v:
            self.header = (self.header | 0x20) & 0xFF
        else:
            self.header = self.header & (0xFF ^ 0x20)

    def m(self) -> bool:
        return (self.header & 0x40) != 0

    def set_m(self, v: bool) -> None:
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
        if self.body.kind == "CodecZenohDeclKexpr":
            self.body.codec_zenoh_decl_kexpr.encode(w, ((self.header >> 5) & 0x1))
        elif self.body.kind == "CodecZenohUndeclKexpr":
            self.body.codec_zenoh_undecl_kexpr.encode(w)
        elif self.body.kind == "CodecZenohDeclSubscriber":
            self.body.codec_zenoh_decl_subscriber.encode(w, ((self.header >> 5) & 0x1))
        elif self.body.kind == "CodecZenohUndeclSubscriber":
            self.body.codec_zenoh_undecl_subscriber.encode(w, ((self.header >> 7) & 0x1))
        elif self.body.kind == "CodecZenohDeclQueryable":
            self.body.codec_zenoh_decl_queryable.encode(w, ((self.header >> 5) & 0x1), ((self.header >> 7) & 0x1))
        elif self.body.kind == "CodecZenohUndeclQueryable":
            self.body.codec_zenoh_undecl_queryable.encode(w, ((self.header >> 7) & 0x1))
        elif self.body.kind == "CodecZenohDeclToken":
            self.body.codec_zenoh_decl_token.encode(w, ((self.header >> 5) & 0x1))
        elif self.body.kind == "CodecZenohUndeclToken":
            self.body.codec_zenoh_undecl_token.encode(w, ((self.header >> 7) & 0x1))
        elif self.body.kind == "CodecZenohDeclFinal":
            self.body.codec_zenoh_decl_final.encode(w)
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
