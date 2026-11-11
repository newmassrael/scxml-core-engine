// SCE-GENERATED — DO NOT EDIT
// source-hash: d5a1192f784ff99a5aa95877e35eef81f703dd6fbaf723312b776f25706b4354
// SCE-MAP: second_of_day.scxml:18 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `SecondOfDay`, a file of the
// module that calls it: a call is qualified by the enum
// (`SecondOfDay.secondOfDay(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

public enum SecondOfDay {
    public static func secondOfDay(_ seconds: Int64) -> Int32 {
        let r: Int64 = seconds % 86400
        let s: Int64 = r < 0 ? r + 86400 : r
        return Int32(truncatingIfNeeded: s)
    }
}
