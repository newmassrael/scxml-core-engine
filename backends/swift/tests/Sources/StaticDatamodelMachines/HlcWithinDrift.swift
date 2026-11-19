// SCE-GENERATED — DO NOT EDIT
// source-hash: b2cf10fca5e63646dc1d7bb6bf5818ec617ab2a0d803d6410cc44dcd29a38bfa
// source: sce:std/merge/hlc_within_drift.scxml
// SCE-MAP: hlc_within_drift.scxml:23 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `HlcWithinDrift`, a file of the
// module that calls it: a call is qualified by the enum
// (`HlcWithinDrift.hlcWithinDrift(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum HlcWithinDrift {
    public static func hlcWithinDrift(_ wallTime: Int64, _ now: Int64, _ maxDrift: Int64) throws(AlgorithmError) -> Bool {
        if !(maxDrift >= 0) {
            throw AlgorithmError.precondition
        }
        if wallTime <= now {
            return true
        }
        return (try Checked.sub(wallTime, now)) <= maxDrift
    }
}
