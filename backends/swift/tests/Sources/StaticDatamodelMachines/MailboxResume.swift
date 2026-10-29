// SCE-GENERATED — DO NOT EDIT
// source-hash: 1e96d54a1b4f9666059e38585db03a53d8ce664a6f48e0045468eeffc6fb717b
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
    public static func mailboxResume(_ restored: Int64, _ held: [Int64], _ cursors: [Int64]) throws -> Int64 {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        do {
            let sceCond2 = !(restored >= 0)
            try sceFailure.throwIfFailed()
            if sceCond2 {
                throw AlgorithmError.precondition
            }
        }
        var next: Int64 = restored
        try sceFailure.throwIfFailed()
        var i: UInt32 = 0
        try sceFailure.throwIfFailed()
        while true {
            let sceCond2 = i < UInt32(truncatingIfNeeded: held.count)
            try sceFailure.throwIfFailed()
            if !sceCond2 { break }
            do {
                let sceCond3 = !(Checked.at(sceFailure, held, i, fallback: 0) >= 0)
                try sceFailure.throwIfFailed()
                if sceCond3 {
                    throw AlgorithmError.precondition
                }
            }
            do {
                let sceCond3 = Checked.at(sceFailure, held, i, fallback: 0) > next
                try sceFailure.throwIfFailed()
                if sceCond3 {
                    next = Checked.at(sceFailure, held, i, fallback: 0)
                    try sceFailure.throwIfFailed()
                }
            }
            i = Checked.add(sceFailure, i, 1)
            try sceFailure.throwIfFailed()
        }
        i = 0
        try sceFailure.throwIfFailed()
        while true {
            let sceCond2 = i < UInt32(truncatingIfNeeded: cursors.count)
            try sceFailure.throwIfFailed()
            if !sceCond2 { break }
            do {
                let sceCond3 = !(Checked.at(sceFailure, cursors, i, fallback: 0) >= 0)
                try sceFailure.throwIfFailed()
                if sceCond3 {
                    throw AlgorithmError.precondition
                }
            }
            do {
                let sceCond3 = Checked.at(sceFailure, cursors, i, fallback: 0) > next
                try sceFailure.throwIfFailed()
                if sceCond3 {
                    next = Checked.at(sceFailure, cursors, i, fallback: 0)
                    try sceFailure.throwIfFailed()
                }
            }
            i = Checked.add(sceFailure, i, 1)
            try sceFailure.throwIfFailed()
        }
        do {
            let sceCond2 = !(next < 9223372036854775807)
            try sceFailure.throwIfFailed()
            if sceCond2 {
                throw AlgorithmError.precondition
            }
        }
        do {
            let sceReturned: Int64 = next
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
