// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A one-shot test timer (docs/SCE_TEST_PROFILE.md §4.7, P7): its status, its
// expiry instant on the logical clock, and its snapshot (ES 201 873-4 §8.3.2.4).

#pragma once

#include <optional>

#include "test/Port.h"

namespace SCE::Test {

/// `STATUS` of a timer binding (-4 §8.3.2.4).
enum class TimerStatus { Idle, Running, Timeout };

class Timer {
public:
    /// `start` (-4 §9.48): running, to expire @p duration after @p now. Starting
    /// a running timer restarts it (-1 §23.2).
    void start(Instant now, Duration duration) {
        status_ = TimerStatus::Running;
        expiry_ = now + duration;
    }

    /// `stop` (-4 §9.52): idle. A timer that is not running is left as it is.
    void stop() {
        if (status_ == TimerStatus::Running) {
            status_ = TimerStatus::Idle;
        }
    }

    /// The update phase (§3.2 step 2): a running timer whose instant has come
    /// times out (-4 §8.6.1.2 a).
    void update(Instant now) {
        if (status_ == TimerStatus::Running && expiry_ <= now) {
            status_ = TimerStatus::Timeout;
        }
    }

    /// The instant it will time out at, while it runs (§3.3).
    std::optional<Instant> expiry() const {
        return status_ == TimerStatus::Running ? std::optional<Instant>(expiry_) : std::nullopt;
    }

    /// `SNAP-STATUS`, taken in a round's snapshot step.
    void takeSnapshot() {
        snapshotTimedOut_ = status_ == TimerStatus::Timeout;
    }

    bool snapshotTimedOut() const {
        return snapshotTimedOut_;
    }

    /// A timeout branch was taken on the snapshot (-4 §9.55, P4 step 1).
    void timeoutTaken() {
        status_ = TimerStatus::Idle;
        snapshotTimedOut_ = false;
    }

    TimerStatus status() const {
        return status_;
    }

private:
    TimerStatus status_ = TimerStatus::Idle;
    Instant expiry_ = 0;
    bool snapshotTimedOut_ = false;
};

}  // namespace SCE::Test
