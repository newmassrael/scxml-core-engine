// SCE-GENERATED — DO NOT EDIT
// source-hash: 62067cbe5198ad923571ee5243de08cf004a9a483bf54ae3e6db2a14e8ad77a7
// SCE-MAP: lookup_unit_scale.scxml:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="lookup")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// A table read by exact match: a static function of the caseless enum `LookupUnitScale`, a
// file of the module that calls it. A call is qualified by the enum
// (`LookupUnitScale.lookupScale(…)`), which a member of the calling type cannot hide as
// it would hide a free function of the same name. A key the table
// does not list answers no value: the call is an error, and the function answers `nil`.
import SceForgeRuntime

public enum LookupUnitScale {
    private static let sceKeys: [Int32] = [1, 2, 3, 4, 5, 6]
    private static let sceValues: [Double] = [0.001, 0.01, 0.1, 1.0, 10.0, 100.0]

    public static func lookupScale(_ unit: Int32) -> Double? {
        SceForgeRuntime.Lookup.find(sceKeys, sceValues, unit)
    }
}
