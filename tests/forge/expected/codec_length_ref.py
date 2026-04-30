# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from dataclasses import dataclass
from typing import Optional


@dataclass
class CodecLengthRef:
    msg_id: int = 0
    len: int = 0
    payload: bytes = b""

    @classmethod
    def decode(cls, raw: bytes) -> Optional[CodecLengthRef]:
        if len(raw) < 2:
            return None
        return cls(
            msg_id=raw[0],
            len=raw[1],
            payload=raw[2:2 + raw[1]],
        )

    def encode(self) -> bytes:
        r = bytearray()
        r.append(self.msg_id & 0xFF)
        r.append(self.len & 0xFF)
        r.extend(self.payload)
        return bytes(r)
