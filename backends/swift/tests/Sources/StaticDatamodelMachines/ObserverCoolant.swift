// SCE-GENERATED — DO NOT EDIT
// source-hash: 7b98612ed0f35af10eaffafb7dd60088b78d6093971dc622b4d3161a336da46c
// SCE-MAP: observer_coolant.scxml:1 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="observer")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// A threshold observer: a class that holds one hysteresis state per monitor, whose `update`
// reads the inputs of one sample and answers the events that sample raised, in the order the
// monitors are written. It holds its states, so each holder of one instance shares them.
//
// The events are a `Tag` enumeration nested in the class. A file-local enumeration of one name
// would be declared again by every observer of the module, which is one SwiftPM target.

import SceForgeRuntime

public final class ObserverCoolant {
    // No sce:event-domain is declared on this <scxml> root: the observer falls back to an
    // enumeration of its own. The resulting EventQueue type cannot be composed with other
    // observers. To enable cross-file composition, add sce:event-domain="..." to the source
    // SCXML. See SCE_FORGE.md Section 4.11.
    public enum Tag {
        case emitWarning
        case clearWarning
        case emergencyShutdown
    }

    private let warning = SceForgeRuntime.ThresholdState()
    private let critical = SceForgeRuntime.ThresholdState()

    public init() {}

    public func update(_ coolantTemp: Double) -> SceForgeRuntime.EventQueue<Tag> {
        var sceEvents = SceForgeRuntime.EventQueue<Tag>()
        if warning.enterIf(coolantTemp > 110.0) {
            sceEvents.push(.emitWarning)
        } else if warning.leaveIf(coolantTemp < 100.0) {
            sceEvents.push(.clearWarning)
        }
        if critical.enterIf(coolantTemp > 120.0) {
            sceEvents.push(.emergencyShutdown)
        } else {
            critical.leaveIf(coolantTemp < 105.0)
        }
        return sceEvents
    }
}
