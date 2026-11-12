// SCE-GENERATED — DO NOT EDIT
// source-hash: d5a1192f784ff99a5aa95877e35eef81f703dd6fbaf723312b776f25706b4354
// SCE-MAP: days_from_civil.scxml:28 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `DaysFromCivil`, a file of the
// module that calls it: a call is qualified by the enum
// (`DaysFromCivil.daysFromCivil(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum DaysFromCivil {
    public static func daysFromCivil(_ year: Int32, _ month: UInt8, _ day: UInt8) throws(AlgorithmError) -> Int64 {
        if !(month >= 1 && month <= 12) {
            throw AlgorithmError.precondition
        }
        if !((try Checked.and(day >= 1, { day <= (try DaysInMonth.daysInMonth(year, month)) }))) {
            throw AlgorithmError.precondition
        }
        let yr: Int64 = Int64(year)
        let mo: Int64 = Int64(month)
        let dy: Int64 = Int64(day)
        let y: Int64 = mo <= 2 ? (try Checked.sub(yr, 1)) : yr
        let era: Int64 = (try Checked.div(y >= 0 ? y : (try Checked.sub(y, 399)), 400))
        let yoe: Int64 = (try Checked.sub(y, (try Checked.mul(era, 400))))
        let mp: Int64 = mo > 2 ? (try Checked.sub(mo, 3)) : (try Checked.add(mo, 9))
        let doy: Int64 = (try Checked.sub((try Checked.add((try Checked.div((try Checked.add((try Checked.mul(153, mp)), 2)), 5)), dy)), 1))
        let doe: Int64 = (try Checked.add((try Checked.sub((try Checked.add((try Checked.mul(yoe, 365)), (try Checked.div(yoe, 4)))), (try Checked.div(yoe, 100)))), doy))
        return (try Checked.sub((try Checked.add((try Checked.mul(era, 146097)), doe)), 719468))
    }
}
