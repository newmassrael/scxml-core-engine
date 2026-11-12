// SCE-GENERATED — DO NOT EDIT
// source-hash: d5a1192f784ff99a5aa95877e35eef81f703dd6fbaf723312b776f25706b4354
// SCE-MAP: utc_from_local.scxml:30 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `UtcFromLocal`, a file of the
// module that calls it: a call is qualified by the enum
// (`UtcFromLocal.utcFromLocal(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum UtcFromLocal {
    public static func utcFromLocal(_ starts: [Int64], _ offsets: [Int32], _ local: Int64) throws(AlgorithmError) -> Int64 {
        if !(starts.count == offsets.count && starts.count > 0) {
            throw AlgorithmError.precondition
        }
        if !((try Checked.sub(local, Int64((try Checked.at(offsets, 0))))) >= (try Checked.at(starts, 0))) {
            throw AlgorithmError.precondition
        }
        let n: UInt32 = UInt32(truncatingIfNeeded: starts.count)
        var k: UInt32 = 0
        while k < n {
            if k > 0 {
                if !((try Checked.at(starts, k)) > (try Checked.at(starts, (try Checked.sub(k, 1))))) {
                    throw AlgorithmError.precondition
                }
            }
            let u: Int64 = (try Checked.sub(local, Int64((try Checked.at(offsets, k)))))
            if (try Checked.and(u >= (try Checked.at(starts, k)), { (try Checked.or((try Checked.add(k, 1)) == n, { u < (try Checked.at(starts, (try Checked.add(k, 1)))) })) })) {
                return u
            }
            k = (try Checked.add(k, 1))
        }
        var g: UInt32 = 1
        while g < n {
            let before: Int64 = (try Checked.sub(local, Int64((try Checked.at(offsets, (try Checked.sub(g, 1)))))))
            if (try Checked.and(before >= (try Checked.at(starts, g)), { (try Checked.sub(local, Int64((try Checked.at(offsets, g))))) < (try Checked.at(starts, g)) })) {
                return before
            }
            g = (try Checked.add(g, 1))
        }
        if !(false) {
            throw AlgorithmError.precondition
        }
        return 0
    }
}
