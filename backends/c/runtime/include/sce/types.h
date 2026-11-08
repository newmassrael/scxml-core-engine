// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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

/* SCE Accepted Subset §2.15: the most bytes of a session id a machine that saves
   and restores holds (docs/adr/0045). A restored session carries on under the id
   it was saved with, which another backend drew (a UUID is 36 bytes) or a parent
   derived from its own (`parent.state.N.invoke`, one segment more for each level
   of a tree), so the id the machine drew for itself (`<name>_session_<n>`) is
   not the size to hold. A saved id past it is refused by the restore that meets
   it, naming this bound; a deployment with a deeper tree overrides at build time. */
#ifndef SCE_MAX_SESSION_ID_LEN
#define SCE_MAX_SESSION_ID_LEN 128
#endif

#ifndef SCE_MAX_INVOKES
#define SCE_MAX_INVOKES 4
#endif

#ifndef SCE_MAX_PARALLEL_REGIONS
#define SCE_MAX_PARALLEL_REGIONS 4
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

// How many microsteps one macrostep may take before the engine stops taking
// them. The clause defines a macrostep as a chain of microsteps ending in a
// configuration where nothing is enabled by NULL and the internal queue is
// empty; the specification's Principles and Constraints then say in as many
// words that such a chain need not exist ("A microstep always terminates. A
// macrostep may not. ... This is currently allowed"). So the ceiling is not
// conformance — it is this engine declining a document the specification
// permits, which is why the decline is published rather than merely applied
// (`_truncated_macrosteps`).
//
// One budget for the whole inner loop, not one per branch. Appendix D takes a
// microstep on an eventless transition or on an internal event inside a single
// loop, and a document alternating the two is one chain: budgeting the
// branches separately is what let that chain past both ceilings in every
// engine here until 2026-08-20.
//
// Ten times SCE_MAX_ERROR_CASCADE_DEPTH, and deliberately not equal to it.
// This is the backstop; the cascade ceiling is a diagnostic that names the
// error a handler keeps failing on, and a backstop that fires first makes that
// diagnostic unreachable. Measured 2026-08-20: with both at a hundred, a
// handler that raises one event of its own per link -- two microsteps a link,
// which is what a document that logs before it fails looks like -- was cut at
// fifty links by this ceiling and `_error_cascade_events` never moved. The
// factor of ten is the headroom that keeps the specific report reachable for a
// handler raising up to eight events a link; a busier one is cut here instead,
// which is coarser but still reported.
#ifndef SCE_MAX_MACROSTEP_MICROSTEPS
#define SCE_MAX_MACROSTEP_MICROSTEPS 1000u
#endif

// How many external events one invocation of the main event loop may take off
// the queue before the engine hands control back (`_truncated_event_chains`,
// ARCHITECTURE.md "External-Event Budget", the contract every engine holds).
//
// A machine that answers an event by sending itself the next one, with no
// target, never lets the external queue empty. Every macrostep of it ends, so
// SCE_MAX_MACROSTEP_MICROSTEPS never applies, and the loop takes the next event
// whenever the queue is not empty: the host call that drove it did not return.
// §scxml-3.13 lets a macrostep fail to end and says nothing of a chain of
// macrosteps, so, as with the microstep ceiling, the number is this engine's to
// choose and the decline has to be visible.
//
// This is not the ring's bound. The external queue is a fixed ring
// (SCE_MAX_EVENTS) that drops on overflow, and a chain that sends the next
// event holds one at a time, so the ring never fills and never stopped it.
//
// The default is the one the contract states: three orders of magnitude above
// the longest invocation measured over the authoring suite (6 external events).
// It is a margin and not a proof, and a host that hands a machine a backlog it
// means to work through in one call can choose another
// (`_set_max_external_events_per_call`).
#ifndef SCE_MAX_EXTERNAL_EVENTS_PER_CALL
#define SCE_MAX_EXTERNAL_EVENTS_PER_CALL 10000u
#endif

// docs/adr/0089: whether the machines of this build tell a transition observer
// each transition they take, by the row of their emitted transition table. Off
// by default, and then the preprocessor leaves nothing of the observer in a
// machine: no declaration, no call, no byte of its struct. A machine's own
// `<PREFIX><NAME>_TRANSITION_OBSERVER` takes this value unless the build sets
// that one, so a build can observe one machine of several. A machine that
// observes calls `<prefix><name>_observe_transition`, which the host defines;
// the link refuses a build that turned the observer on and defined none.
#ifndef SCE_TRANSITION_OBSERVER
#define SCE_TRANSITION_OBSERVER 0
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
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
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

/* The bytes a machine keeps for the id it generates for a `<send idlocation>`:
   `_auto_send_` and up to twenty digits, and the terminator. A machine that
   generates one declares a buffer of this size and a count, and asserts that
   `SCE_MAX_ID_LEN` holds it, so an id is never cut short into another's. */
#define SCE_AUTO_SEND_ID_BUF_LEN 32u

/* The id for a `<send idlocation>`, which the document keeps and may name in a
   `<cancel>` later: `_auto_send_` and the number of ids this machine has
   generated, counted from one, written into `buf` of `SCE_AUTO_SEND_ID_BUF_LEN`
   bytes and answered. The count is the machine's own, so two sends of a machine
   never hold one id, and it is formatted here with no `snprintf`, which a
   target of this runtime may not have. */
SCE_C_UNUSED static inline const char *sce_next_auto_send_id(uint64_t *count, char *buf) {
    static const char prefix[] = "_auto_send_";
    char digits[20];
    size_t digit_count = 0u;
    size_t at = sizeof(prefix) - 1u;
    uint64_t number = ++*count;
    do {
        digits[digit_count++] = (char)('0' + (int)(number % 10u));
        number /= 10u;
    } while (number != 0u);
    memcpy(buf, prefix, at);
    while (digit_count != 0u) {
        buf[at++] = digits[--digit_count];
    }
    buf[at] = '\0';
    return buf;
}

/* The largest delay any engine can hold, in milliseconds: the positive
   range of a signed 64-bit count, because Kotlin's `Long` is signed and
   every engine answers the same text the same way. */
#define SCE_MAX_DELAY_MS ((uint64_t)INT64_MAX)

/* §scxml-6.2: a `<send>` delay, read as the CSS2 time the clause names.

   The grammar is ARCHITECTURE.md's "Durations (Single Source of Truth)":
   surrounding ASCII whitespace aside, a non-negative number (digits with an
   optional fraction of at least one digit, or a leading '.' and digits; no
   sign, no exponent) followed directly by "ms" or "s", either case. The
   milliseconds are computed in exact decimal and truncated — integer
   arithmetic throughout, so the freestanding profile needs no
   floating-point unit. tests/durations/css2_time.json holds the cases every
   engine is measured against.

   Answers false when the text is not a time — a bare number included — so
   the caller raises the argument error rather than choosing a wait. */
SCE_C_UNUSED static inline bool sce_parse_delay_ms(const char *text, uint64_t *out_ms) {
    size_t begin = 0u;
    size_t end;
    size_t whole_end;
    size_t i;
    uint64_t scale;
    uint64_t place;
    uint64_t ms = 0u;
    bool has_fraction;

    if (text == NULL || out_ms == NULL) {
        return false;
    }
    end = strlen(text);
    while (begin < end && (text[begin] == ' ' || text[begin] == '\t' || text[begin] == '\n' || text[begin] == '\r' ||
                           text[begin] == '\f' || text[begin] == '\v')) {
        begin++;
    }
    while (end > begin && (text[end - 1u] == ' ' || text[end - 1u] == '\t' || text[end - 1u] == '\n' ||
                           text[end - 1u] == '\r' || text[end - 1u] == '\f' || text[end - 1u] == '\v')) {
        end--;
    }
    if (end - begin >= 2u && (text[end - 2u] == 'm' || text[end - 2u] == 'M') &&
        (text[end - 1u] == 's' || text[end - 1u] == 'S')) {
        scale = 1u;
        end -= 2u;
    } else if (end - begin >= 1u && (text[end - 1u] == 's' || text[end - 1u] == 'S')) {
        scale = 1000u;
        end -= 1u;
    } else {
        return false;
    }
    whole_end = begin;
    while (whole_end < end && text[whole_end] != '.') {
        whole_end++;
    }
    has_fraction = whole_end < end;
    /* A number is digits, digits "." digits, or "." digits: the fraction is
       never empty, and there is at least one digit somewhere. */
    for (i = begin; i < end; i++) {
        if (i == whole_end) {
            continue;
        }
        if (text[i] < '0' || text[i] > '9') {
            return false;
        }
    }
    if (has_fraction && whole_end + 1u == end) {
        return false;
    }
    if (whole_end == begin && !has_fraction) {
        return false;
    }
    for (i = begin; i < whole_end; i++) {
        const uint64_t digit = (uint64_t)(text[i] - '0');
        if (ms > (SCE_MAX_DELAY_MS - digit) / 10u) {
            return false;
        }
        ms = ms * 10u + digit;
    }
    if (ms > SCE_MAX_DELAY_MS / scale) {
        return false;
    }
    ms *= scale;
    /* Only the fraction digits that name whole milliseconds count; the rest
       truncate. */
    place = scale / 10u;
    for (i = whole_end + 1u; has_fraction && i < end && place > 0u; i++) {
        const uint64_t add = (uint64_t)(text[i] - '0') * place;
        if (ms > SCE_MAX_DELAY_MS - add) {
            return false;
        }
        ms += add;
        place /= 10u;
    }
    *out_ms = ms;
    return true;
}

/* §scxml-5.10 `_event.data`: copy the text `src` (NULL for none) whole into
   `dst` of `cap` bytes, or refuse it: false, and `dst` left as it was, when it
   does not fit. An event's data is never cut, since a cut payload parses as
   something its sender never said (docs/adr/0086, section 8.5 (b)); the
   caller makes a refusal visible. */
SCE_C_UNUSED static inline bool sce_copy_text_whole(char *dst, size_t cap, const char *src) {
    const char *s = src != NULL ? src : "";
    const size_t len = strlen(s);
    if (len >= cap) {
        return false;
    }
    memcpy(dst, s, len + 1u);
    return true;
}

/* ── docs/adr/0086, section 8.6 (d): a text that grows in a heap ─────── */

/* An allocator of the shape of Lua's `lua_Alloc`, so a machine whose data model
   is a script engine hands its engine's own and keeps one heap: `new_size` 0
   frees `ptr`, `old_size` bytes, and answers NULL; any other answers a block of
   `new_size` bytes holding what `ptr` held up to the smaller size (`ptr` NULL
   for a new block), or NULL, with `ptr` left as it was, when it cannot. */
typedef void *(*sce_alloc_fn)(void *context, void *ptr, size_t old_size, size_t new_size);

/* §scxml-5.10 `_event.data`: copy the text `src` (NULL or "" for none) whole
   into a text `alloc` gives at its own size, which `*out` then owns, `*out_size`
   bytes with its NUL; for no data `*out` is NULL and `*out_size` 0. False, with
   `*out` NULL, when there is no allocator or it gives nothing: the text is
   refused whole, never cut, and the caller makes the refusal visible. */
SCE_C_UNUSED static inline bool sce_text_alloc_copy(sce_alloc_fn alloc, void *context, const char *src, char **out,
                                                    size_t *out_size) {
    *out = NULL;
    *out_size = 0u;
    if (src == NULL || src[0] == '\0') {
        return true;
    }
    if (alloc == NULL) {
        return false;
    }
    const size_t size = strlen(src) + 1u;
    char *copy = (char *)alloc(context, NULL, 0u, size);
    if (copy == NULL) {
        return false;
    }
    memcpy(copy, src, size);
    *out = copy;
    *out_size = size;
    return true;
}

/* Free the text `*text` owns, `*size` bytes, through the allocator that gave
   it, and clear the holder, so a holder freed twice frees nothing the second
   time. Nothing for a holder that owns no text. */
SCE_C_UNUSED static inline void sce_text_alloc_free(sce_alloc_fn alloc, void *context, char **text, size_t *size) {
    if (*text != NULL && alloc != NULL) {
        (void)alloc(context, *text, *size, 0u);
    }
    *text = NULL;
    *size = 0u;
}

/* A buffer a machine writes an event's data into before it hands the data on,
   grown in a heap to the text it is asked to hold (docs/adr/0086, section 8.6
   (d)): `data` is `cap` bytes the allocator gave, NULL with `cap` 0 before the
   first growth. Its one holder owns it, reuses it from one write to the next
   and frees it once with `sce_grown_text_release`; what it holds means nothing
   once the call it was lent to returns. */
typedef struct {
    char *data;
    size_t cap;
} sce_grown_text_t;

/* Make `buf` hold at least `need` bytes, growing it through `alloc` when it
   holds fewer. True when it holds them; false, with `buf` left as it was, when
   there is no allocator or it gives nothing: the text is then refused whole,
   never cut, and the caller makes the refusal visible. What `buf` held is
   kept up to the smaller size, which no writer relies on. */
SCE_C_UNUSED static inline bool sce_grown_text_reserve(sce_alloc_fn alloc, void *context, sce_grown_text_t *buf,
                                                       size_t need) {
    if (need <= buf->cap) {
        return true;
    }
    if (alloc == NULL) {
        return false;
    }
    char *grown = (char *)alloc(context, buf->data, buf->data != NULL ? buf->cap : 0u, need);
    if (grown == NULL) {
        return false;
    }
    buf->data = grown;
    buf->cap = need;
    return true;
}

/* Free what `buf` holds through the allocator that gave it, and clear it, so a
   buffer released twice frees nothing the second time. */
SCE_C_UNUSED static inline void sce_grown_text_release(sce_alloc_fn alloc, void *context, sce_grown_text_t *buf) {
    if (buf->data != NULL && alloc != NULL) {
        (void)alloc(context, buf->data, buf->cap, 0u);
    }
    buf->data = NULL;
    buf->cap = 0u;
}

/* §scxml-5.10.1 `_event.type` ("internal" / "external" / "platform"). */
SCE_C_UNUSED static inline void sce_copy_bounded_event_type(char *dst, const char *src) {
    sce_copy_bounded_n(dst, src, (size_t)SCE_MAX_EVENT_TYPE_LEN);
}

/* ── docs/adr/0090: the reply address an event carries ─────────────── */

/* The canonical type URI of the SCXML Event I/O Processor, the processor of
   every origin a session's `<send>` writes (docs/adr/0090, section 4). */
#define SCE_SCXML_EVENT_PROCESSOR_TYPE "http://www.w3.org/TR/scxml/#SCXMLEventProcessor"

/* What every SCXML location begins with; the session id follows,
   percent-encoded (`sce_scxml_location`). */
#define SCE_SCXML_LOCATION_PREFIX "sce://scxml/"

/* The reply address an event carries (docs/adr/0090, section 1): the Event I/O
   Processor that delivered it and the address that processor accepts as a
   `<send>` target. The document reads the two as `_event.origintype` and
   `_event.origin`; they are one value because the address means something only
   relative to its processor.

   The processor that delivers an event writes the pair once, where it builds
   the event (section 2), through `sce_event_origin_set` or one of the writers
   below, and nothing converts it later. An event that no processor stamped
   holds none: both fields empty, which is what a zeroed event holds, so an
   event built with `memset` carries no origin until a processor names one.

   Sized as a URI and an id, the bounds this runtime holds those in: the address
   is a location a session published (`SCE_MAX_URI_LEN`, as
   `basic_http_access_uri`), and the processor a type URI or a host's declared
   type (`SCE_MAX_ID_LEN`). A pair that does not fit is refused whole, never cut:
   a truncated address still looks like one and names nobody. */
typedef struct sce_event_origin_s {
    char processor[SCE_MAX_ID_LEN];
    char address[SCE_MAX_URI_LEN];
} sce_event_origin_t;

/* No origin: both fields empty. */
SCE_C_UNUSED static inline void sce_event_origin_clear(sce_event_origin_t *origin) {
    origin->processor[0] = '\0';
    origin->address[0] = '\0';
}

/* Whether `origin` holds none. NULL holds none. */
SCE_C_UNUSED static inline bool sce_event_origin_is_none(const sce_event_origin_t *origin) {
    return origin == NULL || (origin->processor[0] == '\0' && origin->address[0] == '\0');
}

/* `_event.origin` of an event that carries `origin`: its address, or the empty
   string when it has none. */
SCE_C_UNUSED static inline const char *sce_event_origin_address(const sce_event_origin_t *origin) {
    return origin != NULL ? origin->address : "";
}

/* `_event.origintype` of an event that carries `origin`: its processor, or the
   empty string when it has none. */
SCE_C_UNUSED static inline const char *sce_event_origin_processor(const sce_event_origin_t *origin) {
    return origin != NULL ? origin->processor : "";
}

/* Write the pair `{processor, address}` whole. False, leaving `origin` none, when
   either does not fit its field: a processor names a party only together with
   its address, so neither is kept without the other. NULL reads as empty. */
SCE_C_UNUSED static inline bool sce_event_origin_set(sce_event_origin_t *origin, const char *processor,
                                                     const char *address) {
    const char *p = processor != NULL ? processor : "";
    const char *a = address != NULL ? address : "";
    const size_t p_len = strlen(p);
    const size_t a_len = strlen(a);
    if (p_len >= sizeof(origin->processor) || a_len >= sizeof(origin->address)) {
        sce_event_origin_clear(origin);
        return false;
    }
    memcpy(origin->processor, p, p_len + 1u);
    memcpy(origin->address, a, a_len + 1u);
    return true;
}

/* A byte an SCXML location carries as it is — RFC 3986's unreserved set; every
   other byte is percent-encoded. */
SCE_C_UNUSED static inline bool sce_scxml_location_keeps_(unsigned char c) {
    return (c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z') || (c >= '0' && c <= '9') || c == '-' || c == '_' ||
           c == '.' || c == '~';
}

/* The value of the hex digit `c`, or -1 for a byte that is not one. */
SCE_C_UNUSED static inline int sce_scxml_location_hex_(unsigned char c) {
    if (c >= '0' && c <= '9') {
        return (int)(c - '0');
    }
    if (c >= 'A' && c <= 'F') {
        return (int)(c - 'A') + 10;
    }
    if (c >= 'a' && c <= 'f') {
        return (int)(c - 'a') + 10;
    }
    return -1;
}

/* The bytes of the SCXML location of `session_id`, its terminator not counted. */
SCE_C_UNUSED static inline size_t sce_scxml_location_len(const char *session_id) {
    size_t n = sizeof(SCE_SCXML_LOCATION_PREFIX) - 1u;
    for (const unsigned char *s = (const unsigned char *)(session_id != NULL ? session_id : ""); *s != '\0'; ++s) {
        n += sce_scxml_location_keeps_(*s) ? 1u : 3u;
    }
    return n;
}

/* §scxml-5.10.1: the location a session publishes in `_ioprocessors` and every
   event it sends names as its origin — `sce://scxml/` and the session id,
   percent-encoded as every other runtime encodes it (C++
   `IOProcessorHelper::scxmlLocation`). The one encoder of the address: the
   `_ioprocessors` entry and the origins a session's sends carry both come from
   it, so the two cannot spell one session differently.

   Written into `dst` of `cap` bytes. False, leaving `dst` empty, when it does
   not fit. */
SCE_C_UNUSED static inline bool sce_scxml_location(char *dst, size_t cap, const char *session_id) {
    static const char hex[] = "0123456789ABCDEF";
    const size_t prefix_len = sizeof(SCE_SCXML_LOCATION_PREFIX) - 1u;
    size_t at = prefix_len;
    if (cap == 0u) {
        return false;
    }
    if (sce_scxml_location_len(session_id) >= cap) {
        dst[0] = '\0';
        return false;
    }
    memcpy(dst, SCE_SCXML_LOCATION_PREFIX, prefix_len);
    for (const unsigned char *s = (const unsigned char *)(session_id != NULL ? session_id : ""); *s != '\0'; ++s) {
        if (sce_scxml_location_keeps_(*s)) {
            dst[at++] = (char)*s;
        } else {
            dst[at++] = '%';
            dst[at++] = hex[*s >> 4];
            dst[at++] = hex[*s & 0x0Fu];
        }
    }
    dst[at] = '\0';
    return true;
}

/* The session id the SCXML location `uri` names — the inverse of
   `sce_scxml_location`, kept beside it so the two spellings of one address
   cannot drift apart (C++ `IOProcessorHelper::sessionIdFromScxmlLocation`).

   False when `uri` is not an SCXML location, names no session (nothing after
   the prefix), or names an id longer than `cap` holds; `dst` is left empty then.
   A `%` without two hex digits behind it is kept as it is rather than dropped:
   the input is an address, and deleting part of one turns "no such session"
   into "a different session". */
SCE_C_UNUSED static inline bool sce_session_id_from_scxml_location(char *dst, size_t cap, const char *uri) {
    const size_t prefix_len = sizeof(SCE_SCXML_LOCATION_PREFIX) - 1u;
    const unsigned char *s;
    size_t at = 0u;
    if (cap == 0u) {
        return false;
    }
    dst[0] = '\0';
    if (uri == NULL || strncmp(uri, SCE_SCXML_LOCATION_PREFIX, prefix_len) != 0 || uri[prefix_len] == '\0') {
        return false;
    }
    for (s = (const unsigned char *)uri + prefix_len; *s != '\0'; ++s) {
        unsigned char c = *s;
        if (c == '%' && sce_scxml_location_hex_(s[1]) >= 0 && sce_scxml_location_hex_(s[2]) >= 0) {
            c = (unsigned char)((sce_scxml_location_hex_(s[1]) << 4) | sce_scxml_location_hex_(s[2]));
            s += 2;
        }
        if (at + 1u >= cap) {
            dst[0] = '\0';
            return false;
        }
        dst[at++] = (char)c;
    }
    dst[at] = '\0';
    return true;
}

/* docs/adr/0090, section 2: the origin the SCXML Event I/O Processor writes for
   an event the session `sender_session_id` sends — its own type URI and the
   location that session publishes (`sce_scxml_location`). A sender that holds
   no session id has no address to publish, so the event carries none. False,
   leaving `origin` none, when the location does not fit. */
SCE_C_UNUSED static inline bool sce_event_origin_scxml(sce_event_origin_t *origin, const char *sender_session_id) {
    if (sender_session_id == NULL || sender_session_id[0] == '\0') {
        sce_event_origin_clear(origin);
        return true;
    }
    if (!sce_scxml_location(origin->address, sizeof(origin->address), sender_session_id)) {
        sce_event_origin_clear(origin);
        return false;
    }
    memcpy(origin->processor, SCE_SCXML_EVENT_PROCESSOR_TYPE, sizeof(SCE_SCXML_EVENT_PROCESSOR_TYPE));
    return true;
}

/* docs/adr/0090, section 2: the SCXML Event I/O Processor stamps an event a
   session sends — the pair of `session_id` into `origin`, and `from_session`,
   the one byte an event record and a forwarded record carry beside the pair,
   set to say that this processor wrote it. A sender that holds no id stamps
   none and leaves the flag clear. The provenance is the event's, not the
   pair's: a host may spell any pair it likes, a session's location included,
   and the event it posts is still no session's send. */
SCE_C_UNUSED static inline void sce_event_stamp_session(sce_event_origin_t *origin, uint8_t *from_session,
                                                        const char *session_id) {
    *from_session =
        (uint8_t)((sce_event_origin_scxml(origin, session_id) && !sce_event_origin_is_none(origin)) ? 1u : 0u);
}

/* docs/adr/0090, section 1: the session that sent an event, the id the
   session-keyed lookups (`<finalize>`, a saved state's `sender`) read, written
   into `dst` of `cap` bytes. The sender is the id of the session that sent the
   event, written by the processor that delivered it from that session: the
   SCXML processor for a local session, and the Mesh transport for a remote
   child or parent, beside that session's endpoint.

   Not stored beside the pair on this backend, which is a bound of C11, not a
   rule of the format: C11 runs no remote child, so the one processor here that
   delivers an event from a session is the SCXML processor. It marks the event
   by `from_session` (`sce_event_stamp_session`), and the sender it writes is
   exactly the session its location names, so the id is read back out of the
   pair where it is needed instead of costing every queued event a second copy.
   False, leaving `dst` empty, for an event no session sent — a host's,
   whatever pair it spells, a Mesh peer's, one with no origin — and for an id
   longer than `cap` holds. */
SCE_C_UNUSED static inline bool sce_event_origin_sender(const sce_event_origin_t *origin, uint8_t from_session,
                                                        char *dst, size_t cap) {
    if (cap == 0u) {
        return false;
    }
    dst[0] = '\0';
    if (from_session == 0u || origin == NULL || strcmp(origin->processor, SCE_SCXML_EVENT_PROCESSOR_TYPE) != 0) {
        return false;
    }
    return sce_session_id_from_scxml_location(dst, cap, origin->address);
}

/* The canonical type URI of the BasicHTTP Event I/O Processor. */
#define SCE_BASIC_HTTP_EVENT_PROCESSOR_TYPE "http://www.w3.org/TR/scxml/#BasicHTTPEventProcessor"

/* docs/adr/0090, section 4: the processor a `<send type>` spelling names. The
   short aliases `_ioprocessors` files each processor under (`scxml`,
   `basichttp`) are read as their type URIs, so the type a send is checked
   against, and the origintype its event carries, is never the author's
   spelling (C++ `IOProcessorHelper::canonicalProcessorType`, the build's
   `canonical_send_type`). Any other value is answered as written. */
SCE_C_UNUSED static inline const char *sce_canonical_processor_type(const char *send_type) {
    if (send_type != NULL && strcmp(send_type, "scxml") == 0) {
        return SCE_SCXML_EVENT_PROCESSOR_TYPE;
    }
    if (send_type != NULL && strcmp(send_type, "basichttp") == 0) {
        return SCE_BASIC_HTTP_EVENT_PROCESSOR_TYPE;
    }
    return send_type;
}

/* §scxml-6.4: the function a child session reaches the session that invoked it
   through — its `<send target="#_parent">`, at once or when a delayed one comes
   due. `parent_sm` is the parent as the parent handed itself over, `invoke_id`
   the invocation the child was started under, `sender` the child's own session
   id, from which the parent writes the event's origin (docs/adr/0090, section
   2), and `event_data` the payload, NULL for none. */
typedef void (*sce_parent_dispatch_fn)(void *parent_sm, const char *event_name, const char *invoke_id,
                                       const char *sender, const char *event_data);

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
    /* The event's data, a view of text the sender holds, valid for the one
       synchronous call the record is handed to; NULL and "" are no data. The
       receiving machine copies it into its own slot of `_event.data` and
       refuses, counted and raised, data that does not fit (docs/adr/0086,
       section 8.5 (b)). */
    const char *data;
    /* `_event.origin` and `_event.origintype`, the one pair the delivering
       processor wrote (docs/adr/0090). A host posting an event by name names
       its origin here in its own terms, or none. */
    sce_event_origin_t origin;
    /* 1 when the SCXML processor stamped the pair for a sending session
       (`sce_event_stamp_session`), whose id is then read out of it
       (`sce_event_origin_sender`); 0 for every other event. */
    uint8_t from_session;
    char send_id[SCE_MAX_ID_LEN];
    char type[SCE_MAX_EVENT_TYPE_LEN];
    char invoke_id[SCE_MAX_ID_LEN];
} sce_forwarded_event_t;

#endif  // SCE_TYPES_H
