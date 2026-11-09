// SCE-GENERATED — DO NOT EDIT
// source-hash: d05ce240c5f3b48b3c83b6bee1ba58feef40d57c310e9a29ef5279604bbaaa42
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
