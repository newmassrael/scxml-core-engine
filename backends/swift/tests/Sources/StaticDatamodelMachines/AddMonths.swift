// SCE-GENERATED — DO NOT EDIT
// source-hash: 68d77b5e0c96e361239e1512887ba2ef6d35bfab0188bf0e2afbd53ca387f3bc
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
    public static func addMonths(_ year: Int32, _ month: UInt8, _ day: UInt8, _ n: Int32) throws -> CivilDatePayload {
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
        let step: Int64 = Int64(n)
        try sceFailure.throwIfFailed()
        let total: Int64 = Checked.add(sceFailure, Checked.add(sceFailure, Checked.mul(sceFailure, yr, 12), Checked.sub(sceFailure, mo, 1)), step)
        try sceFailure.throwIfFailed()
        let q: Int64 = Checked.div(sceFailure, total, 12)
        try sceFailure.throwIfFailed()
        let r: Int64 = Checked.rem(sceFailure, total, 12)
        try sceFailure.throwIfFailed()
        let ny: Int64 = r < 0 ? Checked.sub(sceFailure, q, 1) : q
        try sceFailure.throwIfFailed()
        let nm: Int64 = Checked.add(sceFailure, r < 0 ? Checked.add(sceFailure, r, 12) : r, 1)
        try sceFailure.throwIfFailed()
        let last: UInt8 = Checked.take(sceFailure, fallback: 0, { try DaysInMonth.daysInMonth(Checked.narrow(sceFailure, ny, to: Int32.self), Checked.narrow(sceFailure, nm, to: UInt8.self)) })
        try sceFailure.throwIfFailed()
        let date: CivilDatePayload = CivilDatePayload(year: Checked.narrow(sceFailure, ny, to: Int32.self), month: Checked.narrow(sceFailure, nm, to: UInt8.self), day: day < last ? day : last)
        try sceFailure.throwIfFailed()
        do {
            let sceReturned: CivilDatePayload = date
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
