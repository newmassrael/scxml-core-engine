// SCE-GENERATED — DO NOT EDIT
// source-hash: 68d77b5e0c96e361239e1512887ba2ef6d35bfab0188bf0e2afbd53ca387f3bc
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
    public static func civilFromDays(_ days: Int64) throws -> CivilDatePayload {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        let z: Int64 = Checked.add(sceFailure, days, 719468)
        try sceFailure.throwIfFailed()
        let era: Int64 = Checked.div(sceFailure, z >= 0 ? z : Checked.sub(sceFailure, z, 146096), 146097)
        try sceFailure.throwIfFailed()
        let doe: Int64 = Checked.sub(sceFailure, z, Checked.mul(sceFailure, era, 146097))
        try sceFailure.throwIfFailed()
        let yoe: Int64 = Checked.div(sceFailure, Checked.sub(sceFailure, Checked.add(sceFailure, Checked.sub(sceFailure, doe, Checked.div(sceFailure, doe, 1460)), Checked.div(sceFailure, doe, 36524)), Checked.div(sceFailure, doe, 146096)), 365)
        try sceFailure.throwIfFailed()
        let doy: Int64 = Checked.sub(sceFailure, doe, Checked.sub(sceFailure, Checked.add(sceFailure, Checked.mul(sceFailure, 365, yoe), Checked.div(sceFailure, yoe, 4)), Checked.div(sceFailure, yoe, 100)))
        try sceFailure.throwIfFailed()
        let mp: Int64 = Checked.div(sceFailure, Checked.add(sceFailure, Checked.mul(sceFailure, 5, doy), 2), 153)
        try sceFailure.throwIfFailed()
        let m: Int64 = mp < 10 ? Checked.add(sceFailure, mp, 3) : Checked.sub(sceFailure, mp, 9)
        try sceFailure.throwIfFailed()
        let y: Int64 = Checked.add(sceFailure, yoe, Checked.mul(sceFailure, era, 400))
        try sceFailure.throwIfFailed()
        let date: CivilDatePayload = CivilDatePayload(year: Checked.narrow(sceFailure, m <= 2 ? Checked.add(sceFailure, y, 1) : y, to: Int32.self), month: Checked.narrow(sceFailure, m, to: UInt8.self), day: Checked.narrow(sceFailure, Checked.add(sceFailure, Checked.sub(sceFailure, doy, Checked.div(sceFailure, Checked.add(sceFailure, Checked.mul(sceFailure, 153, mp), 2), 5)), 1), to: UInt8.self))
        try sceFailure.throwIfFailed()
        do {
            let sceReturned: CivilDatePayload = date
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
