// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Testing

@testable import SceForgeRuntime

// A host implements the clock; this one is a manual clock that a test advances, which is what
// a host's own tests of a unit use. What is held here is the contract a host is held to: a
// periodic timer calls its callback each interval, a one-shot once, and cancelling stops both.

private final class ManualTimer: ForgeTimer {
    private var periodic: (interval: Int64, callback: () -> Void)?
    private var oneShot: (delay: Int64, callback: () -> Void)?
    private var elapsed: Int64 = 0

    func startPeriodic(intervalMs: Int64, callback: @escaping () -> Void) {
        periodic = (intervalMs, callback)
        elapsed = 0
    }

    func startOneShot(delayMs: Int64, callback: @escaping () -> Void) {
        oneShot = (delayMs, callback)
        elapsed = 0
    }

    func cancel() {
        periodic = nil
        oneShot = nil
    }

    func advance(by ms: Int64) {
        var remaining = ms
        while remaining > 0 {
            remaining -= 1
            elapsed += 1
            if let p = periodic, elapsed % p.interval == 0 {
                p.callback()
            }
            if let o = oneShot, elapsed == o.delay {
                oneShot = nil
                o.callback()
            }
        }
    }
}

@Test func aPeriodicTimerCallsBackEachInterval() {
    let timer = ManualTimer()
    var calls = 0
    timer.startPeriodic(intervalMs: 100) { calls += 1 }
    timer.advance(by: 350)
    #expect(calls == 3)
}

@Test func aOneShotTimerCallsBackOnce() {
    let timer = ManualTimer()
    var calls = 0
    timer.startOneShot(delayMs: 50) { calls += 1 }
    timer.advance(by: 500)
    #expect(calls == 1)
}

@Test func cancellingStopsATimerAndIsIdempotent() {
    let timer = ManualTimer()
    var calls = 0
    timer.startPeriodic(intervalMs: 10) { calls += 1 }
    timer.advance(by: 25)
    timer.cancel()
    timer.cancel()
    timer.advance(by: 100)
    #expect(calls == 2)
}
