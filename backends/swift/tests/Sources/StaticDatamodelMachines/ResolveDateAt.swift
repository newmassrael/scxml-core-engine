// SCE-GENERATED — DO NOT EDIT
// source-hash: 16f3a724053df912275fd9b4ed5c348006be418a891ade10424991dc22444360
// source: sce:std/time/resolve_date_at.scxml
// SCE-MAP: resolve_date_at.scxml:25 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `ResolveDateAt`, a file of the
// module that calls it: a call is qualified by the enum
// (`ResolveDateAt.resolveDateAt(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum ResolveDateAt {
    public static func resolveDateAt(_ today: Int64, _ expressions: [DateExpression], _ at: UInt32) throws(AlgorithmError) -> Int64? {
        let picked: DateExpression = (try Checked.at(expressions, at))
        return (try ResolveDate.resolveDate(today, picked))
    }
}
