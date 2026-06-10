# SCE-MAP: codec_repeat_basic:11

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, SceCursor, SceSink
from .codec_repeat_elem import CodecRepeatElem

from dataclasses import dataclass, field
from typing import Optional, List


@dataclass
class CodecRepeatBasic:
    num_frags: int = 0
    frags: List[CodecRepeatElem] = field(default_factory=list)

    @classmethod
    def decode(cls, cursor: SceCursor) -> Optional[CodecRepeatBasic]:
        """Decode the next frame from ``cursor``. Returns ``None`` when
        the cursor's tail is shorter than the declared minimum frame
        (RFC §5.B L494-519); on success the cursor advances past the
        consumed bytes. VLE codecs also return ``None`` on
        ``VleWidthOverflow``."""
        # RFC §5.B B2 repeat primitive: streaming decode mixes plain
        # fixed-width reads (per-field via the present-if helper's
        # non-gated arm) with `for _ in range(N)` / `while
        # cursor.remaining() > 0` loops that iterate the imported
        # codec's `decode()`. Element bodies recurse and may surface
        # `None`, which propagates up via early `return None` from
        # this outer `try:` block.
        try:
            raw = cursor.peek_slice(1)
            num_frags = raw[0]
            cursor.advance(1)
            frags = []
            for _ in range(num_frags):
                _elem = CodecRepeatElem.decode(cursor)
                if _elem is None:
                    return None
                frags.append(_elem)
        except NeedMoreBytes:
            return None
        return cls(
            num_frags=num_frags,
            frags=frags,
        )

    def encode(self, w: SceSink) -> None:
        """RFC §5.B encode-side primary: write ``self`` into the
        caller-owned ``w`` sink. Returns ``None`` on success; raises
        :class:`BufferOverflow` from a bounded sink when the destination
        has insufficient remaining capacity; growable sinks (e.g.
        :class:`BytearraySink`) are effectively infallible."""
        # RFC §5.B B2 encode: list fields iterate ``self.<id>`` and
        # write each element through the same sink.
        w.write_u8(self.num_frags & 0xFF)
        for _e in self.frags:
            _e.encode(w)

    def encode_to_bytes(self) -> bytes:
        """Heap-backed convenience facade. Runs :meth:`encode` over a
        :class:`BytearraySink` and returns the freshly-encoded bytes.
        Callers targeting zero-alloc hot paths should call :meth:`encode`
        directly against a caller-owned sink."""
        _dst = bytearray()
        self.encode(BytearraySink(_dst))
        return bytes(_dst)
