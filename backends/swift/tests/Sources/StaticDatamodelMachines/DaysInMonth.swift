// SCE-GENERATED — DO NOT EDIT
// source-hash: 3f51ae17d132a35fd3a1ecd1d4b65de1ff71b9a382e0fdbb0ba786d0f4532e45
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
    public static func daysInMonth(_ year: Int32, _ month: UInt8) throws(AlgorithmError) -> UInt8 {
        if !(month >= 1 && month <= 12) {
            throw AlgorithmError.precondition
        }
        if month == 2 {
            if IsLeapYear.isLeapYear(year) {
                return 29
            }
            return 28
        }
        if month == 4 || month == 6 || month == 9 || month == 11 {
            return 30
        }
        return 31
    }
}
