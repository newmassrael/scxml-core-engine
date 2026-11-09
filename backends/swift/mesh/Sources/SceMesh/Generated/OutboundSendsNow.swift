// SCE-GENERATED — DO NOT EDIT
// source-hash: f2f8682669670c7aaedbccb3102dd71d7b187eac29a32d3d20509a5bc070c12e
// SCE-MAP: outbound_sends_now.scxml:16 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `OutboundSendsNow`, a file of the
// module that calls it: a call is qualified by the enum
// (`OutboundSendsNow.outboundSendsNow(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

public enum OutboundSendsNow {
    public static func outboundSendsNow(_ ready: Bool, _ depth: UInt32) -> Bool {
        return ready && depth == 0
    }
}
