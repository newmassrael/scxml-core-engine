# SCE-MAP: algorithm_const_fold_smoke:24

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

DOUBLED: tuple = (0, 2, 4, 6,)

def algorithm_const_fold_smoke() -> int:
    return 0
