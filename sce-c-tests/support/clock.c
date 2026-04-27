// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML 6.2 — host-side default for `_sce_clock_now_ms`.
//
// The C11 backend's delayed-`<send>` codegen emits an `extern uint64_t
// _sce_clock_now_ms(void)` declaration into every fixture whose model
// carries `needs_event_scheduler=true` (analyzer.rs sets the flag when
// any `<send delay>` / `<send delayexpr>` / `<cancel>` action is
// reachable). Without a definition, the linker rejects the fixture; on
// the runtime side, returning a fake constant would freeze the schedule
// and silently mis-fire safety-net timeouts.
//
// This translation unit supplies the POSIX-host implementation: monotonic
// milliseconds since some unspecified epoch via
// `clock_gettime(CLOCK_MONOTONIC, …)`. Linked uniformly into every C11
// fixture binary so safety-net timeouts (test403a/b/c, test404, test405,
// test580) and success-path timeouts (test579) share one clock contract.
//
// MCU profiles override by linking against a SysTick-backed translation
// unit instead of this one — the contract is "monotonic milliseconds
// from any epoch" and the cpp `std::chrono::steady_clock` shape it
// mirrors makes no stricter assumption.

#define _POSIX_C_SOURCE 199309L

#include <stdint.h>
#include <time.h>

uint64_t _sce_clock_now_ms(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (uint64_t)ts.tv_sec * 1000u + (uint64_t)ts.tv_nsec / 1000000u;
}
