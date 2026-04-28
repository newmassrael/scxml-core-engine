// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML 6.4: invoke lifecycle types + helpers.
//
// Per c11_design_decisions.md (T3 lock-in + INTERFACE-only sce-c-runtime):
// types and `static inline` helpers live here; per-SM lifecycle logic
// (child SM creation, completion detection, event routing) is emitted
// by `tools/codegen/templates/c/invoke_methods.jinja2` into each
// generated `_sm.c`. The cpp equivalent is `sce/include/core/InvokeHelper.h`
// — same shape (template-instantiated helpers reused across every SM)
// translated to C11 idioms (static inline functions in header + jinja2
// inline emit).

#ifndef SCE_INVOKE_H
#define SCE_INVOKE_H

#include <stdint.h>
#include <stddef.h>
#include <string.h>
#include <stdio.h>

#include "sce/types.h"

#ifdef __cplusplus
extern "C" {
#endif

/* ── W3C SCXML 6.4: Pending invoke record ─────────────────────────── */
/* Mirrors cpp `PendingInvoke` (sce/include/core/InvokeHelper.h doc):
   `state` is the parent's State enum value (per-fixture int) and
   `invoke_idx` is the per-state index into the state's invokes vector.
   The pair (state, invoke_idx) keys the codegen-time switch in the
   generated `<sm>_execute_pending_invokes` that dispatches to the right
   child SM creation arm. `invoke_id` carries the W3C SCXML 3.12.1
   identifier (auto-generated `<state>.<platformid>._invoke_<n>` or
   user-provided). */
typedef struct sce_invoke_pending_s {
    int state;
    int invoke_idx;
    char invoke_id[SCE_MAX_ID_LEN];
} sce_invoke_pending_t;

/* Bounded queue — per-SM pending invokes. SCE_MAX_INVOKES (default 4)
   is the max simultaneous invokes a parent may carry; the corpus max is
   3 (test422 has 3 invoke sites in s1/s11/s12). MCU profiles override
   via `-DSCE_MAX_INVOKES=N`. */
typedef struct sce_invoke_pending_queue_s {
    sce_invoke_pending_t entries[SCE_MAX_INVOKES];
    int count;
} sce_invoke_pending_queue_t;

/* W3C SCXML 6.4: Defer invoke until macrostep end.
   Saturates silently at SCE_MAX_INVOKES — corpus is bounded; production
   overflow rolls a build-time -D bump. cpp's std::vector grows; C11's
   bounded array matches the runtime/scheduler discipline applied to
   every other queue in this engine (event_queue, scheduled_queue). */
SCE_C_UNUSED static inline void
sce_invoke_pending_push(sce_invoke_pending_queue_t *q,
                        int state,
                        int invoke_idx,
                        const char *invoke_id) {
    if (q->count >= (int)SCE_MAX_INVOKES) {
        return;
    }
    q->entries[q->count].state = state;
    q->entries[q->count].invoke_idx = invoke_idx;
    if (invoke_id != NULL) {
        strncpy(q->entries[q->count].invoke_id,
                invoke_id,
                (size_t)SCE_MAX_ID_LEN - 1u);
        q->entries[q->count].invoke_id[(size_t)SCE_MAX_ID_LEN - 1u] = '\0';
    } else {
        q->entries[q->count].invoke_id[0] = '\0';
    }
    q->count++;
}

/* W3C SCXML 6.4: Cancel pending invokes for an exited state.
   Compaction in place (cpp uses std::remove_if + erase; C11 walks the
   bounded array). Multiple invokes per state are valid (test422), so
   the walk does not stop at the first match. */
SCE_C_UNUSED static inline void
sce_invoke_pending_cancel_for_state(sce_invoke_pending_queue_t *q, int state) {
    int w = 0;
    for (int r = 0; r < q->count; r++) {
        if (q->entries[r].state != state) {
            if (r != w) {
                q->entries[w] = q->entries[r];
            }
            w++;
        }
    }
    q->count = w;
}

/* W3C SCXML 6.4: Clear queue after execution.
   Used by `<sm>_execute_pending_invokes` to reset between macrosteps —
   matches cpp `pending.clear()` after copying for safe iteration. */
SCE_C_UNUSED static inline void
sce_invoke_pending_clear(sce_invoke_pending_queue_t *q) {
    q->count = 0;
}

/* W3C SCXML 3.12.1: Format auto-generated invoke ID.
   cpp pattern: `<state_id>.<runtime_platformid>.<invoke_index>`. The
   platformid in cpp is the SM's `this` pointer hex; C11 mirrors with
   the parent SM struct address. test224's transition cond
   (`Var1.indexOf('s0.') === 0`) verifies this format begins with the
   state id and a dot. Caller-provided buffer must be sized for the
   formatted string; buf[SCE_MAX_ID_LEN] is the conventional sizing
   (matches `sce_invoke_pending_t::invoke_id`). */
SCE_C_UNUSED static inline void
sce_invoke_format_id(char *buf,
                     size_t bufsz,
                     const char *state_id,
                     const void *sm_ptr,
                     int invoke_idx) {
    if (buf == NULL || bufsz == 0u) {
        return;
    }
    (void)snprintf(buf, bufsz, "%s.%lx._invoke_%d",
                   state_id != NULL ? state_id : "",
                   (unsigned long)(uintptr_t)sm_ptr,
                   invoke_idx);
}

/* W3C SCXML 6.3.1: Format `done.invoke.<id>` event name.
   cpp `InvokeHelper::createDoneInvokeEventName`. Used by the parent's
   completion detector when a child reaches its top-level final state.
   The bare `done.invoke` transition match (analyzer.rs collapses
   `done.invoke` and `done.invoke.<id>` to the same Event enum) reads
   the event itself; the fully-qualified name is stashed onto the
   event's `invoke_id` metadata field so a transition cond inspecting
   `_event.invokeid` can select on it. */
SCE_C_UNUSED static inline void
sce_invoke_format_done_event_name(char *buf, size_t bufsz, const char *invoke_id) {
    if (buf == NULL || bufsz == 0u) {
        return;
    }
    (void)snprintf(buf, bufsz, "done.invoke.%s",
                   invoke_id != NULL ? invoke_id : "");
}

#ifdef __cplusplus
}
#endif

#endif /* SCE_INVOKE_H */
