// SCE-GENERATED — DO NOT EDIT
// source-hash: c59281ae7be4758cbffb01ac828a80a4489cab0ae77d7577e991e823091b27bc
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
    public static func aclMembership(_ edges: [AclEdgePayload], _ user: UInt32) throws(AlgorithmError) -> [UInt32] {
        var found: [UInt32] = []
        found.reserveCapacity(256)
        var next: UInt32 = 0
        var current: UInt32 = user
        var more: Bool = user != 0
        while more {
            for e in edges {
                if e.member == current && e.group != user && e.group != 0 {
                    var seen: Bool = false
                    var k: UInt32 = 0
                    while k < UInt32(truncatingIfNeeded: found.count) {
                        if (try Checked.at(found, k)) == e.group {
                            seen = true
                        }
                        k = (try Checked.add(k, 1))
                    }
                    if !seen {
                        found.append(e.group)
                    }
                }
            }
            more = next < UInt32(truncatingIfNeeded: found.count)
            if more {
                current = (try Checked.at(found, next))
                next = (try Checked.add(next, 1))
            }
        }
        return found
    }
}
