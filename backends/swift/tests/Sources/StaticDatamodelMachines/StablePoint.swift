// SCE-GENERATED — DO NOT EDIT
// source-hash: 61656e86d01d1e5e7ddec5f81a1066adefda759dfa20991a07594bf3faeb68d6
// source: sce:std/merge/stable_point.scxml
// SCE-MAP: stable_point.scxml:33 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `StablePoint`, a file of the
// module that calls it: a call is qualified by the enum
// (`StablePoint.stablePoint(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum StablePoint {
    public static func stablePoint(_ cursors: [UInt64], _ lastSeen: [Int64], _ now: Int64, _ maxIdle: Int64) throws(AlgorithmError) -> UInt64 {
        if !(cursors.count == lastSeen.count) {
            throw AlgorithmError.precondition
        }
        if !(maxIdle >= 0) {
            throw AlgorithmError.precondition
        }
        var stable: UInt64 = 0
        var taking: Bool = false
        var i: UInt32 = 0
        while i < UInt32(truncatingIfNeeded: cursors.count) {
            if (try Checked.sub(now, (try Checked.at(lastSeen, i)))) <= maxIdle {
                if (try Checked.or(!taking, { () throws(AlgorithmError) -> Bool in (try Checked.at(cursors, i)) < stable })) {
                    stable = (try Checked.at(cursors, i))
                }
                taking = true
            }
            i = (try Checked.add(i, 1))
        }
        return stable
    }
}
