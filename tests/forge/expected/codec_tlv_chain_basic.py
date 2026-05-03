# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import CodecError, NeedMoreBytes, SceCursor, TlvChainOverflow
from .codec_tlv_entry import CodecTlvEntry

from dataclasses import dataclass, field
from typing import Optional, List


@dataclass
class CodecTlvChainBasic:
    header_flags: int = 0
    extensions: List[CodecTlvEntry] = field(default_factory=list)

    @classmethod
    def decode(cls, cursor: SceCursor) -> Optional[CodecTlvChainBasic]:
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
            header_flags = raw[0]
            cursor.advance(1)
            extensions = []
            for _ in range(8):
                if cursor.remaining() == 0:
                    break
                _elem = CodecTlvEntry.decode(cursor)
                if _elem is None:
                    return None
                extensions.append(_elem)
            if cursor.remaining() > 0:
                raise TlvChainOverflow()
        except NeedMoreBytes:
            return None
        return cls(
            header_flags=header_flags,
            extensions=extensions,
        )

    def encode(self) -> bytes:
        # RFC §5.B B2 encode: fixed prefix appends byte-by-byte; repeat
        # fields iterate `self.<id>` and extend the bytearray with each
        # element's `encode()`. Author keeps count field == list length
        # (trust contract).
        r = bytearray()
        r.append(self.header_flags & 0xFF)
        for _e in self.extensions:
            r.extend(_e.encode())
        return bytes(r)
