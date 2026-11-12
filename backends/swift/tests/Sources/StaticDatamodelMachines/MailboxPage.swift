// SCE-GENERATED — DO NOT EDIT
// source-hash: 9e46d5ba618e2f5a559e7793b0e0ddd1d7ba8428fd3d96128f06266cf4750b56
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
    public static func mailboxPage(_ queue: [MailboxSlotPayload], _ after: Int64, _ limit: UInt32) throws(AlgorithmError) -> [MailboxSlotPayload] {
        if !(limit >= 1 && limit <= 256) {
            throw AlgorithmError.precondition
        }
        var out: [MailboxSlotPayload] = []
        out.reserveCapacity(256)
        var taken: UInt32 = 0
        var prev: Int64 = 0
        for s in queue {
            if !(s.id > prev) {
                throw AlgorithmError.precondition
            }
            prev = s.id
            if s.id > after && taken < limit {
                out.append(s)
                taken = (try Checked.add(taken, 1))
            }
        }
        return out
    }
}
