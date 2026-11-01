// SCE-GENERATED — DO NOT EDIT
// source-hash: 24f1c455257ea04f598b006a589c3dea642c1ea2e58b2dcf4b49b8cebf3f207e
// SCE-MAP: forge_call_weight.scxml:8 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="lookup")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// A table read by exact match: a static function of the caseless enum `ForgeCallWeight`, a
// file of the module that calls it. A call is qualified by the enum
// (`ForgeCallWeight.lookupWeight(…)`), which a member of the calling type cannot hide as
// it would hide a free function of the same name. A key the table
// does not list answers no value: the call is an error, and the function answers `nil`.
import SceForgeRuntime

public enum ForgeCallWeight {
    private static let sceKeys: [ForgeCallStatus] = [ForgeCallStatus.open, ForgeCallStatus.held]
    private static let sceValues: [ForgeCallLevel] = [ForgeCallLevel.light, ForgeCallLevel.heavy]

    public static func lookupWeight(_ status: ForgeCallStatus) -> ForgeCallLevel? {
        SceForgeRuntime.Lookup.find(sceKeys, sceValues, status)
    }
}
