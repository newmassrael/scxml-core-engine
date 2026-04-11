# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from dataclasses import dataclass
from typing import Optional


@dataclass
class CodecSubbyte:
    priority: int = 0
    channel: int = 0
    direction: int = 0

    @classmethod
    def decode(cls, raw: bytes) -> Optional[CodecSubbyte]:
        if len(raw) < 1:
            return None
        return cls(
            priority=(raw[0] >> 5) & 0x07,
            channel=(raw[0] >> 2) & 0x07,
            direction=(raw[0] >> 0) & 0x03,
        )

    def encode(self) -> bytes:
        return bytes([
            ((self.priority & 0x07) << 5 | (self.channel & 0x07) << 2 | (self.direction & 0x03) << 0) & 0xFF,
        ])
