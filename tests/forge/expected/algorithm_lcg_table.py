# SCE-MAP: algorithm_lcg_table:13 :: _forge_body

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

LCG_TABLE: tuple = (12345, 87628868, 71072467, 2332836374, 2726892157, 3908547000, 483019191, 2129828778, 2355140353, 2560230508, 3364893915, 171172990, 3194601925, 4148119648, 316399679, 3004788882,)

def algorithm_lcg_table(n: int) -> int:
    return LCG_TABLE[n & 15]
