// SCE-GENERATED — DO NOT EDIT
// source-hash: 6d71812086a998a80412c256f457bab009adc259010083f54f89e8deddbca71a
// SCE-MAP: algorithm_shape_extent.scxml:8 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `AlgorithmShapeExtent`, a file of the
// module that calls it: a call is qualified by the enum
// (`AlgorithmShapeExtent.algorithmShapeExtent(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

public enum AlgorithmShapeExtent {
    public static func algorithmShapeExtent(_ shape: UnionShapeMinimal) -> UInt16 {
        if shape.kind == ShapeKind.dot {
            return 0
        }
        if shape.kind == ShapeKind.circle {
            return shape.provenCircle.radius
        }
        if shape.kind == ShapeKind.rect {
            return shape.provenRect.width < shape.provenRect.height ? shape.provenRect.height : shape.provenRect.width
        }
        preconditionFailure("algorithm_shape_extent: every case of its union returned before the end of its body")
    }
}
