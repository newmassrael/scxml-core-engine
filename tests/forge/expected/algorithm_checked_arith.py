# SCE-MAP: algorithm_checked_arith:18 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
# Runtime: sce_forge_runtime
# Do not edit — regenerate from the source SCXML file.
#
# RFC §synth-5-A: pure synchronous function with bounded loops. Free
# function, no instance state. `bytes` parameters lower to Python's
# native `bytes` (RFC §synth-5-J-5 emitter table); iteration over `bytes`
# yields `int` values 0..255, which line up with the SCXML type-ctx
# contract that `<sce:foreach item>` is `uint8`. Numeric arithmetic
# in Python is arbitrary-precision so unsigned-narrow truncation is
# the body author's responsibility (e.g. `crc & 0xFFFF`).
#
# SCE_FORGE.md §3.4.1: this algorithm declares may-fail. Each integer
# operation goes through `sce_algorithm`, which holds it to its declared
# width, and a failure raises `sce_algorithm.AlgorithmFailure` to the caller
# in place of a value.

from sce_forge_runtime import algorithm as sce_algorithm

def algorithm_checked_arith(a: int, b: int, op: int) -> int:
    r: int = 0
    if op == 0:
        r = sce_algorithm.I32.add(a, b)
    if op == 1:
        r = sce_algorithm.I32.sub(a, b)
    if op == 2:
        r = sce_algorithm.I32.mul(a, b)
    if op == 3:
        r = sce_algorithm.I32.div(a, b)
    if op == 4:
        r = sce_algorithm.I32.rem(a, b)
    if op == 5:
        r = sce_algorithm.I32.neg(a)
    if op == 6:
        r = sce_algorithm.U8.sub(op, 7)
    return r
