# SCE-MAP: algorithm_crc16:11

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.
#
# RFC §synth-5-A: pure synchronous function with bounded loops. Free
# function, no instance state. `bytes` parameters lower to Python's
# native `bytes` (RFC §synth-5-J-5 emitter table); iteration over `bytes`
# yields `int` values 0..255, which line up with the SCXML type-ctx
# contract that `<sce:foreach item>` is `uint8`. Numeric arithmetic
# in Python is arbitrary-precision so unsigned-narrow truncation is
# the body author's responsibility (e.g. `crc & 0xFFFF`).

def algorithm_crc16(data: bytes) -> int:
    crc: int = 0xFFFF
    for b in data:
        hi: int = b
        crc = crc ^ (hi << 8) & 0xFFFF
        i: int = 0
        while i < 8:
            if crc & 0x8000 != 0:
                crc = (crc << 1) & 0xFFFF ^ 0x1021
            else:
                crc = (crc << 1) & 0xFFFF
            i = (i + 1) & 0xFF
    return crc
