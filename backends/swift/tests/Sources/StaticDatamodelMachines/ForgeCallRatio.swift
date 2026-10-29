// SCE-GENERATED — DO NOT EDIT
// source-hash: 1178ba59796ca1f7c46f8569fd4bfda85c2b97db65f51728f3615d990d97dd1c
// SCE-MAP: forge_call_ratio.scxml:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="lookup")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// A table read by exact match: a static function of the caseless enum `ForgeCallRatio`, a
// file of the module that calls it. A call is qualified by the enum
// (`ForgeCallRatio.lookupRatio(…)`), which a member of the calling type cannot hide as
// it would hide a free function of the same name.
import SceForgeRuntime

public enum ForgeCallRatio {
    private static let sceKeys: [UInt8] = [1, 2, 3]
    private static let sceValues: [Int32] = [35, 20, 14]

    public static func lookupRatio(_ gear: UInt8) -> Int32 {
        SceForgeRuntime.Lookup.find(sceKeys, sceValues, gear) ?? 0
    }
}
