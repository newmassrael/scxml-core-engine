// SCE-GENERATED — DO NOT EDIT
// source-hash: ad4bedcb756956ce54bdaf3d75bb21306a16abe1845fd50a6c67924ec38392a8
// SCE-MAP: forge_call_limit.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="interpolation")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// A table interpolated between its breakpoints: a static function of the caseless enum
// `ForgeCallLimit`, a file of the module that calls it. A call is qualified by the enum
// (`ForgeCallLimit.lookup(…)`), which a member of the calling type cannot hide as
// it would hide a free function of the same name. An input outside an axis answers the value at
// the nearer end. The runtime is named by its module, so a document named for one of its types
// cannot hide it.

import SceForgeRuntime

public enum ForgeCallLimit {
    private static let sceAxisRpm: [Double] = [0.0, 1000.0, 2000.0]
    private static let sceValues: [Double] = [100.0, 150.0, 120.0]

    public static func lookup(_ rpm: UInt16) -> Double {
        Double(SceForgeRuntime.Interpolation.linear(
            sceAxisRpm, sceValues,
            Double(rpm)
        ))
    }
}
