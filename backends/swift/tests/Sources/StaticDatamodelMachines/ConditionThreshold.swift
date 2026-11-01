// SCE-GENERATED — DO NOT EDIT
// source-hash: a0ed8bd925c949021fa83a15dd0434623cdc2a31d67a1cec484ba6e7168d79a0
// SCE-MAP: condition_threshold.scxml:3 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="condition")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// A named boolean guard: a static function of the caseless enum `ConditionThreshold`, a
// file of the module that calls it. A call is qualified by the enum
// (`ConditionThreshold.conditionThreshold(…)`), which a member of the calling type cannot hide as
// it would hide a free function of the same name.

public enum ConditionThreshold {
    public static func conditionThreshold(_ coolantTemp: Double, _ oilTemp: Double, _ maxTemp: Double) -> Bool {
        coolantTemp > maxTemp || oilTemp > maxTemp
    }
}
