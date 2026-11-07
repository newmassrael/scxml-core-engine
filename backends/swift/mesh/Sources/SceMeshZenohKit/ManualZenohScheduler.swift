// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import SceMesh
import Synchronization

/// A scheduler with a clock of its own, that moves only when a test says so: work scheduled `delay` from now
/// runs when `advance(by:)` carries the clock past that moment, in the order of the moments (and of the
/// scheduling, for two at one moment), on the thread that called `advance`. Nothing sleeps.
public final class ManualZenohScheduler: ZenohScheduler {
    private struct Item {
        var id: Int
        var due: Duration
        var work: @Sendable () -> Void
    }

    private struct State {
        var now = Duration.zero
        var nextID = 0
        var items: [Item] = []
        var delays: [Duration] = []
    }

    private let state = Mutex(State())

    public init() {}

    public func schedule(after delay: Duration, _ work: @escaping @Sendable () -> Void) -> any ZenohTimer {
        let id = state.withLock { state -> Int in
            state.nextID += 1
            state.items.append(Item(id: state.nextID, due: state.now + delay, work: work))
            state.delays.append(delay)
            return state.nextID
        }
        return ManualTimer(scheduler: self, id: id)
    }

    /// Carry the clock forward by `span`, running what falls due on the way, including what that work
    /// schedules and what falls due before the new end of the span.
    public func advance(by span: Duration) {
        let target = state.withLock { $0.now + span }
        while true {
            let next = state.withLock { state -> Item? in
                let due = state.items.enumerated().filter { $0.element.due <= target }
                guard
                    let first = due.min(by: { ($0.element.due, $0.element.id) < ($1.element.due, $1.element.id) })
                else {
                    return nil
                }
                let item = state.items.remove(at: first.offset)
                state.now = Swift.max(state.now, item.due)
                return item
            }
            guard let item = next else {
                break
            }
            item.work()
        }
        state.withLock { $0.now = target }
    }

    /// Every wait asked for so far, in the order it was asked for.
    public var scheduledDelays: [Duration] {
        state.withLock { $0.delays }
    }

    /// How many waits are still to run.
    public var pending: Int {
        state.withLock { $0.items.count }
    }

    fileprivate func cancel(_ id: Int) {
        state.withLock { state in state.items.removeAll { $0.id == id } }
    }
}

private final class ManualTimer: ZenohTimer {
    private let scheduler: ManualZenohScheduler
    private let id: Int

    init(scheduler: ManualZenohScheduler, id: Int) {
        self.scheduler = scheduler
        self.id = id
    }

    func cancel() {
        scheduler.cancel(id)
    }
}
