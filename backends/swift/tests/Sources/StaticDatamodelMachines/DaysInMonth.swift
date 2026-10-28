// SCE-GENERATED — DO NOT EDIT
// source-hash: b67ac92c47dabab09bc66be64b0a4b4a6acfabe17f95412102ed640a717fc04f
// SCE-MAP: algorithm_days_in_month.scxml:8 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `DaysInMonth`, a file of the
// module that calls it: a call is qualified by the enum
// (`DaysInMonth.daysInMonth(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

public enum DaysInMonth {
    public static func daysInMonth(_ year: UInt16, _ month: UInt8) -> UInt8 {
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
