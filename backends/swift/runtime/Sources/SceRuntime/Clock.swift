// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

/// The source of "now" behind every `<send delay>` an engine arms and every due
/// judgement its tick makes.
///
/// §scxml-6.2.2 says a delay "indicates how long the processor should wait
/// before dispatching the message", and says nothing about where the processor
/// reads the time from. Leaving that hardwired to the wall answers a question the
/// spec left to the host, and answers it the one way that cannot be reproduced: a
/// host descheduled between two statements of the same `<onentry>` gets two
/// different readings for one instant, and the deadlines it computes from them
/// can order the sends differently on every run.
///
/// So the reading is a seam, not a constant. `MonotonicClock` is the default and
/// is what a production host wants; `ManualClock` hands the clock to the host
/// outright, which is what a simulation, a replay, a discrete-event scheduler and
/// a deterministic test all want. Both are on the shipped surface: a consumer can
/// install either, or write a third.
public protocol SceClock {
    /// Milliseconds since this clock's origin.
    ///
    /// Must be non-decreasing: the scheduler compares readings taken at different
    /// moments, and a reading that went backwards would make an entry that was due
    /// stop being due.
    func elapsedMs() -> Int64
}

/// The default clock: the host's monotonic reading, measured from the moment this
/// clock was constructed.
///
/// This is what an engine gets when nothing else is installed, and what a
/// production host running against real time should keep. It is unaffected by
/// wall-clock adjustments.
public struct MonotonicClock: SceClock {
    private let origin: SuspendingClock.Instant

    /// A clock whose origin is now.
    public init() {
        origin = SuspendingClock.now
    }

    public func elapsedMs() -> Int64 {
        let elapsed = origin.duration(to: SuspendingClock.now)
        let (seconds, attoseconds) = elapsed.components
        return seconds * 1000 + attoseconds / 1_000_000_000_000_000
    }
}

/// A clock the host moves by hand.
///
/// Time advances only when `advance` is called, so a machine driven through one
/// of these reaches the same configuration on every run regardless of what else
/// the machine it runs on is doing. That is what makes it the right clock for a
/// simulation, for replaying a recorded trace at a speed of the host's choosing,
/// and for a test that wants a verdict about the engine and not about the load on
/// the build machine.
///
/// Install it before `Engine.initialize`, and drive the machine with
/// `Engine.advanceTimeMs` and not by calling `advance` directly: the engine's
/// entry point moves this clock and then runs whatever that made due, which is
/// the whole of the contract.
///
/// One instance may be shared by several engines, so a parent and the sessions it
/// invokes read the same absolute time (§scxml-6.4).
public final class ManualClock: SceClock {
    private var nowMs: Int64

    /// A clock reading `startMs`.
    ///
    /// A negative origin traps: `elapsedMs` is required to be non-decreasing and
    /// the scheduler's deadlines are computed by addition, so a negative origin is
    /// a caller error and not a choice of coordinates.
    public init(startMs: Int64 = 0) {
        precondition(startMs >= 0, "sce: ManualClock origin must not be negative")
        nowMs = startMs
    }

    public func elapsedMs() -> Int64 {
        nowMs
    }

    /// Moves this clock forward by `ms` milliseconds.
    ///
    /// A negative delta traps and is not accepted: `SceClock.elapsedMs` is
    /// required to be non-decreasing, and a clock that went backwards would un-due
    /// an entry the scheduler had already judged ready.
    public func advance(_ ms: Int64) {
        precondition(ms >= 0, "sce: ManualClock.advance requires a non-negative delta")
        nowMs += ms
    }
}
