// SCE-GENERATED — DO NOT EDIT
// source-hash: f6c860c2693fcb3366bc871ff801c9b0ee9e491a1759830f8d83238267cc3b45
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
