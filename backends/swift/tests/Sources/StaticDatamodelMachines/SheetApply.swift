// SCE-GENERATED — DO NOT EDIT
// source-hash: fb4f42a34e7ac40a28d8e5daee5d633c3c45dddac513816edf2bb83fdc85f91c
// SCE-MAP: sheet_apply.scxml:21 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `SheetApply`, a file of the
// module that calls it: a call is qualified by the enum
// (`SheetApply.sheetApply(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum SheetApply {
    public static func sheetApply(_ op: UInt8, _ a: Int64, _ b: Int64) throws(AlgorithmError) -> SheetValuePayload {
        let top: Int64 = 9223372036854775807
        let bottom: Int64 = (try Checked.sub((try Checked.neg(top)), 1))
        var value: Int64 = 0
        var fault: UInt8 = 0
        if op == 2 {
            if (try Checked.or((try Checked.and(b > 0, { () throws(AlgorithmError) -> Bool in a > (try Checked.sub(top, b)) })), { () throws(AlgorithmError) -> Bool in (try Checked.and(b < 0, { () throws(AlgorithmError) -> Bool in a < (try Checked.sub(bottom, b)) })) })) {
                fault = 3
            } else {
                value = (try Checked.add(a, b))
            }
        } else {
            if op == 3 {
                if (try Checked.or((try Checked.and(b < 0, { () throws(AlgorithmError) -> Bool in a > (try Checked.add(top, b)) })), { () throws(AlgorithmError) -> Bool in (try Checked.and(b > 0, { () throws(AlgorithmError) -> Bool in a < (try Checked.add(bottom, b)) })) })) {
                    fault = 3
                } else {
                    value = (try Checked.sub(a, b))
                }
            } else {
                if op == 4 {
                    if a == 0 || b == 0 {
                        value = 0
                    } else {
                        if (try Checked.or((try Checked.or((try Checked.or((try Checked.and(a > 0 && b > 0, { () throws(AlgorithmError) -> Bool in a > (try Checked.div(top, b)) })), { () throws(AlgorithmError) -> Bool in (try Checked.and(a > 0 && b < 0, { () throws(AlgorithmError) -> Bool in b < (try Checked.div(bottom, a)) })) })), { () throws(AlgorithmError) -> Bool in (try Checked.and(a < 0 && b > 0, { () throws(AlgorithmError) -> Bool in a < (try Checked.div(bottom, b)) })) })), { () throws(AlgorithmError) -> Bool in (try Checked.and(a < 0 && b < 0, { () throws(AlgorithmError) -> Bool in b < (try Checked.div(top, a)) })) })) {
                            fault = 3
                        } else {
                            value = (try Checked.mul(a, b))
                        }
                    }
                } else {
                    if op == 5 {
                        if b == 0 {
                            fault = 2
                        } else {
                            if a == bottom && b == -1 {
                                fault = 3
                            } else {
                                value = (try Checked.div(a, b))
                            }
                        }
                    } else {
                        fault = 5
                    }
                }
            }
        }
        let result: SheetValuePayload = SheetValuePayload(value: value, fault: fault)
        return result
    }
}
