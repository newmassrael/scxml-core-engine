// SCE-GENERATED — DO NOT EDIT
// source-hash: 3d8d9ae1c55722e19d45549613b03a3797bee9bb634dcef2850b979bfbe85f5e
// SCE-MAP: precondition.scxml:40 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `Precondition`, a file of the
// module that calls it: a call is qualified by the enum
// (`Precondition.precondition(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum Precondition {
    public static func precondition(_ exists: Bool, _ readOnly: Bool, _ ifMatch: UInt8, _ ifUnmodifiedSince: UInt8, _ ifNoneMatch: UInt8, _ ifModifiedSince: UInt8) throws -> UInt16 {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        do {
            let sceCond2 = !(ifMatch <= 3 && ifUnmodifiedSince <= 2 && ifNoneMatch <= 3 && ifModifiedSince <= 2)
            try sceFailure.throwIfFailed()
            if sceCond2 {
                throw AlgorithmError.precondition
            }
        }
        do {
            let sceCond2 = ifMatch != 0
            try sceFailure.throwIfFailed()
            if sceCond2 {
                do {
                    let sceCond4 = !exists || ifMatch == 3
                    try sceFailure.throwIfFailed()
                    if sceCond4 {
                        do {
                            let sceReturned: UInt16 = 412
                            try sceFailure.throwIfFailed()
                            return sceReturned
                        }
                    }
                }
            } else {
                do {
                    let sceCond4 = ifUnmodifiedSince == 2
                    try sceFailure.throwIfFailed()
                    if sceCond4 {
                        do {
                            let sceReturned: UInt16 = 412
                            try sceFailure.throwIfFailed()
                            return sceReturned
                        }
                    }
                }
            }
        }
        do {
            let sceCond2 = ifNoneMatch != 0
            try sceFailure.throwIfFailed()
            if sceCond2 {
                do {
                    let sceCond4 = ifNoneMatch == 1 && exists || ifNoneMatch == 2
                    try sceFailure.throwIfFailed()
                    if sceCond4 {
                        do {
                            let sceReturned: UInt16 = readOnly ? 304 : 412
                            try sceFailure.throwIfFailed()
                            return sceReturned
                        }
                    }
                }
            } else {
                do {
                    let sceCond4 = readOnly && ifModifiedSince == 2
                    try sceFailure.throwIfFailed()
                    if sceCond4 {
                        do {
                            let sceReturned: UInt16 = 304
                            try sceFailure.throwIfFailed()
                            return sceReturned
                        }
                    }
                }
            }
        }
        do {
            let sceReturned: UInt16 = 0
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
