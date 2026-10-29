// SCE-GENERATED — DO NOT EDIT
// source-hash: b3df842d016676bf8989832cb164de51027fadf6ca9e7b5543c7bac0c9a13d1a
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
    public static func sheetApply(_ op: UInt8, _ a: Int64, _ b: Int64) throws -> SheetValuePayload {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        let top: Int64 = 9223372036854775807
        try sceFailure.throwIfFailed()
        let bottom: Int64 = Checked.sub(sceFailure, Checked.neg(sceFailure, top), 1)
        try sceFailure.throwIfFailed()
        var value: Int64 = 0
        try sceFailure.throwIfFailed()
        var fault: UInt8 = 0
        try sceFailure.throwIfFailed()
        do {
            let sceCond2 = op == 2
            try sceFailure.throwIfFailed()
            if sceCond2 {
                do {
                    let sceCond4 = b > 0 && a > Checked.sub(sceFailure, top, b) || b < 0 && a < Checked.sub(sceFailure, bottom, b)
                    try sceFailure.throwIfFailed()
                    if sceCond4 {
                        fault = 3
                        try sceFailure.throwIfFailed()
                    } else {
                        value = Checked.add(sceFailure, a, b)
                        try sceFailure.throwIfFailed()
                    }
                }
            } else {
                do {
                    let sceCond4 = op == 3
                    try sceFailure.throwIfFailed()
                    if sceCond4 {
                        do {
                            let sceCond6 = b < 0 && a > Checked.add(sceFailure, top, b) || b > 0 && a < Checked.add(sceFailure, bottom, b)
                            try sceFailure.throwIfFailed()
                            if sceCond6 {
                                fault = 3
                                try sceFailure.throwIfFailed()
                            } else {
                                value = Checked.sub(sceFailure, a, b)
                                try sceFailure.throwIfFailed()
                            }
                        }
                    } else {
                        do {
                            let sceCond6 = op == 4
                            try sceFailure.throwIfFailed()
                            if sceCond6 {
                                do {
                                    let sceCond8 = a == 0 || b == 0
                                    try sceFailure.throwIfFailed()
                                    if sceCond8 {
                                        value = 0
                                        try sceFailure.throwIfFailed()
                                    } else {
                                        do {
                                            let sceCond10 = a > 0 && b > 0 && a > Checked.div(sceFailure, top, b) || a > 0 && b < 0 && b < Checked.div(sceFailure, bottom, a) || a < 0 && b > 0 && a < Checked.div(sceFailure, bottom, b) || a < 0 && b < 0 && b < Checked.div(sceFailure, top, a)
                                            try sceFailure.throwIfFailed()
                                            if sceCond10 {
                                                fault = 3
                                                try sceFailure.throwIfFailed()
                                            } else {
                                                value = Checked.mul(sceFailure, a, b)
                                                try sceFailure.throwIfFailed()
                                            }
                                        }
                                    }
                                }
                            } else {
                                do {
                                    let sceCond8 = op == 5
                                    try sceFailure.throwIfFailed()
                                    if sceCond8 {
                                        do {
                                            let sceCond10 = b == 0
                                            try sceFailure.throwIfFailed()
                                            if sceCond10 {
                                                fault = 2
                                                try sceFailure.throwIfFailed()
                                            } else {
                                                do {
                                                    let sceCond12 = a == bottom && b == -1
                                                    try sceFailure.throwIfFailed()
                                                    if sceCond12 {
                                                        fault = 3
                                                        try sceFailure.throwIfFailed()
                                                    } else {
                                                        value = Checked.div(sceFailure, a, b)
                                                        try sceFailure.throwIfFailed()
                                                    }
                                                }
                                            }
                                        }
                                    } else {
                                        fault = 5
                                        try sceFailure.throwIfFailed()
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        let result: SheetValuePayload = SheetValuePayload(value: value, fault: fault)
        try sceFailure.throwIfFailed()
        do {
            let sceReturned: SheetValuePayload = result
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
