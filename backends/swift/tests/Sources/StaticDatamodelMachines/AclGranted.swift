// SCE-GENERATED — DO NOT EDIT
// source-hash: c59281ae7be4758cbffb01ac828a80a4489cab0ae77d7577e991e823091b27bc
// SCE-MAP: acl_granted.scxml:31 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `AclGranted`, a file of the
// module that calls it: a call is qualified by the enum
// (`AclGranted.aclGranted(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum AclGranted {
    public static func aclGranted(_ tree: [AclPrivilegePayload], _ acl: [AclAcePayload], _ user: UInt32, _ groups: [UInt32], _ owner: UInt32) throws(AlgorithmError) -> [UInt32] {
        var out: [UInt32] = []
        out.reserveCapacity(256)
        var n: UInt32 = 0
        for p in tree {
            if !(n == 0 ? p.id == 0 && p.parent == 0 : p.id != 0) {
                throw AlgorithmError.precondition
            }
            var parentEarlier: Bool = n == 0
            var k: UInt32 = 0
            for q in tree {
                if k < n && q.id == p.parent {
                    parentEarlier = true
                }
                if k < n {
                    if !(q.id != p.id) {
                        throw AlgorithmError.precondition
                    }
                }
                k = (try Checked.add(k, 1))
            }
            if !(parentEarlier) {
                throw AlgorithmError.precondition
            }
            var at: UInt32 = p.id
            var up: UInt32 = p.parent
            var granted: Bool = false
            var climbing: Bool = true
            var steps: UInt32 = 0
            while climbing {
                for a in acl {
                    if a.privilege == at {
                        var applies: Bool = a.principal == 4294967295 || (a.principal == 4294967294 ? user != 0 : (a.principal == 4294967293 ? user == 0 : false))
                        let who: UInt32 = a.principal == 4294967292 ? owner : a.principal
                        if who != 0 && user != 0 && who < 4294967292 {
                            if who == user {
                                applies = true
                            }
                            for g in groups {
                                if g == who {
                                    applies = true
                                }
                            }
                        }
                        if applies {
                            granted = true
                        }
                    }
                }
                climbing = !granted && at != 0 && steps < n
                if climbing {
                    at = up
                    for r in tree {
                        if r.id == at {
                            up = r.parent
                        }
                    }
                    steps = (try Checked.add(steps, 1))
                }
            }
            if granted {
                out.append(p.id)
            }
            n = (try Checked.add(n, 1))
        }
        return out
    }
}
