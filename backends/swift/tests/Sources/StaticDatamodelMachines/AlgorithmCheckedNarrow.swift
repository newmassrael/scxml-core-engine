// SCE-GENERATED — DO NOT EDIT
// source-hash: 1ee4c478fcd6af0bb675edd6c8e90348072aad4cd4e5fb9446a5e52706545b90
// SCE-MAP: algorithm_checked_narrow.scxml:12 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `AlgorithmCheckedNarrow`, a file of the
// module that calls it: a call is qualified by the enum
// (`AlgorithmCheckedNarrow.algorithmCheckedNarrow(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum AlgorithmCheckedNarrow {
    public static func algorithmCheckedNarrow(_ x: Int64, _ y: UInt32) throws(AlgorithmError) -> Int32 {
        let small: UInt8 = (try Checked.narrow(y, to: UInt8.self))
        let wide: Int64 = (try Checked.add(x, Int64(small)))
        return (try Checked.narrow(wide, to: Int32.self))
    }
}
