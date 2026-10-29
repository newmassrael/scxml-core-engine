// SCE-GENERATED — DO NOT EDIT
// source-hash: 1e96d54a1b4f9666059e38585db03a53d8ce664a6f48e0045468eeffc6fb717b
// SCE-MAP: mailbox_page.scxml:23 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `MailboxPage`, a file of the
// module that calls it: a call is qualified by the enum
// (`MailboxPage.mailboxPage(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum MailboxPage {
    public static func mailboxPage(_ queue: [MailboxSlotPayload], _ after: Int64, _ limit: UInt32) throws -> [MailboxSlotPayload] {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        do {
            let sceCond2 = !(limit >= 1 && limit <= 256)
            try sceFailure.throwIfFailed()
            if sceCond2 {
                throw AlgorithmError.precondition
            }
        }
        var out: [MailboxSlotPayload] = []
        out.reserveCapacity(256)
        try sceFailure.throwIfFailed()
        var taken: UInt32 = 0
        try sceFailure.throwIfFailed()
        var prev: Int64 = 0
        try sceFailure.throwIfFailed()
        for s in queue {
            do {
                let sceCond3 = !(s.id > prev)
                try sceFailure.throwIfFailed()
                if sceCond3 {
                    throw AlgorithmError.precondition
                }
            }
            prev = s.id
            try sceFailure.throwIfFailed()
            do {
                let sceCond3 = s.id > after && taken < limit
                try sceFailure.throwIfFailed()
                if sceCond3 {
                    out.append(s)
                    try sceFailure.throwIfFailed()
                    taken = Checked.add(sceFailure, taken, 1)
                    try sceFailure.throwIfFailed()
                }
            }
        }
        do {
            let sceReturned: [MailboxSlotPayload] = out
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
