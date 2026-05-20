# SCE-MAP: codec_zenoh_request:73

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import CodecError, NeedMoreBytes, SceCursor, TlvChainOverflow
from .codec_zenoh_wireexpr import CodecZenohWireexpr
from .codec_zenoh_ext_entry import CodecZenohExtEntry
from .codec_zenoh_msg_put import CodecZenohMsgPut
from .codec_zenoh_msg_del import CodecZenohMsgDel
from .codec_zenoh_query import CodecZenohQuery

from dataclasses import dataclass, field
from typing import Optional, List


@dataclass
class CodecZenohRequestVariant:
    """RFC §5.B variant primitive (B1-β): discriminated-union body for
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
    kind: str = "CodecZenohMsgPut"
    codec_zenoh_msg_put: Optional[CodecZenohMsgPut] = field(default_factory=CodecZenohMsgPut)
    codec_zenoh_msg_del: Optional[CodecZenohMsgDel] = None
    codec_zenoh_query: Optional[CodecZenohQuery] = None
    default_body: Optional[CodecZenohQuery] = None
    default_tag: int = 0


@dataclass
class CodecZenohRequest:
    header: int = 0x1c
    rid: int = 0
    keyexpr: CodecZenohWireexpr = b""
    extensions: Optional[List[CodecZenohExtEntry]] = b""
    body: CodecZenohRequestVariant = field(default_factory=CodecZenohRequestVariant)

    @classmethod
    def decode(cls, cursor: SceCursor) -> Optional[CodecZenohRequest]:
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
            rid = cursor.read_vle_u64()
            keyexpr = CodecZenohWireexpr.decode(cursor, ((header >> 5) & 0x1))
            if keyexpr is None:
                return None
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
            _peek = cursor.peek_slice(1)[0]
        except NeedMoreBytes:
            return None
        # Dispatch on the tag field; each arm decodes its body codec
        # from the cursor. The default arm (when declared) carries the
        # runtime tag value so encode can round-trip it back onto the
        # wire.
        body = CodecZenohRequestVariant()
        if ((_peek >> 0) & 0x1F) == 1:
            body.kind = "CodecZenohMsgPut"
            _arm = CodecZenohMsgPut.decode(cursor)
            if _arm is None:
                return None
            body.codec_zenoh_msg_put = _arm
        elif ((_peek >> 0) & 0x1F) == 2:
            body.kind = "CodecZenohMsgDel"
            _arm = CodecZenohMsgDel.decode(cursor)
            if _arm is None:
                return None
            body.codec_zenoh_msg_del = _arm
        elif ((_peek >> 0) & 0x1F) == 3:
            body.kind = "CodecZenohQuery"
            _arm = CodecZenohQuery.decode(cursor)
            if _arm is None:
                return None
            body.codec_zenoh_query = _arm
        else:
            body.kind = "Default"
            body.default_tag = ((_peek >> 0) & 0x1F)
            _arm = CodecZenohQuery.decode(cursor)
            if _arm is None:
                return None
            body.default_body = _arm
        return cls(
            header=header,
            rid=rid,
            keyexpr=keyexpr,
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
        # RFC §5.B Y3 atomic 2b-ii peek-byte / 2b-iv streaming-prefix:
        # streaming prefix encode. Peek-byte mode: arm body's encode
        # prepends its own header byte (which the decoder peeked); no
        # separate tag byte here. Streaming-prefix mode (own-field):
        # carrier is part of the prefix fields and emits via the same
        # per-field path.
        r = bytearray()
        r.append(self.header & 0xFF)
        _w = int(self.rid)
        while _w >= 0x80:
            r.append((_w & 0x7F) | 0x80)
            _w >>= 7
        r.append(_w)
        r.extend(self.keyexpr.encode(((self.header >> 5) & 0x1)))
        if self.extensions is not None:
            for _e in self.extensions:
                r.extend(_e.encode())
        # Append the active arm body's encoded bytes.
        if self.body.kind == "CodecZenohMsgPut":
            r.extend(self.body.codec_zenoh_msg_put.encode())
        elif self.body.kind == "CodecZenohMsgDel":
            r.extend(self.body.codec_zenoh_msg_del.encode())
        elif self.body.kind == "CodecZenohQuery":
            r.extend(self.body.codec_zenoh_query.encode())
        elif self.body.kind == "Default":
            r.extend(self.body.default_body.encode())
        return bytes(r)
