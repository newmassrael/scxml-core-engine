// SCE-GENERATED — DO NOT EDIT
// source-hash: b2575d48129e1acf0f70c09ad24f44255a7a62f71444c27101300757ede12222
// SCE-MAP: days_from_epoch_seconds.scxml:18 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `DaysFromEpochSeconds`, a file of the
// module that calls it: a call is qualified by the enum
// (`DaysFromEpochSeconds.daysFromEpochSeconds(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

public enum DaysFromEpochSeconds {
    public static func daysFromEpochSeconds(_ seconds: Int64) -> Int64 {
        let q: Int64 = seconds / 86400
        let r: Int64 = seconds % 86400
        return r < 0 ? q - 1 : q
    }
}
