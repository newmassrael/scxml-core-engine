// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A one-shot test timer (docs/SCE_TEST_PROFILE.md §4.7, P7): its status, its
//! expiry instant on the logical clock, and its snapshot (ES 201 873-4 §8.3.2.4).

use super::port::{Duration, Instant};

/// `STATUS` of a timer binding (-4 §8.3.2.4).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TimerStatus {
    /// Not running.
    #[default]
    Idle,
    /// Running, to expire at its instant.
    Running,
    /// Its instant has come and no timeout branch has taken it yet.
    Timeout,
}

/// One timer binding of a component.
#[derive(Clone, Debug, Default)]
pub struct Timer {
    status: TimerStatus,
    expiry: Instant,
    snapshot_timed_out: bool,
}

impl Timer {
    /// `start` (-4 §9.48): running, to expire `duration` after `now`. Starting
    /// a running timer restarts it (-1 §23.2).
    pub fn start(&mut self, now: Instant, duration: Duration) {
        self.status = TimerStatus::Running;
        self.expiry = now + duration;
    }

    /// `stop` (-4 §9.52): idle. A timer that is not running is left as it is.
    pub fn stop(&mut self) {
        if self.status == TimerStatus::Running {
            self.status = TimerStatus::Idle;
        }
    }

    /// The update phase (§3.2 step 2): a running timer whose instant has come
    /// times out (-4 §8.6.1.2 a).
    pub fn update(&mut self, now: Instant) {
        if self.status == TimerStatus::Running && self.expiry <= now {
            self.status = TimerStatus::Timeout;
        }
    }

    /// The instant it will time out at, while it runs (§3.3).
    pub fn expiry(&self) -> Option<Instant> {
        (self.status == TimerStatus::Running).then_some(self.expiry)
    }

    /// `SNAP-STATUS`, taken in a round's snapshot step.
    pub fn take_snapshot(&mut self) {
        self.snapshot_timed_out = self.status == TimerStatus::Timeout;
    }

    /// Whether the last snapshot saw the timer timed out.
    pub fn snapshot_timed_out(&self) -> bool {
        self.snapshot_timed_out
    }

    /// A timeout branch was taken on the snapshot (-4 §9.55, P4 step 1).
    pub fn timeout_taken(&mut self) {
        self.status = TimerStatus::Idle;
        self.snapshot_timed_out = false;
    }

    /// The timer's status.
    pub fn status(&self) -> TimerStatus {
        self.status
    }
}
