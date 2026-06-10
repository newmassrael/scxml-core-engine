// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE C11 runtime — cross-fixture defaults.
//
// RFC §synth-5-J-1 (watching-zenoh downstream MCU backend). The C++ AOT
// runtime in `sce/include/static/StaticExecutionEngine.h` is a class
// template — the compiler instantiates it once per generated state
// machine, so every concrete data layout (event queue, event
// metadata, transition tables) lives inside the generated translation
// unit. The C11 backend mirrors that pattern: jinja2 macros under
// `tools/codegen/templates/c/helpers/*.jinja2` inline equivalent
// algorithm bodies into each generated `_sm.c`, and per-fixture data
// types are declared inside the same translation unit.
//
// As a result this header carries only what is genuinely cross-fixture
// — capacity defaults that a generated machine references via the
// preprocessor. Each can be overridden at build time per fixture
// (`-DSCE_MAX_EVENTS=N`, etc.).

#ifndef SCE_TYPES_H
#define SCE_TYPES_H

#ifndef SCE_MAX_EVENTS
#define SCE_MAX_EVENTS 32
#endif

#ifndef SCE_MAX_ID_LEN
#define SCE_MAX_ID_LEN 64
#endif

#ifndef SCE_MAX_INVOKES
#define SCE_MAX_INVOKES 4
#endif

#ifndef SCE_MAX_PARALLEL_REGIONS
#define SCE_MAX_PARALLEL_REGIONS 4
#endif

// W3C SCXML Appendix D.2 — optimal enabled transition set.
// Per-microstep cap on the number of transitions that may fire
// simultaneously. Sized for the typical <parallel> region count plus
// headroom; fixtures that drive many concurrent regions override at
// build time (`-DSCE_MAX_ENABLED_TRANSITIONS=N`). Stack-local lifetime
// inside process_transition — no SM struct footprint.
#ifndef SCE_MAX_ENABLED_TRANSITIONS
#define SCE_MAX_ENABLED_TRANSITIONS 8
#endif

// W3C SCXML 6.2 — delayed `<send>` queue capacity. Per-instance bounded
// array carrying scheduled events between dispatch (when `<send delay>`
// runs) and fire time (when `_tick(sm)` promotes ready events into the
// internal queue). Sized for the typical mainEventLoop pattern of one or
// two concurrent timeouts; fixtures with deeper schedules override at
// build time (`-DSCE_MAX_SCHEDULED=N`). Mirrors the C++ `PullScheduler`
// which uses an unbounded `std::map`; the C11 backend bounds it because
// MCU profiles cannot allocate dynamically.
#ifndef SCE_MAX_SCHEDULED
#define SCE_MAX_SCHEDULED 4
#endif

// W3C SCXML 5.10 _event.data — generated event records reserve a fixed
// payload slot so the entire EventWithMetadata is value-typed and queue
// storage is statically reservable. The default sizes datamodel-less and
// short JSON payloads; fixtures that carry longer SCXML <send> data
// override per build.
#ifndef SCE_MAX_DATA_LEN
#define SCE_MAX_DATA_LEN 256
#endif

// W3C SCXML 5.10 _event.type — bounded by the spec's enumerated values
// ("internal", "external", "platform"). 16 bytes leaves headroom for any
// future additions without requiring a per-fixture override.
#ifndef SCE_MAX_EVENT_TYPE_LEN
#define SCE_MAX_EVENT_TYPE_LEN 16
#endif

// Static helpers (state hierarchy queries, history filters, …) are
// emitted unconditionally so generated code stays compilable as fixtures
// climb the W3C category ladder. The flat / datamodel-less subset used
// by the simplest fixtures does not call every helper, so wrap each one
// with this attribute to keep `-Wall -Wunused-function` clean. Resolves
// to nothing on toolchains that lack the GNU attribute, where the
// warning is unlikely to be treated as an error anyway.
#if defined(__GNUC__) || defined(__clang__)
#define SCE_C_UNUSED __attribute__((unused))
#else
#define SCE_C_UNUSED
#endif

/* Bounded copy into an `SCE_MAX_ID_LEN`-sized buffer (`invoke_id` /
   `send_id`). Manual length scan + `memcpy` + explicit NUL keeps the
   helper standalone C11 (no `strnlen` POSIX dependency, no
   `<stdio.h>` surface for freestanding MCU fixtures) and still
   NUL-terminates without tripping gcc's `-Wstringop-truncation`
   false positive on the equivalent `strncpy` pattern. NULL `src`
   is normalised to the empty string so callers do not need a guard.
   Lives in `types.h` (not `invoke.h`) so the delayed-send sites in
   `state_machine.c.jinja2` can call it without dragging the invoke
   types into invoke-free fixtures. */
#include <stddef.h>
#include <string.h>
SCE_C_UNUSED static inline void
sce_copy_bounded_id(char *dst, const char *src) {
    const char *s = src != NULL ? src : "";
    size_t n = 0;
    while (n < (size_t)SCE_MAX_ID_LEN - 1u && s[n] != '\0') {
        n++;
    }
    memcpy(dst, s, n);
    dst[n] = '\0';
}

#endif  // SCE_TYPES_H
