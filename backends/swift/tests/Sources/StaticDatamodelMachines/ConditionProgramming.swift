// SCE-GENERATED — DO NOT EDIT
// source-hash: be2e66b7d73b7a0417f2cd73d69b4e29857f6dc165f59dbce69d4cc4daced358
// SCE-MAP: condition_programming.scxml:3 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="condition")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// A named boolean guard: a static function of the caseless enum `ConditionProgramming`, a
// file of the module that calls it. A call is qualified by the enum
// (`ConditionProgramming.conditionProgramming(…)`), which a member of the calling type cannot hide as
// it would hide a free function of the same name.

public enum ConditionProgramming {
    public static func conditionProgramming(_ engineStop: Bool, _ ignition: Bool) -> Bool {
        engineStop == true && ignition == true
    }
}
