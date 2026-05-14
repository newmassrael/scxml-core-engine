# SCE-MAP: codec_until_eof_basic:10

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import CodecError, NeedMoreBytes, SceCursor
from .codec_repeat_elem import CodecRepeatElem

from dataclasses import dataclass, field
from typing import Optional, List


@dataclass
class CodecUntilEofBasic:
    msgs: List[CodecRepeatElem] = field(default_factory=list)

    @classmethod
    def decode(cls, cursor: SceCursor) -> Optional[CodecUntilEofBasic]:
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
            msgs = []
            while cursor.remaining() > 0:
                _elem = CodecRepeatElem.decode(cursor)
                if _elem is None:
                    return None
                msgs.append(_elem)
        except NeedMoreBytes:
            return None
        return cls(
            msgs=msgs,
        )

    def encode(self) -> bytes:
        # RFC §5.B B2 encode: fixed prefix appends byte-by-byte; repeat
        # fields iterate `self.<id>` and extend the bytearray with each
        # element's `encode()`. Author keeps count field == list length
        # (trust contract).
        r = bytearray()
        for _e in self.msgs:
            r.extend(_e.encode())
        return bytes(r)
