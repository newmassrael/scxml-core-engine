// SCE-GENERATED — DO NOT EDIT
// source-hash: 1e96d54a1b4f9666059e38585db03a53d8ce664a6f48e0045468eeffc6fb717b
// SCE-MAP: sync_upload_outcome.scxml:24 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `SyncUploadOutcome`, a file of the
// module that calls it: a call is qualified by the enum
// (`SyncUploadOutcome.syncUploadOutcome(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum SyncUploadOutcome {
    public static func syncUploadOutcome(_ create: Bool, _ status: Int32, _ davError: Bool) throws -> UInt8 {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        do {
            let sceCond2 = !(status >= 200 && status <= 599)
            try sceFailure.throwIfFailed()
            if sceCond2 {
                throw AlgorithmError.precondition
            }
        }
        var outcome: UInt8 = 2
        try sceFailure.throwIfFailed()
        do {
            let sceCond2 = status < 300
            try sceFailure.throwIfFailed()
            if sceCond2 {
                outcome = 0
                try sceFailure.throwIfFailed()
            }
        }
        do {
            let sceCond2 = status == 403 || status == 412 || status == 409 && davError
            try sceFailure.throwIfFailed()
            if sceCond2 {
                outcome = 1
                try sceFailure.throwIfFailed()
            }
        }
        do {
            let sceCond2 = !create && (status == 404 || status == 410 || status == 409)
            try sceFailure.throwIfFailed()
            if sceCond2 {
                outcome = 1
                try sceFailure.throwIfFailed()
            }
        }
        do {
            let sceReturned: UInt8 = outcome
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
