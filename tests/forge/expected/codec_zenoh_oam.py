# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import CodecError, NeedMoreBytes, SceCursor, TlvChainOverflow
from .codec_zenoh_ext_entry import CodecZenohExtEntry
from .codec_zenoh_ext_unit import CodecZenohExtUnit
from .codec_zenoh_ext_zint import CodecZenohExtZint
from .codec_zenoh_ext_zbuf import CodecZenohExtZbuf

from dataclasses import dataclass, field
from typing import Optional, List


@dataclass
class CodecZenohOamVariant:
    """RFC §5.B variant primitive (B1-β): discriminated-union body for
    the codec's tag-field suffix. ``kind`` selects the active arm; the
    matching ``Optional`` field carries the decoded body. ``default_tag``
    preserves the runtime tag value when the default arm fires so encode
    can round-trip it back onto the wire."""
    # Default to the first declared arm (or "Default" when arms is empty)
    # so a freshly-constructed envelope round-trips through encode without
    # needing the caller to populate the body explicitly.
    kind: str = "CodecZenohExtUnit"
    codec_zenoh_ext_unit: Optional[CodecZenohExtUnit] = None
    codec_zenoh_ext_zint: Optional[CodecZenohExtZint] = None
    codec_zenoh_ext_zbuf: Optional[CodecZenohExtZbuf] = None
    default_body: Optional[CodecZenohExtUnit] = None
    default_tag: int = 0


@dataclass
class CodecZenohOam:
    header: int = 0
    id: int = 0
    extensions: Optional[List[CodecZenohExtEntry]] = b""
    body: CodecZenohOamVariant = field(default_factory=CodecZenohOamVariant)

    @classmethod
    def decode(cls, cursor: SceCursor) -> Optional[CodecZenohOam]:
        """Decode the next frame from ``cursor``. Returns ``None`` when
        the cursor's tail is shorter than the declared minimum frame
        (RFC §5.B L494-519); on success the cursor advances past the
        consumed bytes. VLE codecs also return ``None`` on
        ``VleWidthOverflow``."""
        # RFC §5.B Y3 atomic 2b-ii peek-byte / 2b-iv streaming-prefix:
        # streaming prefix decode (variable-length fields supported via
        # per-field present_if/tlv-chain/embed/repeat helpers). Peek-byte
        # mode additionally peeks the cursor's next byte for variant tag
        # without advancing — arm body decoder reads it as own header.
        try:
            raw = cursor.peek_slice(1)
            header = raw[0]
            cursor.advance(1)
            id = cursor.read_vle_u16()
            if (header & 0x80) != 0:
                extensions = []
                for _ in range(4):
                    if cursor.remaining() == 0:
                        break
                    _elem = CodecZenohExtEntry.decode(cursor)
                    if _elem is None:
                        return None
                    extensions.append(_elem)
                    if not _elem.z():
                        break
            else:
                extensions = None
        except NeedMoreBytes:
            return None
        # Dispatch on the tag field; each arm decodes its body codec
        # from the cursor. The default arm (when declared) carries the
        # runtime tag value so encode can round-trip it back onto the
        # wire.
        body = CodecZenohOamVariant()
        if ((header >> 5) & 0x03) == 0:
            body.kind = "CodecZenohExtUnit"
            _arm = CodecZenohExtUnit.decode(cursor)
            if _arm is None:
                return None
            body.codec_zenoh_ext_unit = _arm
        elif ((header >> 5) & 0x03) == 1:
            body.kind = "CodecZenohExtZint"
            _arm = CodecZenohExtZint.decode(cursor)
            if _arm is None:
                return None
            body.codec_zenoh_ext_zint = _arm
        elif ((header >> 5) & 0x03) == 2:
            body.kind = "CodecZenohExtZbuf"
            _arm = CodecZenohExtZbuf.decode(cursor)
            if _arm is None:
                return None
            body.codec_zenoh_ext_zbuf = _arm
        else:
            body.kind = "Default"
            body.default_tag = ((header >> 5) & 0x03)
            _arm = CodecZenohExtUnit.decode(cursor)
            if _arm is None:
                return None
            body.default_body = _arm
        return cls(
            header=header,
            id=id,
            extensions=extensions,
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

    def enc(self) -> int:
        return (self.header >> 5) & 0x03

    def set_enc(self, v: int) -> None:
        _shifted_mask = 0x03 << 5
        _val = (v & 0x03) << 5
        self.header = ((self.header & (0xFF ^ _shifted_mask)) | _val) & 0xFF

    def z(self) -> bool:
        return (self.header & 0x80) != 0

    def set_z(self, v: bool) -> None:
        if v:
            self.header = (self.header | 0x80) & 0xFF
        else:
            self.header = self.header & (0xFF ^ 0x80)

    def encode(self) -> bytes:
        # RFC §5.B Y3 atomic 2b-ii peek-byte / 2b-iv streaming-prefix:
        # streaming prefix encode. Peek-byte mode: arm body's encode
        # prepends its own header byte (which the decoder peeked); no
        # separate tag byte here. Streaming-prefix mode (own-field):
        # carrier is part of the prefix fields and emits via the same
        # per-field path.
        r = bytearray()
        r.append(self.header & 0xFF)
        _w = int(self.id)
        while _w >= 0x80:
            r.append((_w & 0x7F) | 0x80)
            _w >>= 7
        r.append(_w)
        if self.extensions is not None:
            for _e in self.extensions:
                r.extend(_e.encode())
        # Append the active arm body's encoded bytes.
        if self.body.kind == "CodecZenohExtUnit":
            r.extend(self.body.codec_zenoh_ext_unit.encode())
        elif self.body.kind == "CodecZenohExtZint":
            r.extend(self.body.codec_zenoh_ext_zint.encode())
        elif self.body.kind == "CodecZenohExtZbuf":
            r.extend(self.body.codec_zenoh_ext_zbuf.encode())
        elif self.body.kind == "Default":
            r.extend(self.body.default_body.encode())
        return bytes(r)
