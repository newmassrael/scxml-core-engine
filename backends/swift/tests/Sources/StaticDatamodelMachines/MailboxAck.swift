// SCE-GENERATED — DO NOT EDIT
// source-hash: 5a1aa51ca5a8db7035ce9d7d91cc1a8dbefd495dd1a69b8489d65864a9902e3a
// source: sce:std/sync/mailbox_ack.scxml
// SCE-MAP: mailbox_ack.scxml:19 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `MailboxAck`, a file of the
// module that calls it: a call is qualified by the enum
// (`MailboxAck.mailboxAck(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

public enum MailboxAck {
    public static func mailboxAck(_ queue: [MailboxSlotPayload], _ acked: [EnvelopeIdPayload]) -> [MailboxSlotPayload] {
        var out: [MailboxSlotPayload] = []
        out.reserveCapacity(256)
        for s in queue {
            var gone: Bool = false
            for a in acked {
                if a.hi == s.hi && a.lo == s.lo {
                    gone = true
                }
            }
            if !gone {
                out.append(s)
            }
        }
        return out
    }
}
