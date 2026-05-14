# SCE-MAP: codec_zenoh_declaration:54

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import CodecError, NeedMoreBytes, SceCursor
from .codec_zenoh_decl_keyexpr import CodecZenohDeclKeyexpr
from .codec_zenoh_undecl_keyexpr import CodecZenohUndeclKeyexpr
from .codec_zenoh_decl_subscriber import CodecZenohDeclSubscriber
from .codec_zenoh_undecl_subscriber import CodecZenohUndeclSubscriber
from .codec_zenoh_decl_queryable import CodecZenohDeclQueryable
from .codec_zenoh_undecl_queryable import CodecZenohUndeclQueryable
from .codec_zenoh_decl_token import CodecZenohDeclToken
from .codec_zenoh_undecl_token import CodecZenohUndeclToken
from .codec_decl_final import CodecDeclFinal

from dataclasses import dataclass, field
from typing import Optional


@dataclass
class CodecZenohDeclarationVariant:
    """RFC §5.B variant primitive (B1-β): discriminated-union body for
    the codec's tag-field suffix. ``kind`` selects the active arm; the
    matching ``Optional`` field carries the decoded body. ``default_tag``
    preserves the runtime tag value when the default arm fires so encode
    can round-trip it back onto the wire."""
    # Default to the first declared arm (or "Default" when arms is empty)
    # so a freshly-constructed envelope round-trips through encode without
    # needing the caller to populate the body explicitly.
    kind: str = "CodecZenohDeclKeyexpr"
    codec_zenoh_decl_keyexpr: Optional[CodecZenohDeclKeyexpr] = None
    codec_zenoh_undecl_keyexpr: Optional[CodecZenohUndeclKeyexpr] = None
    codec_zenoh_decl_subscriber: Optional[CodecZenohDeclSubscriber] = None
    codec_zenoh_undecl_subscriber: Optional[CodecZenohUndeclSubscriber] = None
    codec_zenoh_decl_queryable: Optional[CodecZenohDeclQueryable] = None
    codec_zenoh_undecl_queryable: Optional[CodecZenohUndeclQueryable] = None
    codec_zenoh_decl_token: Optional[CodecZenohDeclToken] = None
    codec_zenoh_undecl_token: Optional[CodecZenohUndeclToken] = None
    codec_decl_final: Optional[CodecDeclFinal] = None
    default_body: Optional[CodecDeclFinal] = None
    default_tag: int = 0


@dataclass
class CodecZenohDeclaration:
    header: int = 0
    body: CodecZenohDeclarationVariant = field(default_factory=CodecZenohDeclarationVariant)

    @classmethod
    def decode(cls, cursor: SceCursor) -> Optional[CodecZenohDeclaration]:
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
        body = CodecZenohDeclarationVariant()
        if ((header >> 0) & 0x1F) == 0:
            body.kind = "CodecZenohDeclKeyexpr"
            _arm = CodecZenohDeclKeyexpr.decode(cursor, header)
            if _arm is None:
                return None
            body.codec_zenoh_decl_keyexpr = _arm
        elif ((header >> 0) & 0x1F) == 1:
            body.kind = "CodecZenohUndeclKeyexpr"
            _arm = CodecZenohUndeclKeyexpr.decode(cursor)
            if _arm is None:
                return None
            body.codec_zenoh_undecl_keyexpr = _arm
        elif ((header >> 0) & 0x1F) == 2:
            body.kind = "CodecZenohDeclSubscriber"
            _arm = CodecZenohDeclSubscriber.decode(cursor, header)
            if _arm is None:
                return None
            body.codec_zenoh_decl_subscriber = _arm
        elif ((header >> 0) & 0x1F) == 3:
            body.kind = "CodecZenohUndeclSubscriber"
            _arm = CodecZenohUndeclSubscriber.decode(cursor, header)
            if _arm is None:
                return None
            body.codec_zenoh_undecl_subscriber = _arm
        elif ((header >> 0) & 0x1F) == 4:
            body.kind = "CodecZenohDeclQueryable"
            _arm = CodecZenohDeclQueryable.decode(cursor, header)
            if _arm is None:
                return None
            body.codec_zenoh_decl_queryable = _arm
        elif ((header >> 0) & 0x1F) == 5:
            body.kind = "CodecZenohUndeclQueryable"
            _arm = CodecZenohUndeclQueryable.decode(cursor, header)
            if _arm is None:
                return None
            body.codec_zenoh_undecl_queryable = _arm
        elif ((header >> 0) & 0x1F) == 6:
            body.kind = "CodecZenohDeclToken"
            _arm = CodecZenohDeclToken.decode(cursor, header)
            if _arm is None:
                return None
            body.codec_zenoh_decl_token = _arm
        elif ((header >> 0) & 0x1F) == 7:
            body.kind = "CodecZenohUndeclToken"
            _arm = CodecZenohUndeclToken.decode(cursor, header)
            if _arm is None:
                return None
            body.codec_zenoh_undecl_token = _arm
        elif ((header >> 0) & 0x1F) == 26:
            body.kind = "CodecDeclFinal"
            _arm = CodecDeclFinal.decode(cursor)
            if _arm is None:
                return None
            body.codec_decl_final = _arm
        else:
            body.kind = "Default"
            body.default_tag = ((header >> 0) & 0x1F)
            _arm = CodecDeclFinal.decode(cursor)
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

    def encode(self) -> bytes:
        # Encode fixed prefix (tag field bytes are part of the prefix).
        # The tag value is read from the struct field, NOT derived from
        # the body discriminant — keeping author-set tag / body in sync
        # is the caller's responsibility (v1 keeps the layout simple).
        r = bytearray()
        r.append(self.header & 0xFF)
        # Append the active arm body's encoded bytes.
        if self.body.kind == "CodecZenohDeclKeyexpr":
            r.extend(self.body.codec_zenoh_decl_keyexpr.encode(self.header))
        elif self.body.kind == "CodecZenohUndeclKeyexpr":
            r.extend(self.body.codec_zenoh_undecl_keyexpr.encode())
        elif self.body.kind == "CodecZenohDeclSubscriber":
            r.extend(self.body.codec_zenoh_decl_subscriber.encode(self.header))
        elif self.body.kind == "CodecZenohUndeclSubscriber":
            r.extend(self.body.codec_zenoh_undecl_subscriber.encode(self.header))
        elif self.body.kind == "CodecZenohDeclQueryable":
            r.extend(self.body.codec_zenoh_decl_queryable.encode(self.header))
        elif self.body.kind == "CodecZenohUndeclQueryable":
            r.extend(self.body.codec_zenoh_undecl_queryable.encode(self.header))
        elif self.body.kind == "CodecZenohDeclToken":
            r.extend(self.body.codec_zenoh_decl_token.encode(self.header))
        elif self.body.kind == "CodecZenohUndeclToken":
            r.extend(self.body.codec_zenoh_undecl_token.encode(self.header))
        elif self.body.kind == "CodecDeclFinal":
            r.extend(self.body.codec_decl_final.encode())
        elif self.body.kind == "Default":
            r.extend(self.body.default_body.encode())
        return bytes(r)
