// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import SceForgeRuntime
import XCTest

@testable import StaticDatamodelMachines

// The timers `sce-codegen generate -l swift` writes (`scripts/regen_static_datamodel_swift.sh`):
// a class that starts a periodic timer of the document's period on a clock the host provides,
// fires its handler each period, restarts on the reset event's hook and stops on the cancel hook.
// The clock here is a manual one a test advances, as a host's own tests of a unit use.
final class TimerUnitTests: XCTestCase {
    private final class ManualTimer: ForgeTimer {
        private var interval: Int64?
        private var callback: (() -> Void)?
        private var elapsed: Int64 = 0

        func startPeriodic(intervalMs: Int64, callback: @escaping () -> Void) {
            interval = intervalMs
            self.callback = callback
            elapsed = 0
        }

        func startOneShot(delayMs: Int64, callback: @escaping () -> Void) {}

        func cancel() {
            interval = nil
            callback = nil
        }

        func advance(by ms: Int64) {
            for _ in 0..<ms {
                elapsed += 1
                if let interval, let callback, elapsed % interval == 0 {
                    callback()
                }
            }
        }
    }

    private final class Counter: TimerTesterPresentHandler {
        var fires = 0

        func fireTesterPresent() {
            fires += 1
        }
    }

    // `timer_tester_present.scxml`: a period of two seconds, so that the millisecond and the
    // microsecond constants differ by the factor a unit mix-up would not keep (2000 and 2000000),
    // reset by `DiagRequestSent` and cancelled on leaving `SessionActive`.
    func testThePeriodAndTheNamesAreTheDocuments() {
        XCTAssertEqual(TimerTesterPresent.periodUs, 2_000_000)
        XCTAssertEqual(TimerTesterPresent.periodMs, 2000)
        XCTAssertEqual(TimerTesterPresent.resetOnEvent, "DiagRequestSent")
        XCTAssertEqual(TimerTesterPresent.cancelOnStateExit, "SessionActive")
    }

    func testAStartedTimerFiresItsHandlerEachPeriod() {
        let clock = ManualTimer()
        let handler = Counter()
        let timer = TimerTesterPresent(handler: handler, timer: clock)
        timer.start()
        clock.advance(by: 5000)
        XCTAssertEqual(handler.fires, 2)
    }

    func testTheResetHookRestartsThePeriod() {
        let clock = ManualTimer()
        let handler = Counter()
        let timer = TimerTesterPresent(handler: handler, timer: clock)
        timer.start()
        clock.advance(by: 1500)
        timer.onResetDiagRequestSent()
        clock.advance(by: 1500)
        XCTAssertEqual(handler.fires, 0)
        clock.advance(by: 500)
        XCTAssertEqual(handler.fires, 1)
    }

    func testTheCancelHookStopsTheTimer() {
        let clock = ManualTimer()
        let handler = Counter()
        let timer = TimerTesterPresent(handler: handler, timer: clock)
        timer.start()
        clock.advance(by: 2500)
        timer.onCancelSessionActiveExit()
        clock.advance(by: 10_000)
        XCTAssertEqual(handler.fires, 1)
    }

    // The timer holds its handler weakly, so a handler that owns the timer is no cycle, and a
    // handler that is gone is not fired into.
    func testAHandlerThatIsGoneIsNotFiredInto() {
        let clock = ManualTimer()
        weak var gone: Counter?
        var timer: TimerTesterPresent?
        do {
            let handler = Counter()
            gone = handler
            timer = TimerTesterPresent(handler: handler, timer: clock)
            timer?.start()
            clock.advance(by: 2000)
            XCTAssertEqual(handler.fires, 1)
        }
        XCTAssertNil(gone)
        clock.advance(by: 4000)
        XCTAssertNotNil(timer)
    }
}
