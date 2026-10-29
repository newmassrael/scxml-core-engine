// SCE-GENERATED — DO NOT EDIT
// source-hash: cabd60d6426d729773396692f3fde7a45257e44f0e01ae8a7f61990f6a4ec86b
// SCE-MAP: acl_membership.scxml:23 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `AclMembership`, a file of the
// module that calls it: a call is qualified by the enum
// (`AclMembership.aclMembership(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum AclMembership {
    public static func aclMembership(_ edges: [AclEdgePayload], _ user: UInt32) throws -> [UInt32] {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        var found: [UInt32] = []
        found.reserveCapacity(256)
        try sceFailure.throwIfFailed()
        var next: UInt32 = 0
        try sceFailure.throwIfFailed()
        var current: UInt32 = user
        try sceFailure.throwIfFailed()
        var more: Bool = user != 0
        try sceFailure.throwIfFailed()
        while true {
            let sceCond2 = more
            try sceFailure.throwIfFailed()
            if !sceCond2 { break }
            for e in edges {
                do {
                    let sceCond4 = e.member == current && e.group != user && e.group != 0
                    try sceFailure.throwIfFailed()
                    if sceCond4 {
                        var seen: Bool = false
                        try sceFailure.throwIfFailed()
                        var k: UInt32 = 0
                        try sceFailure.throwIfFailed()
                        while true {
                            let sceCond6 = k < UInt32(truncatingIfNeeded: found.count)
                            try sceFailure.throwIfFailed()
                            if !sceCond6 { break }
                            do {
                                let sceCond7 = Checked.at(sceFailure, found, k, fallback: 0) == e.group
                                try sceFailure.throwIfFailed()
                                if sceCond7 {
                                    seen = true
                                    try sceFailure.throwIfFailed()
                                }
                            }
                            k = Checked.add(sceFailure, k, 1)
                            try sceFailure.throwIfFailed()
                        }
                        do {
                            let sceCond6 = !seen
                            try sceFailure.throwIfFailed()
                            if sceCond6 {
                                found.append(e.group)
                                try sceFailure.throwIfFailed()
                            }
                        }
                    }
                }
            }
            more = next < UInt32(truncatingIfNeeded: found.count)
            try sceFailure.throwIfFailed()
            do {
                let sceCond3 = more
                try sceFailure.throwIfFailed()
                if sceCond3 {
                    current = Checked.at(sceFailure, found, next, fallback: 0)
                    try sceFailure.throwIfFailed()
                    next = Checked.add(sceFailure, next, 1)
                    try sceFailure.throwIfFailed()
                }
            }
        }
        do {
            let sceReturned: [UInt32] = found
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
