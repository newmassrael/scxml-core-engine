// SCE-MAP: algorithm_lcg_table:13 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `algorithm_lcg_table`, no instance state. `bytes`
// parameters lower to `[]byte` (RFC §synth-5-J-5 emitter table).

package algorithm_lcg_table

var LCG_TABLE = [16]uint32{ 12345, 87628868, 71072467, 2332836374, 2726892157, 3908547000, 483019191, 2129828778, 2355140353, 2560230508, 3364893915, 171172990, 3194601925, 4148119648, 316399679, 3004788882 }

func AlgorithmLcgTable(n uint8) uint32 {
    return LCG_TABLE[n & 15]
}
