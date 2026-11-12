// SCE-GENERATED — DO NOT EDIT
// source-hash: d5a1192f784ff99a5aa95877e35eef81f703dd6fbaf723312b776f25706b4354
// SCE-MAP: add_months.scxml:23 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `AddMonths`, a file of the
// module that calls it: a call is qualified by the enum
// (`AddMonths.addMonths(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum AddMonths {
    public static func addMonths(_ year: Int32, _ month: UInt8, _ day: UInt8, _ n: Int32) throws(AlgorithmError) -> CivilDatePayload {
        if !(month >= 1 && month <= 12) {
            throw AlgorithmError.precondition
        }
        if !((try Checked.and(day >= 1, { () throws(AlgorithmError) -> Bool in day <= (try DaysInMonth.daysInMonth(year, month)) }))) {
            throw AlgorithmError.precondition
        }
        let yr: Int64 = Int64(year)
        let mo: Int64 = Int64(month)
        let step: Int64 = Int64(n)
        let total: Int64 = (try Checked.add((try Checked.add((try Checked.mul(yr, 12)), (try Checked.sub(mo, 1)))), step))
        let q: Int64 = (try Checked.div(total, 12))
        let r: Int64 = (try Checked.rem(total, 12))
        let ny: Int64 = r < 0 ? (try Checked.sub(q, 1)) : q
        let nm: Int64 = (try Checked.add(r < 0 ? (try Checked.add(r, 12)) : r, 1))
        let last: UInt8 = (try DaysInMonth.daysInMonth((try Checked.narrow(ny, to: Int32.self)), (try Checked.narrow(nm, to: UInt8.self))))
        let date: CivilDatePayload = CivilDatePayload(year: (try Checked.narrow(ny, to: Int32.self)), month: (try Checked.narrow(nm, to: UInt8.self)), day: day < last ? day : last)
        return date
    }
}
