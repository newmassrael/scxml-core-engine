// SCE-GENERATED — DO NOT EDIT
// source-hash: 52868da22c06561e2c560050fc709417107e0518328e8ceb0790994e53be08ca
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
