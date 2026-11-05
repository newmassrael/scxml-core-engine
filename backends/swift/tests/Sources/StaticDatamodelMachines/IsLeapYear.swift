// SCE-GENERATED — DO NOT EDIT
// source-hash: b2575d48129e1acf0f70c09ad24f44255a7a62f71444c27101300757ede12222
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
