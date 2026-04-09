# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from dataclasses import dataclass
from typing import Optional


@dataclass
class CodecLittleEndian:
    sensor_id: int
    value: int
    status: int

    @classmethod
    def decode(cls, raw: bytes) -> Optional[CodecLittleEndian]:
        if len(raw) < 4:
            return None
        return cls(
            sensor_id=raw[0],
            value=raw[1] | (raw[2] << 8),
            status=raw[3],
        )

    def encode(self) -> bytes:
        return bytes([
            self.sensor_id & 0xFF,
            self.value & 0xFF,
            (self.value >> 8) & 0xFF,
            self.status & 0xFF,
        ])