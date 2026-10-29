// SCE-GENERATED — DO NOT EDIT
// source-hash: 68d77b5e0c96e361239e1512887ba2ef6d35bfab0188bf0e2afbd53ca387f3bc
// SCE-MAP: days_in_month.scxml:16 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `DaysInMonth`, a file of the
// module that calls it: a call is qualified by the enum
// (`DaysInMonth.daysInMonth(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum DaysInMonth {
    public static func daysInMonth(_ year: Int32, _ month: UInt8) throws -> UInt8 {
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
            let sceCond2 = month == 2
            try sceFailure.throwIfFailed()
            if sceCond2 {
                do {
                    let sceCond4 = IsLeapYear.isLeapYear(year)
                    try sceFailure.throwIfFailed()
                    if sceCond4 {
                        do {
                            let sceReturned: UInt8 = 29
                            try sceFailure.throwIfFailed()
                            return sceReturned
                        }
                    }
                }
                do {
                    let sceReturned: UInt8 = 28
                    try sceFailure.throwIfFailed()
                    return sceReturned
                }
            }
        }
        do {
            let sceCond2 = month == 4 || month == 6 || month == 9 || month == 11
            try sceFailure.throwIfFailed()
            if sceCond2 {
                do {
                    let sceReturned: UInt8 = 30
                    try sceFailure.throwIfFailed()
                    return sceReturned
                }
            }
        }
        do {
            let sceReturned: UInt8 = 31
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
