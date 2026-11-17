// SCE-GENERATED — DO NOT EDIT
// source-hash: a86b3b19a12dab1982345f4a46c1e37355ff902583da2c2f5d8395ee9c56f9e7
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
    public static func mailboxAssign(_ queue: [MailboxSlotPayload], _ counter: Int64, _ hi: UInt64, _ lo: UInt64) throws(AlgorithmError) -> Int64 {
        if !(counter >= 0 && counter < 9223372036854775807) {
            throw AlgorithmError.precondition
        }
        var id: Int64 = (try Checked.add(counter, 1))
        for s in queue {
            if !(s.id >= 1 && s.id <= counter) {
                throw AlgorithmError.precondition
            }
            if s.hi == hi && s.lo == lo {
                id = s.id
            }
        }
        return id
    }
}
