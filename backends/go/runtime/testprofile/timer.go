// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package testprofile

// A one-shot test timer (docs/SCE_TEST_PROFILE.md §4.7, P7): its status, its
// expiry instant on the logical clock, and its snapshot (ES 201 873-4
// §8.3.2.4).
//
// Cross-language siblings: `sce_rust_runtime::test::Timer` (Rust),
// `SCE::Test::Timer` (C++), `com.sce.runtime.test.Timer` (Kotlin).

// TimerStatus is `STATUS` of a timer binding (-4 §8.3.2.4).
type TimerStatus int

const (
	// TimerIdle is a timer that is not running.
	TimerIdle TimerStatus = iota
	// TimerRunning is a timer running, to expire at its instant.
	TimerRunning
	// TimerTimeout is a timer whose instant has come and that no timeout
	// branch has taken yet.
	TimerTimeout
)

// Timer is one timer binding of a component. The zero value is an idle timer.
type Timer struct {
	status           TimerStatus
	expiresAt        Instant
	snapshotTimedOut bool
}

// Status is the timer's status.
func (t *Timer) Status() TimerStatus {
	return t.status
}

// Start is `start` (-4 §9.48): running, to expire duration after now. Starting
// a running timer restarts it (-1 §23.2).
func (t *Timer) Start(now Instant, duration Duration) {
	t.status = TimerRunning
	t.expiresAt = now + duration
}

// Stop is `stop` (-4 §9.52): idle. A timer that is not running is left as it
// is.
func (t *Timer) Stop() {
	if t.status == TimerRunning {
		t.status = TimerIdle
	}
}

// Update is the update phase (§3.2 step 2): a running timer whose instant has
// come times out (-4 §8.6.1.2 a).
func (t *Timer) Update(now Instant) {
	if t.status == TimerRunning && t.expiresAt <= now {
		t.status = TimerTimeout
	}
}

// Expiry is the instant the timer will time out at and true while it runs
// (§3.3), and false otherwise.
func (t *Timer) Expiry() (Instant, bool) {
	if t.status != TimerRunning {
		return 0, false
	}
	return t.expiresAt, true
}

// TakeSnapshot takes `SNAP-STATUS` in a round's snapshot step.
func (t *Timer) TakeSnapshot() {
	t.snapshotTimedOut = t.status == TimerTimeout
}

// SnapshotTimedOut reports whether the last snapshot saw the timer timed out.
func (t *Timer) SnapshotTimedOut() bool {
	return t.snapshotTimedOut
}

// TimeoutTaken is a timeout branch taken on the snapshot (-4 §9.55, P4 step 1).
func (t *Timer) TimeoutTaken() {
	t.status = TimerIdle
	t.snapshotTimedOut = false
}
