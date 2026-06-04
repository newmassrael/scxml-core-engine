# SCE-MAP: algorithm_bytes_equal:18

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.
#
# RFC §5.A: pure synchronous function with bounded loops. Free
# function, no instance state. `bytes` parameters lower to Python's
# native `bytes` (RFC §5.J.5 emitter table); iteration over `bytes`
# yields `int` values 0..255, which line up with the SCXML type-ctx
# contract that `<sce:foreach item>` is `uint8`. Numeric arithmetic
# in Python is arbitrary-precision so unsigned-narrow truncation is
# the body author's responsibility (e.g. `crc & 0xFFFF`).

def bytes_equal(a: bytes, b: bytes) -> bool:
    if len(a) != len(b):
        return False
    i: int = 0
    while i < len(a):
        if a[i] != b[i]:
            return False
        i = (i + 1) & 0xFFFFFFFF
    return True
