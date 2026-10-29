// SCE-GENERATED — DO NOT EDIT
// source-hash: 1e96d54a1b4f9666059e38585db03a53d8ce664a6f48e0045468eeffc6fb717b
// SCE-MAP: mailbox_assign.scxml:29 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `MailboxAssign`, a file of the
// module that calls it: a call is qualified by the enum
// (`MailboxAssign.mailboxAssign(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum MailboxAssign {
    public static func mailboxAssign(_ queue: [MailboxSlotPayload], _ counter: Int64, _ hi: UInt64, _ lo: UInt64) throws -> Int64 {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        do {
            let sceCond2 = !(counter >= 0 && counter < 9223372036854775807)
            try sceFailure.throwIfFailed()
            if sceCond2 {
                throw AlgorithmError.precondition
            }
        }
        var id: Int64 = Checked.add(sceFailure, counter, 1)
        try sceFailure.throwIfFailed()
        for s in queue {
            do {
                let sceCond3 = !(s.id >= 1 && s.id <= counter)
                try sceFailure.throwIfFailed()
                if sceCond3 {
                    throw AlgorithmError.precondition
                }
            }
            do {
                let sceCond3 = s.hi == hi && s.lo == lo
                try sceFailure.throwIfFailed()
                if sceCond3 {
                    id = s.id
                    try sceFailure.throwIfFailed()
                }
            }
        }
        do {
            let sceReturned: Int64 = id
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
