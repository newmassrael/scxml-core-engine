// SCE-GENERATED — DO NOT EDIT
// source-hash: 68d77b5e0c96e361239e1512887ba2ef6d35bfab0188bf0e2afbd53ca387f3bc
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
    public static func daysFromCivil(_ year: Int32, _ month: UInt8, _ day: UInt8) throws -> Int64 {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        do {
            let sceCond2 = !(month >= 1 && month <= 12)
            try sceFailure.throwIfFailed()
            if sceCond2 {
                throw AlgorithmError.precondition
            }
        }
        do {
            let sceCond2 = !(day >= 1 && day <= Checked.take(sceFailure, fallback: 0, { try DaysInMonth.daysInMonth(year, month) }))
            try sceFailure.throwIfFailed()
            if sceCond2 {
                throw AlgorithmError.precondition
            }
        }
        let yr: Int64 = Int64(year)
        try sceFailure.throwIfFailed()
        let mo: Int64 = Int64(month)
        try sceFailure.throwIfFailed()
        let dy: Int64 = Int64(day)
        try sceFailure.throwIfFailed()
        let y: Int64 = mo <= 2 ? Checked.sub(sceFailure, yr, 1) : yr
        try sceFailure.throwIfFailed()
        let era: Int64 = Checked.div(sceFailure, y >= 0 ? y : Checked.sub(sceFailure, y, 399), 400)
        try sceFailure.throwIfFailed()
        let yoe: Int64 = Checked.sub(sceFailure, y, Checked.mul(sceFailure, era, 400))
        try sceFailure.throwIfFailed()
        let mp: Int64 = mo > 2 ? Checked.sub(sceFailure, mo, 3) : Checked.add(sceFailure, mo, 9)
        try sceFailure.throwIfFailed()
        let doy: Int64 = Checked.sub(sceFailure, Checked.add(sceFailure, Checked.div(sceFailure, Checked.add(sceFailure, Checked.mul(sceFailure, 153, mp), 2), 5), dy), 1)
        try sceFailure.throwIfFailed()
        let doe: Int64 = Checked.add(sceFailure, Checked.sub(sceFailure, Checked.add(sceFailure, Checked.mul(sceFailure, yoe, 365), Checked.div(sceFailure, yoe, 4)), Checked.div(sceFailure, yoe, 100)), doy)
        try sceFailure.throwIfFailed()
        do {
            let sceReturned: Int64 = Checked.sub(sceFailure, Checked.add(sceFailure, Checked.mul(sceFailure, era, 146097), doe), 719468)
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
