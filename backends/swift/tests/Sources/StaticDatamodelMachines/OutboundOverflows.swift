// SCE-GENERATED — DO NOT EDIT
// source-hash: 9b4cfcaed7ada488617b52a841abc3e598466613b60c7e1131e1820f7a244b47
// SCE-MAP: outbound_overflows.scxml:17 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `OutboundOverflows`, a file of the
// module that calls it: a call is qualified by the enum
// (`OutboundOverflows.outboundOverflows(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

public enum OutboundOverflows {
    public static func outboundOverflows(_ depth: UInt32, _ maxPending: UInt32) -> Bool {
        return depth >= maxPending
    }
}
