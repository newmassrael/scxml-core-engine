# SCE-MAP: codec_embed_basic:43

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import CodecError, NeedMoreBytes, SceCursor
from .codec_zenoh_locator import CodecZenohLocator

from dataclasses import dataclass
from typing import Optional


@dataclass
class CodecEmbedBasic:
    tag: int = 0
    locator: CodecZenohLocator = b""

    @classmethod
    def decode(cls, cursor: SceCursor) -> Optional[CodecEmbedBasic]:
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
            tag = raw[0]
            cursor.advance(1)
            locator = CodecZenohLocator.decode(cursor)
            if locator is None:
                return None
        except NeedMoreBytes:
            return None
        return cls(
            tag=tag,
            locator=locator,
        )

    def encode(self) -> bytes:
        # RFC §5.B B2 encode: fixed prefix appends byte-by-byte; repeat
        # fields iterate `self.<id>` and extend the bytearray with each
        # element's `encode()`. Author keeps count field == list length
        # (trust contract).
        r = bytearray()
        r.append(self.tag & 0xFF)
        r.extend(self.locator.encode())
        return bytes(r)
