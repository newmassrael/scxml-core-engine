// SCE-GENERATED — DO NOT EDIT
// source-hash: 6b431984688d288798089f16432b8bdf8a4e08b28da85ae7117e434e9ad27889
// SCE-MAP: interpolation_2d_bilinear.scxml:1 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="interpolation")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// A table interpolated between its breakpoints: a static function of the caseless enum
// `Interpolation2dBilinear`, a file of the module that calls it. A call is qualified by the enum
// (`Interpolation2dBilinear.lookup(…)`), which a member of the calling type cannot hide as
// it would hide a free function of the same name. An input outside an axis answers the value at
// the nearer end. The runtime is named by its module, so a document named for one of its types
// cannot hide it.

import SceForgeRuntime

public enum Interpolation2dBilinear {
    private static let sceAxisRpm: [Double] = [800.0, 1200.0, 2000.0, 3000.0]
    private static let sceAxisLoad: [Double] = [10.0, 50.0, 100.0]
    private static let sceValues: [[Double]] = [
        [2.1, 4.5, 7.0],
        [2.5, 5.0, 8.0],
        [3.0, 6.0, 9.5],
        [3.5, 7.0, 11.0]
    ]

    public static func lookup(_ rpm: UInt16, _ load: UInt8) -> Double {
        Double(SceForgeRuntime.Interpolation.bilinear(
            sceAxisRpm, sceAxisLoad, sceValues,
            Double(rpm), Double(load)
        ))
    }
}
