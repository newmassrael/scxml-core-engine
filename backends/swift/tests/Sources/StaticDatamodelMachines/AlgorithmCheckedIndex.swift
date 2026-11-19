// SCE-GENERATED — DO NOT EDIT
// source-hash: 80c4b34a67af488527a8af2e0bd7c88ff63fd2444e341fe9d1548c600b90d53a
// source: algorithm_checked_index.scxml
// SCE-MAP: algorithm_checked_index.scxml:12 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `AlgorithmCheckedIndex`, a file of the
// module that calls it: a call is qualified by the enum
// (`AlgorithmCheckedIndex.algorithmCheckedIndex(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum AlgorithmCheckedIndex {
    public static func algorithmCheckedIndex(_ data: [UInt8], _ i: Int32, _ u: UInt32) throws(AlgorithmError) -> UInt8 {
        return (try Checked.at(data, i)) ^ (try Checked.at(data, u))
    }
}
