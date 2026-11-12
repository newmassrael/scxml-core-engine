// SCE-GENERATED — DO NOT EDIT
// source-hash: d5a1192f784ff99a5aa95877e35eef81f703dd6fbaf723312b776f25706b4354
// SCE-MAP: civil_from_days.scxml:24 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `CivilFromDays`, a file of the
// module that calls it: a call is qualified by the enum
// (`CivilFromDays.civilFromDays(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum CivilFromDays {
    public static func civilFromDays(_ days: Int64) throws(AlgorithmError) -> CivilDatePayload {
        let z: Int64 = (try Checked.add(days, 719468))
        let era: Int64 = (try Checked.div(z >= 0 ? z : (try Checked.sub(z, 146096)), 146097))
        let doe: Int64 = (try Checked.sub(z, (try Checked.mul(era, 146097))))
        let yoe: Int64 = (try Checked.div((try Checked.sub((try Checked.add((try Checked.sub(doe, (try Checked.div(doe, 1460)))), (try Checked.div(doe, 36524)))), (try Checked.div(doe, 146096)))), 365))
        let doy: Int64 = (try Checked.sub(doe, (try Checked.sub((try Checked.add((try Checked.mul(365, yoe)), (try Checked.div(yoe, 4)))), (try Checked.div(yoe, 100))))))
        let mp: Int64 = (try Checked.div((try Checked.add((try Checked.mul(5, doy)), 2)), 153))
        let m: Int64 = mp < 10 ? (try Checked.add(mp, 3)) : (try Checked.sub(mp, 9))
        let y: Int64 = (try Checked.add(yoe, (try Checked.mul(era, 400))))
        let date: CivilDatePayload = CivilDatePayload(year: (try Checked.narrow(m <= 2 ? (try Checked.add(y, 1)) : y, to: Int32.self)), month: (try Checked.narrow(m, to: UInt8.self)), day: (try Checked.narrow((try Checked.add((try Checked.sub(doy, (try Checked.div((try Checked.add((try Checked.mul(153, mp)), 2)), 5)))), 1)), to: UInt8.self)))
        return date
    }
}
