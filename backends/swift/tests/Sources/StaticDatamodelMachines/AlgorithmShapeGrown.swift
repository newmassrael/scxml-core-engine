// SCE-GENERATED — DO NOT EDIT
// source-hash: 5ba52a313a5f53415f5c7e49a957fa6eb851fa976ae81a41db1540add4319d51
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
