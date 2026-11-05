// SCE-GENERATED — DO NOT EDIT
// source-hash: 756970c374508abdd4d68a996d5fd6f31c978e573ad8a3257b72b5a7e7773666
// SCE-MAP: transform_multi_output.scxml:3 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="transform")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// A formula per output: a static function of the caseless enum `TransformMultiOutput` for each, a
// file of the module that calls them. A call is qualified by the enum
// (`TransformMultiOutput.<function>(…)`), which a member of the calling type cannot hide as it would
// hide a free function of the same name. An output that reads another output calls its function.

public enum TransformMultiOutput {

    public static func computeFahrenheit(_ celsius: Double) -> Double {
        celsius * 9.0 / 5.0 + 32.0
    }

    public static func computeKelvin(_ celsius: Double) -> Double {
        celsius + 273.15
    }
}
