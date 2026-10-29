// SCE-GENERATED — DO NOT EDIT
// source-hash: 68d77b5e0c96e361239e1512887ba2ef6d35bfab0188bf0e2afbd53ca387f3bc
// SCE-MAP: utc_from_local.scxml:30 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `UtcFromLocal`, a file of the
// module that calls it: a call is qualified by the enum
// (`UtcFromLocal.utcFromLocal(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum UtcFromLocal {
    public static func utcFromLocal(_ starts: [Int64], _ offsets: [Int32], _ local: Int64) throws -> Int64 {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        do {
            let sceCond2 = !(starts.count == offsets.count && starts.count > 0)
            try sceFailure.throwIfFailed()
            if sceCond2 {
                throw AlgorithmError.precondition
            }
        }
        do {
            let sceCond2 = !(Checked.sub(sceFailure, local, Int64(Checked.at(sceFailure, offsets, 0, fallback: 0))) >= Checked.at(sceFailure, starts, 0, fallback: 0))
            try sceFailure.throwIfFailed()
            if sceCond2 {
                throw AlgorithmError.precondition
            }
        }
        let n: UInt32 = UInt32(truncatingIfNeeded: starts.count)
        try sceFailure.throwIfFailed()
        var k: UInt32 = 0
        try sceFailure.throwIfFailed()
        while true {
            let sceCond2 = k < n
            try sceFailure.throwIfFailed()
            if !sceCond2 { break }
            do {
                let sceCond3 = k > 0
                try sceFailure.throwIfFailed()
                if sceCond3 {
                    do {
                        let sceCond5 = !(Checked.at(sceFailure, starts, k, fallback: 0) > Checked.at(sceFailure, starts, Checked.sub(sceFailure, k, 1), fallback: 0))
                        try sceFailure.throwIfFailed()
                        if sceCond5 {
                            throw AlgorithmError.precondition
                        }
                    }
                }
            }
            let u: Int64 = Checked.sub(sceFailure, local, Int64(Checked.at(sceFailure, offsets, k, fallback: 0)))
            try sceFailure.throwIfFailed()
            do {
                let sceCond3 = u >= Checked.at(sceFailure, starts, k, fallback: 0) && (Checked.add(sceFailure, k, 1) == n || u < Checked.at(sceFailure, starts, Checked.add(sceFailure, k, 1), fallback: 0))
                try sceFailure.throwIfFailed()
                if sceCond3 {
                    do {
                        let sceReturned: Int64 = u
                        try sceFailure.throwIfFailed()
                        return sceReturned
                    }
                }
            }
            k = Checked.add(sceFailure, k, 1)
            try sceFailure.throwIfFailed()
        }
        var g: UInt32 = 1
        try sceFailure.throwIfFailed()
        while true {
            let sceCond2 = g < n
            try sceFailure.throwIfFailed()
            if !sceCond2 { break }
            let before: Int64 = Checked.sub(sceFailure, local, Int64(Checked.at(sceFailure, offsets, Checked.sub(sceFailure, g, 1), fallback: 0)))
            try sceFailure.throwIfFailed()
            do {
                let sceCond3 = before >= Checked.at(sceFailure, starts, g, fallback: 0) && Checked.sub(sceFailure, local, Int64(Checked.at(sceFailure, offsets, g, fallback: 0))) < Checked.at(sceFailure, starts, g, fallback: 0)
                try sceFailure.throwIfFailed()
                if sceCond3 {
                    do {
                        let sceReturned: Int64 = before
                        try sceFailure.throwIfFailed()
                        return sceReturned
                    }
                }
            }
            g = Checked.add(sceFailure, g, 1)
            try sceFailure.throwIfFailed()
        }
        do {
            let sceCond2 = !(false)
            try sceFailure.throwIfFailed()
            if sceCond2 {
                throw AlgorithmError.precondition
            }
        }
        do {
            let sceReturned: Int64 = 0
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
