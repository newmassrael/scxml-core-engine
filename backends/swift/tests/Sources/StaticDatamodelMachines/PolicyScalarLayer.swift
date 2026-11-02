// SCE-GENERATED — DO NOT EDIT
// source-hash: 814c7782483a2e1383636d59b115ce52cb83fc95e5c66e5ef4b90acbc7ce1d0e
// SCE-MAP: policy_scalar_layer.scxml:17 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `PolicyScalarLayer`, a file of the
// module that calls it: a call is qualified by the enum
// (`PolicyScalarLayer.policyScalarLayer(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

public enum PolicyScalarLayer {
    public static func policyScalarLayer(_ base: PolicyScalarSettingsPayload, _ over: PolicyScalarSettingsPayload, _ mask: UInt64) -> PolicyScalarSettingsPayload {
        let out: PolicyScalarSettingsPayload = PolicyScalarSettingsPayload(strict: over.strict || base.strict, limitDays: (mask & 1) != 0 ? over.limitDays : base.limitDays, burst: (mask & 2) != 0 ? over.burst : base.burst, locked: (mask & 4) != 0 ? over.locked : base.locked)
        return out
    }
}
