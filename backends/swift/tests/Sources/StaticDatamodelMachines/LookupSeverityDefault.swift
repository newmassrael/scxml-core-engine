// SCE-GENERATED — DO NOT EDIT
// source-hash: adac8824ce3bf12969cf8a9f4a1db094aa59b5b7a2d7b37b7a611f3c7eb217dc
// SCE-MAP: lookup_severity_default.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="lookup")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// A table read by exact match: a static function of the caseless enum `LookupSeverityDefault`, a
// file of the module that calls it. A call is qualified by the enum
// (`LookupSeverityDefault.lookupSeverity(…)`), which a member of the calling type cannot hide as
// it would hide a free function of the same name.
import SceForgeRuntime

public enum LookupSeverityDefault {
    private static let sceKeys: [Int32] = [100, 200, 300, 400, 500]
    private static let sceValues: [Int32] = [1, 2, 3, 2, 4]

    public static func lookupSeverity(_ code: Int32) -> Int32 {
        SceForgeRuntime.Lookup.find(sceKeys, sceValues, code) ?? 0
    }
}
