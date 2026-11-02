// SCE-GENERATED — DO NOT EDIT
// source-hash: f52959f8eb76029a8e6cff0fae9c1d7f0469eb21cd52301f306b53afbc606d1a
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
