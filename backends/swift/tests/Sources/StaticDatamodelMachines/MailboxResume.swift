// SCE-GENERATED — DO NOT EDIT
// source-hash: fbf6d74eed288a955e76ccdcaaf966f2b277b20786c37dd95086908e29bc362c
// source: sce:std/sync/mailbox_resume.scxml
// SCE-MAP: mailbox_resume.scxml:35 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `MailboxResume`, a file of the
// module that calls it: a call is qualified by the enum
// (`MailboxResume.mailboxResume(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum MailboxResume {
    public static func mailboxResume(_ restored: Int64, _ held: [Int64], _ cursors: [Int64]) throws(AlgorithmError) -> Int64 {
        if !(restored >= 0) {
            throw AlgorithmError.precondition
        }
        var next: Int64 = restored
        var i: UInt32 = 0
        while i < UInt32(truncatingIfNeeded: held.count) {
            if !((try Checked.at(held, i)) >= 0) {
                throw AlgorithmError.precondition
            }
            if (try Checked.at(held, i)) > next {
                next = (try Checked.at(held, i))
            }
            i = (try Checked.add(i, 1))
        }
        i = 0
        while i < UInt32(truncatingIfNeeded: cursors.count) {
            if !((try Checked.at(cursors, i)) >= 0) {
                throw AlgorithmError.precondition
            }
            if (try Checked.at(cursors, i)) > next {
                next = (try Checked.at(cursors, i))
            }
            i = (try Checked.add(i, 1))
        }
        if !(next < 9223372036854775807) {
            throw AlgorithmError.precondition
        }
        return next
    }
}
