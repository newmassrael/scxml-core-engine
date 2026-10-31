// SCE-GENERATED — DO NOT EDIT
// source-hash: be2e66b7d73b7a0417f2cd73d69b4e29857f6dc165f59dbce69d4cc4daced358
// SCE-MAP: lookup_alarm_code.scxml:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="lookup")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// A table read by exact match: a static function of the caseless enum `LookupAlarmCode`, a
// file of the module that calls it. A call is qualified by the enum
// (`LookupAlarmCode.lookupSeverity(…)`), which a member of the calling type cannot hide as
// it would hide a free function of the same name. A key the table
// does not list answers no value: the call is an error, and the function answers `nil`.
import SceForgeRuntime

public enum LookupAlarmCode {
    private static let sceKeys: [Int32] = [100, 200, 300, 400, 500]
    private static let sceValues: [Int32] = [1, 2, 3, 2, 4]

    public static func lookupSeverity(_ code: Int32) -> Int32? {
        SceForgeRuntime.Lookup.find(sceKeys, sceValues, code)
    }
}
