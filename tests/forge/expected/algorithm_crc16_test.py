# SCE Forge: Auto-generated test-vector sidecar (RFC §synth-5-B B2)
# Companion to algorithm_crc16.py — do not edit; regenerate from the source SCXML.

import unittest

from .algorithm_crc16 import algorithm_crc16


class AlgorithmCrc16TestVectors(unittest.TestCase):
    def test_vector_algorithm_crc16_l47(self) -> None:
        actual = algorithm_crc16(bytes([0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39]))
        expected = 0x29b1
        self.assertEqual(
            actual,
            expected,
            f"<sce:test-vector> at SCXML L47: algorithm_crc16(<313233343536373839>) returned {actual!r}, expected {expected!r}",
        )
