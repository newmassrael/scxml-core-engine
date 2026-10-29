// SCE-GENERATED — DO NOT EDIT
// source-hash: cabd60d6426d729773396692f3fde7a45257e44f0e01ae8a7f61990f6a4ec86b
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
    public static func aclGranted(_ tree: [AclPrivilegePayload], _ acl: [AclAcePayload], _ user: UInt32, _ groups: [UInt32], _ owner: UInt32) throws -> [UInt32] {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        var out: [UInt32] = []
        out.reserveCapacity(256)
        try sceFailure.throwIfFailed()
        var n: UInt32 = 0
        try sceFailure.throwIfFailed()
        for p in tree {
            do {
                let sceCond3 = !(n == 0 ? p.id == 0 && p.parent == 0 : p.id != 0)
                try sceFailure.throwIfFailed()
                if sceCond3 {
                    throw AlgorithmError.precondition
                }
            }
            var parentEarlier: Bool = n == 0
            try sceFailure.throwIfFailed()
            var k: UInt32 = 0
            try sceFailure.throwIfFailed()
            for q in tree {
                do {
                    let sceCond4 = k < n && q.id == p.parent
                    try sceFailure.throwIfFailed()
                    if sceCond4 {
                        parentEarlier = true
                        try sceFailure.throwIfFailed()
                    }
                }
                do {
                    let sceCond4 = k < n
                    try sceFailure.throwIfFailed()
                    if sceCond4 {
                        do {
                            let sceCond6 = !(q.id != p.id)
                            try sceFailure.throwIfFailed()
                            if sceCond6 {
                                throw AlgorithmError.precondition
                            }
                        }
                    }
                }
                k = Checked.add(sceFailure, k, 1)
                try sceFailure.throwIfFailed()
            }
            do {
                let sceCond3 = !(parentEarlier)
                try sceFailure.throwIfFailed()
                if sceCond3 {
                    throw AlgorithmError.precondition
                }
            }
            var at: UInt32 = p.id
            try sceFailure.throwIfFailed()
            var up: UInt32 = p.parent
            try sceFailure.throwIfFailed()
            var granted: Bool = false
            try sceFailure.throwIfFailed()
            var climbing: Bool = true
            try sceFailure.throwIfFailed()
            var steps: UInt32 = 0
            try sceFailure.throwIfFailed()
            while true {
                let sceCond3 = climbing
                try sceFailure.throwIfFailed()
                if !sceCond3 { break }
                for a in acl {
                    do {
                        let sceCond5 = a.privilege == at
                        try sceFailure.throwIfFailed()
                        if sceCond5 {
                            var applies: Bool = a.principal == 4294967295 || (a.principal == 4294967294 ? user != 0 : (a.principal == 4294967293 ? user == 0 : false))
                            try sceFailure.throwIfFailed()
                            let who: UInt32 = a.principal == 4294967292 ? owner : a.principal
                            try sceFailure.throwIfFailed()
                            do {
                                let sceCond7 = who != 0 && user != 0 && who < 4294967292
                                try sceFailure.throwIfFailed()
                                if sceCond7 {
                                    do {
                                        let sceCond9 = who == user
                                        try sceFailure.throwIfFailed()
                                        if sceCond9 {
                                            applies = true
                                            try sceFailure.throwIfFailed()
                                        }
                                    }
                                    for g in groups {
                                        do {
                                            let sceCond10 = g == who
                                            try sceFailure.throwIfFailed()
                                            if sceCond10 {
                                                applies = true
                                                try sceFailure.throwIfFailed()
                                            }
                                        }
                                    }
                                }
                            }
                            do {
                                let sceCond7 = applies
                                try sceFailure.throwIfFailed()
                                if sceCond7 {
                                    granted = true
                                    try sceFailure.throwIfFailed()
                                }
                            }
                        }
                    }
                }
                climbing = !granted && at != 0 && steps < n
                try sceFailure.throwIfFailed()
                do {
                    let sceCond4 = climbing
                    try sceFailure.throwIfFailed()
                    if sceCond4 {
                        at = up
                        try sceFailure.throwIfFailed()
                        for r in tree {
                            do {
                                let sceCond7 = r.id == at
                                try sceFailure.throwIfFailed()
                                if sceCond7 {
                                    up = r.parent
                                    try sceFailure.throwIfFailed()
                                }
                            }
                        }
                        steps = Checked.add(sceFailure, steps, 1)
                        try sceFailure.throwIfFailed()
                    }
                }
            }
            do {
                let sceCond3 = granted
                try sceFailure.throwIfFailed()
                if sceCond3 {
                    out.append(p.id)
                    try sceFailure.throwIfFailed()
                }
            }
            n = Checked.add(sceFailure, n, 1)
            try sceFailure.throwIfFailed()
        }
        do {
            let sceReturned: [UInt32] = out
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
