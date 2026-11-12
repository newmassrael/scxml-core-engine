// SCE-GENERATED — DO NOT EDIT
// source-hash: adac8824ce3bf12969cf8a9f4a1db094aa59b5b7a2d7b37b7a611f3c7eb217dc
// SCE-MAP: transform_temperature.scxml:3 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="transform")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// A formula per output: a static function of the caseless enum `TransformTemperature` for each, a
// file of the module that calls them. A call is qualified by the enum
// (`TransformTemperature.<function>(…)`), which a member of the calling type cannot hide as it would
// hide a free function of the same name. An output that reads another output calls its function.

public enum TransformTemperature {

    public static func computeTemperature(_ raw: UInt16) -> Double {
        Double(raw) * 0.1 - 40.0
    }
}
