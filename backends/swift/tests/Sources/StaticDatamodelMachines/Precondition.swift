// SCE-GENERATED — DO NOT EDIT
// source-hash: 3d8d9ae1c55722e19d45549613b03a3797bee9bb634dcef2850b979bfbe85f5e
// SCE-MAP: precondition.scxml:40 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `Precondition`, a file of the
// module that calls it: a call is qualified by the enum
// (`Precondition.precondition(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum Precondition {
    public static func precondition(_ exists: Bool, _ readOnly: Bool, _ ifMatch: UInt8, _ ifUnmodifiedSince: UInt8, _ ifNoneMatch: UInt8, _ ifModifiedSince: UInt8) throws(AlgorithmError) -> UInt16 {
        if !(ifMatch <= 3 && ifUnmodifiedSince <= 2 && ifNoneMatch <= 3 && ifModifiedSince <= 2) {
            throw AlgorithmError.precondition
        }
        if ifMatch != 0 {
            if !exists || ifMatch == 3 {
                return 412
            }
        } else {
            if ifUnmodifiedSince == 2 {
                return 412
            }
        }
        if ifNoneMatch != 0 {
            if ifNoneMatch == 1 && exists || ifNoneMatch == 2 {
                return readOnly ? 304 : 412
            }
        } else {
            if readOnly && ifModifiedSince == 2 {
                return 304
            }
        }
        return 0
    }
}
