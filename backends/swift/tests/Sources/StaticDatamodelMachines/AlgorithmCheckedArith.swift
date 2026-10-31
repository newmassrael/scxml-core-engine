// SCE-GENERATED — DO NOT EDIT
// source-hash: 90dfacafb9c3b3dd4ea571704b12a814c54947805373c06b331ad43ea5f220d0
// SCE-MAP: algorithm_checked_arith.scxml:18 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `AlgorithmCheckedArith`, a file of the
// module that calls it: a call is qualified by the enum
// (`AlgorithmCheckedArith.algorithmCheckedArith(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum AlgorithmCheckedArith {
    public static func algorithmCheckedArith(_ a: Int32, _ b: Int32, _ op: UInt8) throws -> Int32 {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        var r: Int32 = 0
        try sceFailure.throwIfFailed()
        do {
            let sceCond2 = op == 0
            try sceFailure.throwIfFailed()
            if sceCond2 {
                r = Checked.add(sceFailure, a, b)
                try sceFailure.throwIfFailed()
            }
        }
        do {
            let sceCond2 = op == 1
            try sceFailure.throwIfFailed()
            if sceCond2 {
                r = Checked.sub(sceFailure, a, b)
                try sceFailure.throwIfFailed()
            }
        }
        do {
            let sceCond2 = op == 2
            try sceFailure.throwIfFailed()
            if sceCond2 {
                r = Checked.mul(sceFailure, a, b)
                try sceFailure.throwIfFailed()
            }
        }
        do {
            let sceCond2 = op == 3
            try sceFailure.throwIfFailed()
            if sceCond2 {
                r = Checked.div(sceFailure, a, b)
                try sceFailure.throwIfFailed()
            }
        }
        do {
            let sceCond2 = op == 4
            try sceFailure.throwIfFailed()
            if sceCond2 {
                r = Checked.rem(sceFailure, a, b)
                try sceFailure.throwIfFailed()
            }
        }
        do {
            let sceCond2 = op == 5
            try sceFailure.throwIfFailed()
            if sceCond2 {
                r = Checked.neg(sceFailure, a)
                try sceFailure.throwIfFailed()
            }
        }
        do {
            let sceCond2 = op == 6
            try sceFailure.throwIfFailed()
            if sceCond2 {
                r = Int32(Checked.sub(sceFailure, op, 7))
                try sceFailure.throwIfFailed()
            }
        }
        do {
            let sceReturned: Int32 = r
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
