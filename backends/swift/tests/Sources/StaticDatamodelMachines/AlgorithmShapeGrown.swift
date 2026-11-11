// SCE-GENERATED — DO NOT EDIT
// source-hash: db4336c6a9bf789bb8f0ab985d2bf7ad97273b841d99ca0c54e9bd1050729ae4
// SCE-MAP: algorithm_shape_grown.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `AlgorithmShapeGrown`, a file of the
// module that calls it: a call is qualified by the enum
// (`AlgorithmShapeGrown.algorithmShapeGrown(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

public enum AlgorithmShapeGrown {
    public static func algorithmShapeGrown(_ shape: UnionShapeMinimal, _ radius: UInt16) -> UnionShapeMinimal {
        if shape.kind != ShapeKind.dot {
            return shape
        }
        let grown: UnionShapeMinimal = UnionShapeMinimal.circle(ShapeRadiusMinimalPayload(radius: radius))
        return grown
    }
}
