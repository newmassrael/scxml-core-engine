// SCE-GENERATED — DO NOT EDIT
// source-hash: 68d77b5e0c96e361239e1512887ba2ef6d35bfab0188bf0e2afbd53ca387f3bc
// SCE-MAP: weekday_from_days.scxml:18 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `WeekdayFromDays`, a file of the
// module that calls it: a call is qualified by the enum
// (`WeekdayFromDays.weekdayFromDays(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

public enum WeekdayFromDays {
    public static func weekdayFromDays(_ days: Int64) -> UInt8 {
        let w: Int64 = (days % 7 + 11) % 7
        return UInt8(truncatingIfNeeded: w)
    }
}
