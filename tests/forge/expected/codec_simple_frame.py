# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from dataclasses import dataclass
from typing import Optional


@dataclass
class CodecSimpleFrame:
    msg_id: int
    length: int
    payload: int

    @classmethod
    def decode(cls, raw: bytes) -> Optional[CodecSimpleFrame]:
        if len(raw) < 4:
            return None
        return cls(
            msg_id=raw[0],
            length=raw[1],
            payload=(raw[2] << 8) | raw[3],
        )

    def encode(self) -> bytes:
        return bytes([
            self.msg_id & 0xFF,
            self.length & 0xFF,
            (self.payload >> 8) & 0xFF,
            self.payload & 0xFF,
        ])
