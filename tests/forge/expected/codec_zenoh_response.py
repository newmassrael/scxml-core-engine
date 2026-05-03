# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import CodecError, InvalidUtf8, NeedMoreBytes, SceCursor, TlvChainOverflow
from .codec_zenoh_ext_entry import CodecZenohExtEntry
from .codec_zenoh_msg_reply import CodecZenohMsgReply
from .codec_zenoh_msg_err import CodecZenohMsgErr

from dataclasses import dataclass, field
from typing import Optional, List


@dataclass
class CodecZenohResponseVariant:
    """RFC §5.B variant primitive (B1-β): discriminated-union body for
    the codec's tag-field suffix. ``kind`` selects the active arm; the
    matching ``Optional`` field carries the decoded body. ``default_tag``
    preserves the runtime tag value when the default arm fires so encode
    can round-trip it back onto the wire."""
    # Default to the first declared arm (or "Default" when arms is empty)
    # so a freshly-constructed envelope round-trips through encode without
    # needing the caller to populate the body explicitly.
    kind: str = "CodecZenohMsgReply"
    codec_zenoh_msg_reply: Optional[CodecZenohMsgReply] = None
    codec_zenoh_msg_err: Optional[CodecZenohMsgErr] = None
    default_body: Optional[CodecZenohMsgReply] = None
    default_tag: int = 0


@dataclass
class CodecZenohResponse:
    header: int = 0
    request_id: int = 0
    key_id: int = 0
    suffix_len: Optional[int] = None
    suffix: Optional[str] = None
    extensions: Optional[List[CodecZenohExtEntry]] = b""
    body: CodecZenohResponseVariant = field(default_factory=CodecZenohResponseVariant)

    @classmethod
    def decode(cls, cursor: SceCursor) -> Optional[CodecZenohResponse]:
        """Decode the next frame from ``cursor``. Returns ``None`` when
        the cursor's tail is shorter than the declared minimum frame
        (RFC §5.B L494-519); on success the cursor advances past the
        consumed bytes. VLE codecs also return ``None`` on
        ``VleWidthOverflow``."""
        # RFC §5.B Y3 atomic 2b-ii peek-byte — peek-byte mode: streaming
        # prefix decode (variable-length supported), then peek the
        # cursor's next byte for variant tag without advancing. Arm
        # body decoder reads peeked byte as own header.
        try:
            raw = cursor.peek_slice(1)
            header = raw[0]
            cursor.advance(1)
            request_id = cursor.read_vle_u64()
            key_id = cursor.read_vle_u32()
            if (header & 0x40) != 0:
                _v = cursor.read_vle_u64()
                suffix_len = _v
            else:
                suffix_len = None
            if (header & 0x40) != 0:
                _n = suffix_len
                raw = cursor.peek_slice(_n)
                try:
                    _v = bytes(raw).decode('utf-8')
                except UnicodeDecodeError as exc:
                    raise InvalidUtf8() from exc
                cursor.advance(_n)
                suffix = _v
            else:
                suffix = None
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
        body = CodecZenohResponseVariant()
        if ((_peek >> 0) & 0x1F) == 4:
            body.kind = "CodecZenohMsgReply"
            _arm = CodecZenohMsgReply.decode(cursor)
            if _arm is None:
                return None
            body.codec_zenoh_msg_reply = _arm
        elif ((_peek >> 0) & 0x1F) == 5:
            body.kind = "CodecZenohMsgErr"
            _arm = CodecZenohMsgErr.decode(cursor)
            if _arm is None:
                return None
            body.codec_zenoh_msg_err = _arm
        else:
            body.kind = "Default"
            body.default_tag = ((_peek >> 0) & 0x1F)
            _arm = CodecZenohMsgReply.decode(cursor)
            if _arm is None:
                return None
            body.default_body = _arm
        return cls(
            header=header,
            request_id=request_id,
            key_id=key_id,
            suffix_len=suffix_len,
            suffix=suffix,
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

    def m(self) -> bool:
        return (self.header & 0x20) != 0

    def set_m(self, v: bool) -> None:
        if v:
            self.header = (self.header | 0x20) & 0xFF
        else:
            self.header = self.header & (0xFF ^ 0x20)

    def n(self) -> bool:
        return (self.header & 0x40) != 0

    def set_n(self, v: bool) -> None:
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
        # RFC §5.B Y3 atomic 2b-ii peek-byte — peek-byte mode: streaming
        # prefix encode. Arm body's encode prepends its own header byte
        # (which the decoder peeked); no separate tag byte here.
        r = bytearray()
        r.append(self.header & 0xFF)
        _w = int(self.request_id)
        while _w >= 0x80:
            r.append((_w & 0x7F) | 0x80)
            _w >>= 7
        r.append(_w)
        _w = int(self.key_id)
        while _w >= 0x80:
            r.append((_w & 0x7F) | 0x80)
            _w >>= 7
        r.append(_w)
        if self.suffix_len is not None:
            _w = int(self.suffix_len)
            while _w >= 0x80:
                r.append((_w & 0x7F) | 0x80)
                _w >>= 7
            r.append(_w)
        if self.suffix is not None:
            r.extend(self.suffix.encode('utf-8'))
        if self.extensions is not None:
            for _e in self.extensions:
                r.extend(_e.encode())
        # Append the active arm body's encoded bytes.
        if self.body.kind == "CodecZenohMsgReply":
            r.extend(self.body.codec_zenoh_msg_reply.encode())
        elif self.body.kind == "CodecZenohMsgErr":
            r.extend(self.body.codec_zenoh_msg_err.encode())
        elif self.body.kind == "Default":
            r.extend(self.body.default_body.encode())
        return bytes(r)
