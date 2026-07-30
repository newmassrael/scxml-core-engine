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
//     metal / RTOS implementations supplied by the consumer's link tree
//     (see `consumer_watching_zenoh.md`). Must satisfy the contract:
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

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

uint64_t _sce_clock_now_ms(void);

#ifdef __cplusplus
}
#endif

#endif  // SCE_CLOCK_H
