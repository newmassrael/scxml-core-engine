// SCE-GENERATED — DO NOT EDIT
// source-hash: f6c860c2693fcb3366bc871ff801c9b0ee9e491a1759830f8d83238267cc3b45
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
