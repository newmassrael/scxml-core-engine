// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE C11 runtime — cross-fixture defaults.
//
// RFC §synth-5-J-1 (downstream MCU backend). The C++ AOT
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

/* §scxml-C-2-3: capacity of the BasicHTTP access URI a deployment
   declares on a machine, published as that processor's `_ioprocessors`
   location. Sized for a host:port URL with a path; a deployment whose
   endpoint URI is longer overrides at build time. */
#ifndef SCE_MAX_URI_LEN
#define SCE_MAX_URI_LEN 256
#endif

#ifndef SCE_MAX_INVOKES
#define SCE_MAX_INVOKES 4
#endif

#ifndef SCE_MAX_PARALLEL_REGIONS
#define SCE_MAX_PARALLEL_REGIONS 4
#endif

// §scxml-D-microstepProcedure — optimal enabled transition set.
// Per-microstep cap on the number of transitions that may fire
// simultaneously. Sized for the typical <parallel> region count plus
// headroom; fixtures that drive many concurrent regions override at
// build time (`-DSCE_MAX_ENABLED_TRANSITIONS=N`). Stack-local lifetime
// inside process_transition — no SM struct footprint.
#ifndef SCE_MAX_ENABLED_TRANSITIONS
#define SCE_MAX_ENABLED_TRANSITIONS 8
#endif

// §scxml-6.2 — delayed `<send>` queue capacity. Per-instance bounded
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

// §scxml-5.10 _event.data — generated event records reserve a fixed
// payload slot so the entire EventWithMetadata is value-typed and queue
// storage is statically reservable. The default sizes datamodel-less and
// short JSON payloads; fixtures that carry longer SCXML <send> data
// override per build.
#ifndef SCE_MAX_DATA_LEN
#define SCE_MAX_DATA_LEN 256
#endif

// §scxml-5.10 _event.type — bounded by the spec's enumerated values
// ("internal", "external", "platform"). 16 bytes leaves headroom for any
// future additions without requiring a per-fixture override.
#ifndef SCE_MAX_EVENT_TYPE_LEN
#define SCE_MAX_EVENT_TYPE_LEN 16
#endif

// §scxml-3.12.2 — how many links an `error.*` chain may have before the
// engine stops feeding it. The clause says what to do with an error event
// nothing matches; it does not say what to do when something DOES match it
// and that handler fails too, so the failure raises the same error, the same
// transition answers it, and the drain never empties. Nothing in the
// specification bounds that, so the number is this engine's to choose, and it
// is the hundred the Rust, Go, Kotlin and C++ engines use for the sibling case
// of a macrostep that cannot finish. Overridable per build for a document
// whose repair strategy is genuinely deeper, which no plausible one is.
#ifndef SCE_MAX_ERROR_CASCADE_DEPTH
#define SCE_MAX_ERROR_CASCADE_DEPTH 100u
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

SCE_C_UNUSED static inline void sce_copy_bounded_n(char *dst, const char *src, size_t cap) {
    const char *s = src != NULL ? src : "";
    size_t n = 0;
    if (cap == 0u) {
        return;
    }
    while (n < cap - 1u && s[n] != '\0') {
        n++;
    }
    memcpy(dst, s, n);
    dst[n] = '\0';
}

SCE_C_UNUSED static inline void sce_copy_bounded_id(char *dst, const char *src) {
    sce_copy_bounded_n(dst, src, (size_t)SCE_MAX_ID_LEN);
}

/* §scxml-5.10 `_event.data` payload buffer (SCE_MAX_DATA_LEN). Separate
   from the id-sized helper because the two caps differ; both delegate to
   `sce_copy_bounded_n` so the scan/copy/NUL logic exists once. */
SCE_C_UNUSED static inline void sce_copy_bounded_data(char *dst, const char *src) {
    sce_copy_bounded_n(dst, src, (size_t)SCE_MAX_DATA_LEN);
}

/* §scxml-5.10.1 `_event.type` ("internal" / "external" / "platform"). */
SCE_C_UNUSED static inline void sce_copy_bounded_event_type(char *dst, const char *src) {
    sce_copy_bounded_n(dst, src, (size_t)SCE_MAX_EVENT_TYPE_LEN);
}

/* Bounded copy of an `_event.origin`. Sized as a URI, not as an id,
   because of what the field holds: the 'location' the sending session
   published in its `_ioprocessors` — the same kind of value
   `basic_http_access_uri` carries, and for the same reason. Copying an
   address through the id-sized helper truncates it, and a truncated
   address compares unequal to the location it was cut from while still
   looking like one, so the failure reads as a spec violation rather than
   as a buffer that was too small. */
SCE_C_UNUSED static inline void sce_copy_bounded_origin(char *dst, const char *src) {
    sce_copy_bounded_n(dst, src, (size_t)SCE_MAX_URI_LEN);
}

/* ── §scxml-6.4: Autoforward carrier ───────────────────────────── */
/* §scxml-6.4 requires the parent to forward an *exact copy* of every
   external event to an `<invoke autoforward="true">` child. The child is a
   different generated machine, so its `<sm>_event_with_meta_t` is an
   unrelated type and the copy cannot cross as that struct; it crosses as
   this transport-neutral record instead, addressed by event NAME (the only
   identity the two machines share). `target` is deliberately absent: it is
   a routing decision owned by the `<send>` that produced the original
   event, and inheriting it would re-route the child's copy.

   Lives in `types.h` rather than `invoke.h` for the same reason
   `sce_copy_bounded_id` does: the *child* machine declares
   `_raise_external_forwarded` while having no `<invoke>` of its own, so it
   never includes `invoke.h` (and must not pull that header's `<stdio.h>`
   surface into a freestanding MCU fixture). The C mirror of cpp
   `SCE::Common::ForwardedEvent` / Rust `(&str, &EventMetadata)`. */
typedef struct sce_forwarded_event_s {
    char name[SCE_MAX_ID_LEN];
    char data[SCE_MAX_DATA_LEN];
    /* An address, not an id — see `sce_copy_bounded_origin`. */
    char origin[SCE_MAX_URI_LEN];
    char send_id[SCE_MAX_ID_LEN];
    char type[SCE_MAX_EVENT_TYPE_LEN];
    char origin_type[SCE_MAX_ID_LEN];
    char invoke_id[SCE_MAX_ID_LEN];
} sce_forwarded_event_t;

#endif  // SCE_TYPES_H
