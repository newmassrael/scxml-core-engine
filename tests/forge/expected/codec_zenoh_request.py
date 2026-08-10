# SCE-MAP: codec_zenoh_request:73 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, SceCursor, SceSink, TlvChainOverflow
from .codec_zenoh_wireexpr import CodecZenohWireexpr
from .codec_zenoh_ext_entry import CodecZenohExtEntry
from .codec_zenoh_msg_put import CodecZenohMsgPut
from .codec_zenoh_msg_del import CodecZenohMsgDel
from .codec_zenoh_query import CodecZenohQuery

from dataclasses import dataclass, field
from typing import Optional, List


@dataclass
class CodecZenohRequestVariant:
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
        (RFC §synth-5-B L494-519); on success the cursor advances past the
        consumed bytes. VLE codecs also return ``None`` on
        ``VleWidthOverflow``."""
        # RFC §synth-5-B peek-byte / streaming-prefix:
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
                _more = False
                for _ in range(4):
                    if cursor.remaining() == 0:
                        break
                    _elem = CodecZenohExtEntry.decode(cursor)
                    if _elem is None:
                        return None
                    _more = _elem.z()
                    extensions.append(_elem)
                    if not _more:
                        break
                if _more and cursor.remaining() == 0:
                    raise NeedMoreBytes()
                if _more:
                    raise TlvChainOverflow()
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
        # RFC §synth-5-B peek-byte / streaming-prefix:
        # streaming prefix encode.
        w.write_u8(self.header & 0xFF)
        w.write_vle_u64(self.rid)
        self.keyexpr.encode(w, ((self.header >> 5) & 0x1))
        if self.extensions is not None:
            for _e in self.extensions:
                _e.encode(w)
        # Append the active arm body's encoded bytes via the same sink.
        if self.body.kind == "CodecZenohMsgPut":
            self.body.codec_zenoh_msg_put.encode(w)
        elif self.body.kind == "CodecZenohMsgDel":
            self.body.codec_zenoh_msg_del.encode(w)
        elif self.body.kind == "CodecZenohQuery":
            self.body.codec_zenoh_query.encode(w)
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
