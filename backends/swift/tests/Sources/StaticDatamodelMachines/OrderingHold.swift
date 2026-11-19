// SCE-GENERATED — DO NOT EDIT
// source-hash: 175a2b197d681a36f12d525ee8454412c2f1b38f4501dcb1ba1cc669070b46f6
// source: sce:std/mesh/ordering_hold.scxml
// SCE-MAP: ordering_hold.scxml:16 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `OrderingHold`, a file of the
// module that calls it: a call is qualified by the enum
// (`OrderingHold.orderingHold(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

public enum OrderingHold {
    public static func orderingHold(_ pending: [OrderingSlotPayload], _ seq: UInt64, _ now: Int64) -> [OrderingSlotPayload] {
        let arrived: OrderingSlotPayload = OrderingSlotPayload(seq: seq, arrivedAtMs: now)
        var out: [OrderingSlotPayload] = []
        out.reserveCapacity(256)
        var placed: Bool = false
        for slot in pending {
            if slot.seq == seq {
                placed = true
            }
            if !placed && seq < slot.seq {
                out.append(arrived)
                placed = true
            }
            out.append(slot)
        }
        if !placed {
            out.append(arrived)
        }
        return out
    }
}
