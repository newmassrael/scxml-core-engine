# SCE-MAP: codec_zenoh_msg_put:64

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, SceCursor, SceSink, TlvChainOverflow
from .codec_zenoh_timestamp import CodecZenohTimestamp
from .codec_zenoh_encoding import CodecZenohEncoding
from .codec_zenoh_ext_entry import CodecZenohExtEntry

from dataclasses import dataclass, field
from typing import Optional, List


@dataclass
class CodecZenohMsgPut:
    header: int = 0x01
    timestamp: Optional[CodecZenohTimestamp] = None
    encoding: Optional[CodecZenohEncoding] = None
    extensions: Optional[List[CodecZenohExtEntry]] = b""
    payload_len: int = 0
    payload: bytes = b""

    @classmethod
    def decode(cls, cursor: SceCursor) -> Optional[CodecZenohMsgPut]:
        """Decode the next frame from ``cursor``. Returns ``None`` when
        the cursor's tail is shorter than the declared minimum frame
        (RFC §synth-5-B L494-519); on success the cursor advances past the
        consumed bytes. VLE codecs also return ``None`` on
        ``VleWidthOverflow``."""
        # RFC §synth-5-B present-if primitive: streaming decode
        # advances the cursor per field. Per-field statements live
        # inside one outer `try:` block so the first peek/advance
        # failure unwinds to a single `except NeedMoreBytes`. Per-
        # field `is_repeat` routes Repeat fields to the dedicated
        # helper. Branch fires before has_vle_fields so a codec mixing
        # VLE + present-if uses the unified streaming path.
        try:
            raw = cursor.peek_slice(1)
            header = raw[0]
            cursor.advance(1)
            if (header & 0x20) != 0:
                timestamp = CodecZenohTimestamp.decode(cursor)
                if timestamp is None:
                    return None
            else:
                timestamp = None
            if (header & 0x40) != 0:
                encoding = CodecZenohEncoding.decode(cursor)
                if encoding is None:
                    return None
            else:
                encoding = None
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
            payload_len = cursor.read_vle_u64()
            _n = payload_len
            raw = cursor.peek_slice(_n)
            payload = bytes(raw)
            cursor.advance(_n)
        except NeedMoreBytes:
            return None
        return cls(
            header=header,
            timestamp=timestamp,
            encoding=encoding,
            extensions=extensions,
            payload_len=payload_len,
            payload=payload,
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

    def t(self) -> bool:
        return (self.header & 0x20) != 0

    def set_t(self, v: bool) -> None:
        if v:
            self.header = (self.header | 0x20) & 0xFF
        else:
            self.header = self.header & (0xFF ^ 0x20)

    def e(self) -> bool:
        return (self.header & 0x40) != 0

    def set_e(self, v: bool) -> None:
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
        # RFC §synth-5-B present-if encode.
        w.write_u8(self.header & 0xFF)
        if self.timestamp is not None:
            self.timestamp.encode(w)
        if self.encoding is not None:
            self.encoding.encode(w)
        if self.extensions is not None:
            for _e in self.extensions:
                _e.encode(w)
        _vle = int(self.payload_len)
        while _vle >= 0x80:
            w.write_u8((_vle & 0x7F) | 0x80)
            _vle >>= 7
        w.write_u8(_vle)
        w.write_bytes(self.payload)

    def encode_to_bytes(self) -> bytes:
        """Heap-backed convenience facade. Runs :meth:`encode` over a
        :class:`BytearraySink` and returns the freshly-encoded bytes.
        Callers targeting zero-alloc hot paths should call :meth:`encode`
        directly against a caller-owned sink."""
        _dst = bytearray()
        self.encode(BytearraySink(_dst))
        return bytes(_dst)
