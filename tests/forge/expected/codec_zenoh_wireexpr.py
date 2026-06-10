# SCE-MAP: codec_zenoh_wireexpr:53

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, InvalidUtf8, NeedMoreBytes, SceCursor, SceSink

from dataclasses import dataclass
from typing import Optional


@dataclass
class CodecZenohWireexpr:
    id: int = 0
    suffix_len: Optional[int] = None
    suffix: Optional[str] = None

    @classmethod
    def decode(cls, cursor: SceCursor, n: int) -> Optional[CodecZenohWireexpr]:
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
            id = cursor.read_vle_u64()
            if (n & 0x01) != 0:
                _v = cursor.read_vle_u64()
                suffix_len = _v
            else:
                suffix_len = None
            if (n & 0x01) != 0:
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
        except NeedMoreBytes:
            return None
        return cls(
            id=id,
            suffix_len=suffix_len,
            suffix=suffix,
        )

    def encode(self, w: SceSink, n: int) -> None:
        """RFC §synth-5-B encode-side primary: write ``self`` into the
        caller-owned ``w`` sink. Returns ``None`` on success; raises
        :class:`BufferOverflow` from a bounded sink when the destination
        has insufficient remaining capacity; growable sinks (e.g.
        :class:`BytearraySink`) are effectively infallible."""
        # RFC §synth-5-B present-if encode.
        _vle = int(self.id)
        while _vle >= 0x80:
            w.write_u8((_vle & 0x7F) | 0x80)
            _vle >>= 7
        w.write_u8(_vle)
        if self.suffix_len is not None:
            _vle = int(self.suffix_len)
            while _vle >= 0x80:
                w.write_u8((_vle & 0x7F) | 0x80)
                _vle >>= 7
            w.write_u8(_vle)
        if self.suffix is not None:
            w.write_bytes(self.suffix.encode('utf-8'))

    def encode_to_bytes(self, n: int) -> bytes:
        """Heap-backed convenience facade. Runs :meth:`encode` over a
        :class:`BytearraySink` and returns the freshly-encoded bytes.
        Callers targeting zero-alloc hot paths should call :meth:`encode`
        directly against a caller-owned sink."""
        _dst = bytearray()
        self.encode(BytearraySink(_dst), n)
        return bytes(_dst)
