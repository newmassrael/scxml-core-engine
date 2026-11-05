// SCE-GENERATED — DO NOT EDIT
// source-hash: dcc96ea8c5321d36e69f038b98535f0793d14bdbecc7d72d9cf5831d9f96378c
// SCE-MAP: forge_call_code.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="lookup")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// A table read by exact match: a static function of the caseless enum `ForgeCallCode`, a
// file of the module that calls it. A call is qualified by the enum
// (`ForgeCallCode.lookupCode(…)`), which a member of the calling type cannot hide as
// it would hide a free function of the same name. A key the table
// does not list answers no value: the call is an error, and the function answers `nil`.
import SceForgeRuntime

public enum ForgeCallCode {
    private static let sceKeys: [UInt8] = [1, 2]
    private static let sceValues: [Int32] = [100, 200]

    public static func lookupCode(_ gear: UInt8) -> Int32? {
        SceForgeRuntime.Lookup.find(sceKeys, sceValues, gear)
    }
}
