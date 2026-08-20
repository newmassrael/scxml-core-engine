// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// §scxml-6.2 delayed-send clock primitive (sce-c-core tier).
//
// Generated C11 state machines reference `_sce_clock_now_ms` to drive the
// scheduled-event queue (`scheduled_pop_ready` / `_tick`); the symbol is
// resolved at link time by whichever implementation tier the consumer
// links in:
//
//   - `sce_c_runtime_posix` (backends/c/runtime/posix/clock.c) — POSIX
//     `clock_gettime(CLOCK_MONOTONIC)` reference impl. Default for the
//     W3C 204 conformance runner and any host-compatible consumer.
//
//   - Downstream `sce_c_runtime_<target>` (e.g. lwip, FreeRTOS) — bare-
//     metal / RTOS implementations supplied by the consumer's link
//     tree. Must satisfy the contract:
//
//   * Returns a monotonically non-decreasing millisecond counter.
//   * Origin is implementation-defined (epoch-since-boot, since-startup,
//     etc.); the generated code only ever takes deltas.
//   * Thread-safe: callable from any context that drives `_tick`.
//
// Pre-1.0: this contract may evolve before SCE 1.0 (per
// `feedback_pre_release_no_compat.md`); downstream impls re-link
// against new releases.

#ifndef SCE_CLOCK_H
#define SCE_CLOCK_H

#include <stdbool.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

uint64_t _sce_clock_now_ms(void);

// A reading function the host supplies, in milliseconds since an origin of its
// own choosing. Must be non-decreasing, for the reason on `sce_clock_t`.
typedef uint64_t (*sce_clock_read_fn)(void *user_data);

// §scxml-6.2.2: where a generated machine reads "now" from.
//
// The clause says a delay "indicates how long the processor should wait before
// dispatching the message", and says nothing about where the processor reads
// the time from. Leaving that hardwired to `_sce_clock_now_ms()` answers a
// question the spec left to the host, and answers it the one way that cannot be
// reproduced: a host descheduled between two statements of the same `<onentry>`
// gets two different readings for one instant, and the deadlines it computes
// from them can order the sends differently on every run.
//
// `_sce_clock_now_ms()` remains the default and stays the link-time seam a
// bare-metal consumer wires its tick source into — one clock for the image.
// This struct is the per-instance seam beside it: a value on the machine, so
// one generated machine serves a host on the wall clock and a host that owns
// time outright, chosen at run time rather than at link time.
//
// Zero-initialised means the default, which is what `_init`'s `memset` leaves.
typedef struct sce_clock {
    // NULL: read the link-time `_sce_clock_now_ms()`.
    sce_clock_read_fn read;
    void *user_data;
    // true: `now_ms` below IS the time, and only the host moves it.
    bool manual;
    uint64_t now_ms;
} sce_clock_t;

// The default: the link-time monotonic clock.
static inline sce_clock_t sce_clock_monotonic(void) {
    sce_clock_t clock = {NULL, NULL, false, 0u};
    return clock;
}

// Host-owned time, starting at `start_ms`. A machine driven through one of
// these reaches the same configuration on every run regardless of what else the
// machine it runs on is doing, which is what a simulation, a replay, a
// discrete-event scheduler and a deterministic test all need.
static inline sce_clock_t sce_clock_manual(uint64_t start_ms) {
    sce_clock_t clock = {NULL, NULL, true, start_ms};
    return clock;
}

// A host-supplied reading function — an RTOS tick counter reached through a
// different symbol, a media clock, a simulation running faster than real time.
static inline sce_clock_t sce_clock_source(sce_clock_read_fn read, void *user_data) {
    sce_clock_t clock = {read, user_data, false, 0u};
    return clock;
}

// Read `clock`. Never called by a host directly — the generated machine reads
// its own clock once per turn (see the generated `_set_clock` doc).
static inline uint64_t sce_clock_read(const sce_clock_t *clock) {
    if (clock->manual) {
        return clock->now_ms;
    }
    if (clock->read != NULL) {
        return clock->read(clock->user_data);
    }
    return _sce_clock_now_ms();
}

#ifdef __cplusplus
}
#endif

#endif  // SCE_CLOCK_H
