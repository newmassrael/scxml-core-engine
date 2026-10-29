// SCE-MAP: algorithm_tick_until_due:12 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `algorithm_tick_until_due`, no instance state. `bytes`
// parameters lower to `[]byte` (RFC §synth-5-J-5 emitter table).

package algorithm_tick_until_due

import (
	scealgorithm "github.com/newmassrael/sce-forge-runtime/algorithm"
)

func AlgorithmTickUntilDue(now uint32, last uint32, period uint32) uint32 {
    return scealgorithm.SaturatingSubUint32(period, scealgorithm.WrappingSubUint32(now, last))
}
