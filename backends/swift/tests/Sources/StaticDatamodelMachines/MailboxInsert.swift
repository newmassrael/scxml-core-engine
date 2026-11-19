// SCE-GENERATED — DO NOT EDIT
// source-hash: eb6c62381db72b88f86990ca657b3a8a71db2353a7229b01b0e65e0a23cb9a93
// source: sce:std/sync/mailbox_insert.scxml
// SCE-MAP: mailbox_insert.scxml:20 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `MailboxInsert`, a file of the
// module that calls it: a call is qualified by the enum
// (`MailboxInsert.mailboxInsert(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum MailboxInsert {
    public static func mailboxInsert(_ queue: [MailboxSlotPayload], _ counter: Int64, _ hi: UInt64, _ lo: UInt64) throws(AlgorithmError) -> [MailboxSlotPayload] {
        if !(counter >= 0 && counter < 9223372036854775807) {
            throw AlgorithmError.precondition
        }
        var out: [MailboxSlotPayload] = []
        out.reserveCapacity(256)
        var held: Bool = false
        var prev: Int64 = 0
        for s in queue {
            if !(s.id > prev && s.id <= counter) {
                throw AlgorithmError.precondition
            }
            prev = s.id
            if s.hi == hi && s.lo == lo {
                held = true
            }
            out.append(s)
        }
        if !held {
            let fresh: MailboxSlotPayload = MailboxSlotPayload(id: (try Checked.add(counter, 1)), hi: hi, lo: lo)
            out.append(fresh)
        }
        return out
    }
}
