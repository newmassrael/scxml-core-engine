// SCE-GENERATED — DO NOT EDIT
// source-hash: 62067cbe5198ad923571ee5243de08cf004a9a483bf54ae3e6db2a14e8ad77a7
// SCE-MAP: lookup_engine_status.scxml:3 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="lookup")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// A table read by exact match: a static function of the caseless enum `LookupEngineStatus`, a
// file of the module that calls it. A call is qualified by the enum
// (`LookupEngineStatus.lookupStatus(…)`), which a member of the calling type cannot hide as
// it would hide a free function of the same name.
public enum LookupEngineStatus {
    /// The names the table answers, each case holding the name the document spells.
    public enum Status: String, Sendable {
        case STOP = "STOP"
        case RUNNING = "RUNNING"
        case FAULT = "FAULT"
    }

    public static func lookupStatus(_ engSta: UInt8) -> Status {
        switch engSta {
        case 0x07:
            return .FAULT
        case 0x03:
            return .RUNNING
        case 0x00, 0x01, 0x02:
            return .STOP
        default:
            return .STOP
        }
    }
}
