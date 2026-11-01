// SCE-GENERATED — DO NOT EDIT
// source-hash: 54480e3a74914cc26e2c9613cedbc157df3a8f640cc21e599aebce75be7f1163
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
