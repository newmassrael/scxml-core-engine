// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// A one-shot test timer (docs/SCE_TEST_PROFILE.md §4.7, P7): its status, its expiry instant on the
// logical clock, and its snapshot (ES 201 873-4 §8.3.2.4). `TestTimer` and not `Timer`, which
// Foundation declares (`Port.swift` says why the profile's names carry `Test`).
//
// Cross-language siblings: `sce_rust_runtime::test::Timer` (Rust), `SCE::Test::Timer` (C++),
// `com.sce.runtime.test.Timer` (Kotlin), `testprofile.Timer` (Go), `sce_runtime.testprofile.Timer`
// (Python), `sce_test_timer_t` (C11).

/// `STATUS` of a timer binding (-4 §8.3.2.4).
public enum TestTimerStatus: Sendable {
    /// Not running.
    case idle
    /// Running, to expire at its instant.
    case running
    /// Its instant has come and no timeout branch has taken it yet.
    case timeout
}

/// One timer binding of a component. A new timer is idle.
public final class TestTimer {
    /// The timer's status.
    public private(set) var status: TestTimerStatus = .idle

    private var expiresAt: TestInstant = 0

    /// Whether the last snapshot saw the timer timed out.
    public private(set) var snapshotTimedOut = false

    public init() {}

    /// `start` (-4 §9.48): running, to expire `duration` after `now`. Starting a running timer restarts
    /// it (-1 §23.2).
    public func start(now: TestInstant, duration: TestDuration) {
        status = .running
        expiresAt = now + duration
    }

    /// `stop` (-4 §9.52): idle. A timer that is not running is left as it is.
    public func stop() {
        if status == .running {
            status = .idle
        }
    }

    /// The update phase (§3.2 step 2): a running timer whose instant has come times out
    /// (-4 §8.6.1.2 a).
    public func update(now: TestInstant) {
        if status == .running && expiresAt <= now {
            status = .timeout
        }
    }

    /// The instant it will time out at, while it runs (§3.3); nil otherwise.
    public var expiry: TestInstant? {
        status == .running ? expiresAt : nil
    }

    /// `SNAP-STATUS`, taken in a round's snapshot step.
    public func takeSnapshot() {
        snapshotTimedOut = status == .timeout
    }

    /// A timeout branch was taken on the snapshot (-4 §9.55, P4 step 1).
    public func timeoutTaken() {
        status = .idle
        snapshotTimedOut = false
    }
}
