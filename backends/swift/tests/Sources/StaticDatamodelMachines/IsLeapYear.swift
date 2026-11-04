// SCE-GENERATED — DO NOT EDIT
// source-hash: 0c71031c0d27e4d67ce7e4f29c8b0629804b2b22ba0439698c8a940297a0f1bd
// SCE-MAP: is_leap_year.scxml:15 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `IsLeapYear`, a file of the
// module that calls it: a call is qualified by the enum
// (`IsLeapYear.isLeapYear(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

public enum IsLeapYear {
    public static func isLeapYear(_ year: Int32) -> Bool {
        return year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
    }
}
