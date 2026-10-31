// SCE-GENERATED — DO NOT EDIT
// source-hash: 90dfacafb9c3b3dd4ea571704b12a814c54947805373c06b331ad43ea5f220d0
// SCE-MAP: interpolation_1d_linear.scxml:1 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="interpolation")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// A table interpolated between its breakpoints: a static function of the caseless enum
// `Interpolation1dLinear`, a file of the module that calls it. A call is qualified by the enum
// (`Interpolation1dLinear.lookup(…)`), which a member of the calling type cannot hide as
// it would hide a free function of the same name. An input outside an axis answers the value at
// the nearer end. The runtime is named by its module, so a document named for one of its types
// cannot hide it.

import SceForgeRuntime

public enum Interpolation1dLinear {
    private static let sceAxisRpm: [Double] = [800.0, 1200.0, 2000.0, 3000.0, 4000.0, 6000.0]
    private static let sceValues: [Double] = [120.0, 145.0, 200.0, 230.0, 210.0, 180.0]

    public static func lookup(_ rpm: UInt16) -> Double {
        Double(SceForgeRuntime.Interpolation.linear(
            sceAxisRpm, sceValues,
            Double(rpm)
        ))
    }
}
