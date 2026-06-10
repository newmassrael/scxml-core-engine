# SCE-MAP: codec_zenoh_oam:56

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, SceCursor, SceSink, TlvChainOverflow
from .codec_zenoh_ext_entry import CodecZenohExtEntry
from .codec_zenoh_ext_unit import CodecZenohExtUnit
from .codec_zenoh_ext_zint import CodecZenohExtZint
from .codec_zenoh_ext_zbuf import CodecZenohExtZbuf

from dataclasses import dataclass, field
from typing import Optional, List


@dataclass
class CodecZenohOamVariant:
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
    kind: str = "CodecZenohExtUnit"
    codec_zenoh_ext_unit: Optional[CodecZenohExtUnit] = field(default_factory=CodecZenohExtUnit)
    codec_zenoh_ext_zint: Optional[CodecZenohExtZint] = None
    codec_zenoh_ext_zbuf: Optional[CodecZenohExtZbuf] = None
    default_body: Optional[CodecZenohExtUnit] = None
    default_tag: int = 0


@dataclass
class CodecZenohOam:
    header: int = 0x1f
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

    def encode(self, w: SceSink) -> None:
        """RFC §5.B encode-side primary: write ``self`` into the
        caller-owned ``w`` sink. Returns ``None`` on success; raises
        :class:`BufferOverflow` from a bounded sink when the destination
        has insufficient remaining capacity; growable sinks (e.g.
        :class:`BytearraySink`) are effectively infallible."""
        # RFC §5.B Y3 atomic 2b-ii peek-byte / 2b-iv streaming-prefix:
        # streaming prefix encode.
        w.write_u8(self.header & 0xFF)
        _vle = int(self.id)
        while _vle >= 0x80:
            w.write_u8((_vle & 0x7F) | 0x80)
            _vle >>= 7
        w.write_u8(_vle)
        if self.extensions is not None:
            for _e in self.extensions:
                _e.encode(w)
        # Append the active arm body's encoded bytes via the same sink.
        if self.body.kind == "CodecZenohExtUnit":
            self.body.codec_zenoh_ext_unit.encode(w)
        elif self.body.kind == "CodecZenohExtZint":
            self.body.codec_zenoh_ext_zint.encode(w)
        elif self.body.kind == "CodecZenohExtZbuf":
            self.body.codec_zenoh_ext_zbuf.encode(w)
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
