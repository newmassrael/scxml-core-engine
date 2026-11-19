// SCE-GENERATED — DO NOT EDIT
// source-hash: a58f6624548b520a5ba51086e8783c160d80754eb4040e27c0302b3da2a2c079
// source: outbound_overflows.scxml
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
