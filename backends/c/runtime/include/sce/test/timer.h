/* SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/**
 * @file timer.h
 * @brief A one-shot test timer (docs/SCE_TEST_PROFILE.md §4.7, P7): its
 *        status, its expiry instant on the logical clock, and its snapshot
 *        (ES 201 873-4 §8.3.2.4)
 *
 * The C11 port of `sce/include/test/Timer.h` and `sce_rust_runtime::test::Timer`.
 * Header-only and allocation-free.
 */

#ifndef SCE_TEST_TIMER_H
#define SCE_TEST_TIMER_H

#include <stdbool.h>
#include <stddef.h>

#include "sce/test/port.h"

#ifdef __cplusplus
extern "C" {
#endif

/** `STATUS` of a timer binding (-4 §8.3.2.4). */
typedef enum sce_test_timer_status_e {
    SCE_TEST_TIMER_IDLE = 0,
    SCE_TEST_TIMER_RUNNING,
    SCE_TEST_TIMER_TIMEOUT
} sce_test_timer_status_t;

/** One timer binding of a component: its id, status, expiry and snapshot. */
typedef struct sce_test_timer_s {
    /** The timer's id, a string that outlives the timer (the configuration's,
        or the statement's that started it). */
    const char *id;
    sce_test_timer_status_t status;
    sce_test_instant_t expiry;
    bool snapshot_timed_out;
} sce_test_timer_t;

/** An idle timer of the id `id`. */
static inline void sce_test_timer_init(sce_test_timer_t *timer, const char *id) {
    timer->id = id;
    timer->status = SCE_TEST_TIMER_IDLE;
    timer->expiry = 0;
    timer->snapshot_timed_out = false;
}

/** `start` (-4 §9.48): running, to expire `duration` after `now`. Starting a
 *  running timer restarts it (-1 §23.2). */
static inline void sce_test_timer_start(sce_test_timer_t *timer, sce_test_instant_t now, sce_test_duration_t duration) {
    timer->status = SCE_TEST_TIMER_RUNNING;
    timer->expiry = now + duration;
}

/** `stop` (-4 §9.52): idle. A timer that is not running is left as it is. */
static inline void sce_test_timer_stop(sce_test_timer_t *timer) {
    if (timer->status == SCE_TEST_TIMER_RUNNING) {
        timer->status = SCE_TEST_TIMER_IDLE;
    }
}

/** The update phase (§3.2 step 2): a running timer whose instant has come
 *  times out (-4 §8.6.1.2 a). */
static inline void sce_test_timer_update(sce_test_timer_t *timer, sce_test_instant_t now) {
    if (timer->status == SCE_TEST_TIMER_RUNNING && timer->expiry <= now) {
        timer->status = SCE_TEST_TIMER_TIMEOUT;
    }
}

/** The instant it will time out at, in `*at`, while it runs (§3.3); false,
 *  `*at` untouched, when it does not run. */
static inline bool sce_test_timer_expiry(const sce_test_timer_t *timer, sce_test_instant_t *at) {
    if (timer->status != SCE_TEST_TIMER_RUNNING) {
        return false;
    }
    *at = timer->expiry;
    return true;
}

/** `SNAP-STATUS`, taken in a round's snapshot step. */
static inline void sce_test_timer_take_snapshot(sce_test_timer_t *timer) {
    timer->snapshot_timed_out = timer->status == SCE_TEST_TIMER_TIMEOUT;
}

/** The last snapshot saw the timer timed out. */
static inline bool sce_test_timer_snapshot_timed_out(const sce_test_timer_t *timer) {
    return timer->snapshot_timed_out;
}

/** A timeout branch was taken on the snapshot (-4 §9.55, P4 step 1). */
static inline void sce_test_timer_timeout_taken(sce_test_timer_t *timer) {
    timer->status = SCE_TEST_TIMER_IDLE;
    timer->snapshot_timed_out = false;
}

#ifdef __cplusplus
}
#endif

#endif /* SCE_TEST_TIMER_H */
