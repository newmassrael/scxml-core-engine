// SCE-GENERATED — DO NOT EDIT
// source-hash: 7317770ec1283ca290af8bec72c8ce9016c9ca2036acfb8719dfd7159e505458
// SCE-MAP: algorithm_tick_until_due.scxml:12 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `AlgorithmTickUntilDue`, a file of the
// module that calls it: a call is qualified by the enum
// (`AlgorithmTickUntilDue.algorithmTickUntilDue(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum AlgorithmTickUntilDue {
    public static func algorithmTickUntilDue(_ now: UInt32, _ last: UInt32, _ period: UInt32) -> UInt32 {
        return Saturating.sub(period, Wrapping.sub(now, last))
    }
}
