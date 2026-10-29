// SCE-GENERATED — DO NOT EDIT
// source-hash: f7347a8b98455a65e31ae2b3af12ef65d68a284242bd71a8102ab373827910fd
// SCE-MAP: utf8_length.scxml:56 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `Utf8Length`, a file of the
// module that calls it: a call is qualified by the enum
// (`Utf8Length.utf8Length(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum Utf8Length {
    public static func utf8Length(_ data: [UInt8]) throws -> UInt32 {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        let n: UInt32 = UInt32(truncatingIfNeeded: data.count)
        try sceFailure.throwIfFailed()
        var i: UInt32 = 0
        try sceFailure.throwIfFailed()
        var count: UInt32 = 0
        try sceFailure.throwIfFailed()
        while true {
            let sceCond2 = i < n
            try sceFailure.throwIfFailed()
            if !sceCond2 { break }
            let lead: UInt32 = UInt32(Checked.at(sceFailure, data, i, fallback: 0))
            try sceFailure.throwIfFailed()
            var tails: UInt32 = 0
            try sceFailure.throwIfFailed()
            var lo: UInt32 = 128
            try sceFailure.throwIfFailed()
            var hi: UInt32 = 191
            try sceFailure.throwIfFailed()
            do {
                let sceCond3 = lead <= 127
                try sceFailure.throwIfFailed()
                if sceCond3 {
                    tails = 0
                    try sceFailure.throwIfFailed()
                } else {
                    do {
                        let sceCond5 = lead >= 194 && lead <= 223
                        try sceFailure.throwIfFailed()
                        if sceCond5 {
                            tails = 1
                            try sceFailure.throwIfFailed()
                        } else {
                            do {
                                let sceCond7 = lead == 224
                                try sceFailure.throwIfFailed()
                                if sceCond7 {
                                    tails = 2
                                    try sceFailure.throwIfFailed()
                                    lo = 160
                                    try sceFailure.throwIfFailed()
                                } else {
                                    do {
                                        let sceCond9 = lead == 237
                                        try sceFailure.throwIfFailed()
                                        if sceCond9 {
                                            tails = 2
                                            try sceFailure.throwIfFailed()
                                            hi = 159
                                            try sceFailure.throwIfFailed()
                                        } else {
                                            do {
                                                let sceCond11 = lead >= 225 && lead <= 239
                                                try sceFailure.throwIfFailed()
                                                if sceCond11 {
                                                    tails = 2
                                                    try sceFailure.throwIfFailed()
                                                } else {
                                                    do {
                                                        let sceCond13 = lead == 240
                                                        try sceFailure.throwIfFailed()
                                                        if sceCond13 {
                                                            tails = 3
                                                            try sceFailure.throwIfFailed()
                                                            lo = 144
                                                            try sceFailure.throwIfFailed()
                                                        } else {
                                                            do {
                                                                let sceCond15 = lead == 244
                                                                try sceFailure.throwIfFailed()
                                                                if sceCond15 {
                                                                    tails = 3
                                                                    try sceFailure.throwIfFailed()
                                                                    hi = 143
                                                                    try sceFailure.throwIfFailed()
                                                                } else {
                                                                    do {
                                                                        let sceCond17 = !(lead >= 241 && lead <= 243)
                                                                        try sceFailure.throwIfFailed()
                                                                        if sceCond17 {
                                                                            throw AlgorithmError.precondition
                                                                        }
                                                                    }
                                                                    tails = 3
                                                                    try sceFailure.throwIfFailed()
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            do {
                let sceCond3 = !(tails <= Checked.sub(sceFailure, Checked.sub(sceFailure, n, i), 1))
                try sceFailure.throwIfFailed()
                if sceCond3 {
                    throw AlgorithmError.precondition
                }
            }
            var k: UInt32 = 1
            try sceFailure.throwIfFailed()
            while true {
                let sceCond3 = k <= tails
                try sceFailure.throwIfFailed()
                if !sceCond3 { break }
                let b: UInt32 = UInt32(Checked.at(sceFailure, data, Checked.add(sceFailure, i, k), fallback: 0))
                try sceFailure.throwIfFailed()
                do {
                    let sceCond4 = !(b >= (k == 1 ? lo : 128) && b <= (k == 1 ? hi : 191))
                    try sceFailure.throwIfFailed()
                    if sceCond4 {
                        throw AlgorithmError.precondition
                    }
                }
                k = Checked.add(sceFailure, k, 1)
                try sceFailure.throwIfFailed()
            }
            i = Checked.add(sceFailure, Checked.add(sceFailure, i, tails), 1)
            try sceFailure.throwIfFailed()
            count = Checked.add(sceFailure, count, 1)
            try sceFailure.throwIfFailed()
        }
        do {
            let sceReturned: UInt32 = count
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
