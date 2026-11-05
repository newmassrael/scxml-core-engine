// SCE-GENERATED — DO NOT EDIT
// source-hash: 756970c374508abdd4d68a996d5fd6f31c978e573ad8a3257b72b5a7e7773666
// SCE-MAP: condition_range.scxml:3 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="condition")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// A named boolean guard: a static function of the caseless enum `ConditionRange`, a
// file of the module that calls it. A call is qualified by the enum
// (`ConditionRange.conditionRange(…)`), which a member of the calling type cannot hide as
// it would hide a free function of the same name.

public enum ConditionRange {
    public static func conditionRange(_ rpm: UInt32, _ minRpm: UInt32, _ maxRpm: UInt32) -> Bool {
        rpm >= minRpm && rpm <= maxRpm
    }
}
