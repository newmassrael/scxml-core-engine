// SCE-GENERATED — DO NOT EDIT
// source-hash: d947250ac8dbe681be6c82964fb8feadd2bb3cbd6b46adffc9d553a7fc0e7ce6
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
