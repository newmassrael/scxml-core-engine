// SCE-GENERATED — DO NOT EDIT
// source-hash: 138d54bef1f6b9b0e78600cb129e88342af00344f9fa58b6a90edf9adcaeb40d
// SCE-MAP: algorithm_days_in_month.scxml:14 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `MonthLength`, a file of the
// module that calls it: a call is qualified by the enum
// (`MonthLength.monthLength(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

public enum MonthLength {
    public static func monthLength(_ year: UInt16, _ month: UInt8) -> UInt8 {
        var days: UInt8 = 31
        if month == 4 || month == 6 || month == 9 || month == 11 {
            days = 30
        }
        if month == 2 {
            days = 28
            if year % 4 == 0 && year % 100 != 0 || year % 400 == 0 {
                days = 29
            }
        }
        return days
    }
}
