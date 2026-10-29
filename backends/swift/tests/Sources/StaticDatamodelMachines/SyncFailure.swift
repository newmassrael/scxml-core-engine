// SCE-GENERATED — DO NOT EDIT
// source-hash: 1e96d54a1b4f9666059e38585db03a53d8ce664a6f48e0045468eeffc6fb717b
// SCE-MAP: sync_failure.scxml:30 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `SyncFailure`, a file of the
// module that calls it: a call is qualified by the enum
// (`SyncFailure.syncFailure(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum SyncFailure {
    public static func syncFailure(_ kind: UInt8, _ status: Int32) throws -> UInt8 {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        do {
            let sceCond2 = !(kind >= 1 && kind <= 9)
            try sceFailure.throwIfFailed()
            if sceCond2 {
                throw AlgorithmError.precondition
            }
        }
        do {
            let sceCond2 = !(kind == 4 ? status >= 300 && status <= 599 : status == 0)
            try sceFailure.throwIfFailed()
            if sceCond2 {
                throw AlgorithmError.precondition
            }
        }
        var action: UInt8 = 2
        try sceFailure.throwIfFailed()
        do {
            let sceCond2 = kind == 8 || kind == 1 || kind == 2
            try sceFailure.throwIfFailed()
            if sceCond2 {
                action = 1
                try sceFailure.throwIfFailed()
            }
        }
        do {
            let sceCond2 = kind == 7
            try sceFailure.throwIfFailed()
            if sceCond2 {
                action = 4
                try sceFailure.throwIfFailed()
            }
        }
        do {
            let sceCond2 = kind == 3
            try sceFailure.throwIfFailed()
            if sceCond2 {
                action = 0
                try sceFailure.throwIfFailed()
            }
        }
        do {
            let sceCond2 = kind == 4 && status == 401
            try sceFailure.throwIfFailed()
            if sceCond2 {
                action = 3
                try sceFailure.throwIfFailed()
            }
        }
        do {
            let sceCond2 = kind == 4 && (status == 503 || status == 502)
            try sceFailure.throwIfFailed()
            if sceCond2 {
                action = 1
                try sceFailure.throwIfFailed()
            }
        }
        do {
            let sceReturned: UInt8 = action
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
