# SCE-MAP: codec_zenoh_keep_alive:10

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import CodecError, NeedMoreBytes, SceCursor

from dataclasses import dataclass
from typing import Optional


@dataclass
class CodecZenohKeepAlive:
    # RFC §5.B B5-α empty body — Python @dataclass tolerates zero
    # fields; `pass` keeps the class body syntactically non-empty so
    # methods below attach correctly.
    pass

    @classmethod
    def decode(cls, cursor: SceCursor) -> Optional[CodecZenohKeepAlive]:
        """Decode the next frame from ``cursor``. Returns ``None`` when
        the cursor's tail is shorter than the declared minimum frame
        (RFC §5.B L494-519); on success the cursor advances past the
        consumed bytes. VLE codecs also return ``None`` on
        ``VleWidthOverflow``."""
        # RFC §5.B B5-α empty body — zero-byte payload, no cursor work.
        _ = cursor
        return cls()

    def encode(self) -> bytes:
        # RFC §5.B B5-α empty body — zero-byte payload.
        return b""
