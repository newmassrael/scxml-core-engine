// SCE-GENERATED — DO NOT EDIT
// source-hash: 1ba579cdc10cbe049a3766817dcc6397d58e8716381e63be0e21ee74788ac719
// SCE-MAP: timer_tester_present.scxml:19 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="timer")
// Shape: a single timer per document with an event-driven reset, a state-exit cancel and a
// fire event, as the timer section of the Protocol-Synthesis RFC defines it.
// Runtime: sce_forge_runtime::hal
// Do not edit — regenerate from the source SCXML file.
//
// The handler is held weakly. A state class that owns this timer is the usual handler, and a
// handler held strongly by the timer's callback, which the host's clock holds, would be a
// cycle nothing breaks: the timer fires into a handler that is gone, which is no fire at all.

import SceForgeRuntime

/// The object a timer fires into. The state class implements the fire method and hands an
/// instance to `TimerTesterPresent`.
public protocol TimerTesterPresentHandler: AnyObject {
    func fireTesterPresent()
}

public final class TimerTesterPresent {
    /// The period configured at compile time from `<sce:period>`, in microseconds, which covers
    /// microsecond ticks and minute-scale watchdogs in one type.
    public static let periodUs: Int64 = 2000000

    /// The period in milliseconds, which is what the clock is asked for.
    public static let periodMs: Int64 = 2000

    /// The event `<sce:reset-on>` names.
    public static let resetOnEvent = "DiagRequestSent"

    /// The state `<sce:cancel-on>` names.
    public static let cancelOnStateExit = "SessionActive"

    private weak var handler: (TimerTesterPresentHandler)?
    private let timer: SceForgeRuntime.ForgeTimer

    public init(handler: TimerTesterPresentHandler, timer: SceForgeRuntime.ForgeTimer) {
        self.handler = handler
        self.timer = timer
    }

    /// Starts the periodic timer at `periodMs`.
    public func start() {
        // The callback holds the handler and not this object, so the clock fires whether or not
        // this object is still held, and stops firing when the handler is gone.
        timer.startPeriodic(intervalMs: Self.periodMs) { [weak handler = self.handler] in
            handler?.fireTesterPresent()
        }
    }

    /// Cancels the timer. Idempotent per the runtime contract.
    public func cancel() {
        timer.cancel()
    }

    /// `<sce:reset-on event="DiagRequestSent"/>` consumer hook.
    public func onResetDiagRequestSent() {
        cancel()
        start()
    }

    /// `<sce:cancel-on state-exit="SessionActive"/>` consumer hook.
    public func onCancelSessionActiveExit() {
        cancel()
    }
}
