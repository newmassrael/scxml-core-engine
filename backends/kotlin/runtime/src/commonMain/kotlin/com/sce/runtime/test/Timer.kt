// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A one-shot test timer (docs/SCE_TEST_PROFILE.md §4.7, P7): its status, its
// expiry instant on the logical clock, and its snapshot (ES 201 873-4 §8.3.2.4).
//
// Cross-language siblings: `sce_rust_runtime::test::Timer` (Rust),
// `SCE::Test::Timer` (C++).

package com.sce.runtime.test

/** `STATUS` of a timer binding (-4 §8.3.2.4). */
enum class TimerStatus {
    /** Not running. */
    IDLE,

    /** Running, to expire at its instant. */
    RUNNING,

    /** Its instant has come and no timeout branch has taken it yet. */
    TIMEOUT,
}

/** One timer binding of a component. */
class Timer {
    /** The timer's status. */
    var status: TimerStatus = TimerStatus.IDLE
        private set

    private var expiresAt: Instant = 0

    /** Whether the last snapshot saw the timer timed out. */
    var snapshotTimedOut: Boolean = false
        private set

    /**
     * `start` (-4 §9.48): running, to expire [duration] after [now]. Starting
     * a running timer restarts it (-1 §23.2).
     */
    fun start(now: Instant, duration: Duration) {
        status = TimerStatus.RUNNING
        expiresAt = now + duration
    }

    /** `stop` (-4 §9.52): idle. A timer that is not running is left as it is. */
    fun stop() {
        if (status == TimerStatus.RUNNING) {
            status = TimerStatus.IDLE
        }
    }

    /**
     * The update phase (§3.2 step 2): a running timer whose instant has come
     * times out (-4 §8.6.1.2 a).
     */
    fun update(now: Instant) {
        if (status == TimerStatus.RUNNING && expiresAt <= now) {
            status = TimerStatus.TIMEOUT
        }
    }

    /** The instant it will time out at, while it runs (§3.3); null otherwise. */
    val expiry: Instant?
        get() = if (status == TimerStatus.RUNNING) expiresAt else null

    /** `SNAP-STATUS`, taken in a round's snapshot step. */
    fun takeSnapshot() {
        snapshotTimedOut = status == TimerStatus.TIMEOUT
    }

    /** A timeout branch was taken on the snapshot (-4 §9.55, P4 step 1). */
    fun timeoutTaken() {
        status = TimerStatus.IDLE
        snapshotTimedOut = false
    }
}
