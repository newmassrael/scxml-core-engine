# SCE-MAP: codec_zenoh_hello:41

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, SceCursor, SceSink
from .codec_zenoh_locator import CodecZenohLocator

from dataclasses import dataclass, field
from typing import Optional, List


@dataclass
class CodecZenohHello:
    version: int = 0
    cbyte: int = 0
    zid: bytes = b""
    num_locators: Optional[int] = None
    locators: Optional[List[CodecZenohLocator]] = None

    @classmethod
    def decode(cls, cursor: SceCursor, l: int) -> Optional[CodecZenohHello]:
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
            version = raw[0]
            cursor.advance(1)
            raw = cursor.peek_slice(1)
            cbyte = raw[0]
            cursor.advance(1)
            _n = (((cbyte >> 4) & 0xF) + 1)
            raw = cursor.peek_slice(_n)
            zid = bytes(raw)
            cursor.advance(_n)
            if (l & 0x01) != 0:
                _v = cursor.read_vle_u64()
                num_locators = _v
            else:
                num_locators = None
            if (l & 0x01) != 0:
                locators = []
                for _ in range(num_locators):
                    _elem = CodecZenohLocator.decode(cursor)
                    if _elem is None:
                        return None
                    locators.append(_elem)
            else:
                locators = None
        except NeedMoreBytes:
            return None
        return cls(
            version=version,
            cbyte=cbyte,
            zid=zid,
            num_locators=num_locators,
            locators=locators,
        )

    # RFC §synth-5-B flags primitive: per-bit-range accessors over
    # the carrier field. Single-bit (width=1) reads as bool; multi-bit
    # (width>=2) reads as ``int`` (Python ints are unbounded, so a single
    # ``int`` covers every result-type width). Setters mask + shift on
    # the way in so out-of-range callers can't corrupt sibling bits.
    # Plain methods (rather than @property) for API symmetry with
    # Rust / Cpp / Kotlin / Go / C11. Wire layout is unchanged.
    def whatami(self) -> int:
        return (self.cbyte >> 0) & 0x03

    def set_whatami(self, v: int) -> None:
        _shifted_mask = 0x03 << 0
        _val = (v & 0x03) << 0
        self.cbyte = ((self.cbyte & (0xFF ^ _shifted_mask)) | _val) & 0xFF

    def zid_len_m1(self) -> int:
        return (self.cbyte >> 4) & 0x0F

    def set_zid_len_m1(self, v: int) -> None:
        _shifted_mask = 0x0F << 4
        _val = (v & 0x0F) << 4
        self.cbyte = ((self.cbyte & (0xFF ^ _shifted_mask)) | _val) & 0xFF

    def encode(self, w: SceSink, l: int) -> None:
        """RFC §synth-5-B encode-side primary: write ``self`` into the
        caller-owned ``w`` sink. Returns ``None`` on success; raises
        :class:`BufferOverflow` from a bounded sink when the destination
        has insufficient remaining capacity; growable sinks (e.g.
        :class:`BytearraySink`) are effectively infallible."""
        # RFC §synth-5-B present-if encode.
        w.write_u8(self.version & 0xFF)
        w.write_u8(self.cbyte & 0xFF)
        w.write_bytes(self.zid)
        if self.num_locators is not None:
            _vle = int(self.num_locators)
            while _vle >= 0x80:
                w.write_u8((_vle & 0x7F) | 0x80)
                _vle >>= 7
            w.write_u8(_vle)
        if self.locators is not None:
            for _e in self.locators:
                _e.encode(w)

    def encode_to_bytes(self, l: int) -> bytes:
        """Heap-backed convenience facade. Runs :meth:`encode` over a
        :class:`BytearraySink` and returns the freshly-encoded bytes.
        Callers targeting zero-alloc hot paths should call :meth:`encode`
        directly against a caller-owned sink."""
        _dst = bytearray()
        self.encode(BytearraySink(_dst), l)
        return bytes(_dst)
