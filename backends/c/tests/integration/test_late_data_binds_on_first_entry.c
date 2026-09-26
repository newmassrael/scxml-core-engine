// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.3 / Appendix D enterStates, late binding: a state's <data> is
// bound on that state's FIRST entry and never again — C11 AOT.
//
// `s` is entered, its `v` changed to 5, `s` left and entered again; its
// <onentry> records the `v` it sees each time. Measured 2026-09-26, this
// channel bound late data on every entry — deliberately, copying the C++ AOT —
// so the second entry saw 1 again.
//
// Fixture: integration_resources/late_data_binds_on_first_entry/late_data_binds_on_first_entry.scxml
// (canonical, shared with the C++ / Rust / Go / Kotlin / Python channels).
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(late_data_binds_on_first_entry ...)`
// in `backends/c/tests/CMakeLists.txt`.

#include <stdio.h>

#include "late_data_binds_on_first_entry_sm.h"

typedef late_data_binds_on_first_entry_t sm_t;

/// What the handlers recorded (W3C SCXML 5.3 accessors).
static void report(const sm_t *sm) {
    int64_t entries = -1, seen = -1, content_seen = -1;
    (void)late_data_binds_on_first_entry_entries(sm, &entries);
    (void)late_data_binds_on_first_entry_seen(sm, &seen);
    (void)late_data_binds_on_first_entry_content_seen(sm, &content_seen);
    fprintf(stderr, "       entries=%lld seen=%lld contentSeen=%lld (wanted 2 / 15 / 7)\n", (long long)entries,
            (long long)seen, (long long)content_seen);
}

static void deliver(sm_t *sm, late_data_binds_on_first_entry_event_t event) {
    late_data_binds_on_first_entry_event_with_meta_t carrier = {0};
    carrier.event = event;
    late_data_binds_on_first_entry_raise_external(sm, &carrier);
    late_data_binds_on_first_entry_run(sm);
}

static int expect_record(const sm_t *sm, const char *name, bool (*reader)(const sm_t *, int64_t *), int64_t want,
                         const char *why) {
    int64_t value = -1;
    if (!reader(sm, &value) || value != want) {
        fprintf(stderr, "FAIL: %s = %lld, want %lld: %s\n", name, (long long)value, (long long)want, why);
        report(sm);
        return 0;
    }
    return 1;
}

int main(void) {
    sm_t sm;
    late_data_binds_on_first_entry_init(&sm);
    late_data_binds_on_first_entry_run(&sm);
    if (!late_data_binds_on_first_entry_in_state(&sm, LATE_DATA_BINDS_ON_FIRST_ENTRY_STATE_IDLE)) {
        fprintf(stderr, "FAIL: the run has to start in `idle`\n");
        return 1;
    }

    // Enter `s`, change its `v`, leave it, enter it again — one step each.
    deliver(&sm, LATE_DATA_BINDS_ON_FIRST_ENTRY_EVENT_GO);
    deliver(&sm, LATE_DATA_BINDS_ON_FIRST_ENTRY_EVENT_BUMP);
    deliver(&sm, LATE_DATA_BINDS_ON_FIRST_ENTRY_EVENT_BACK);
    deliver(&sm, LATE_DATA_BINDS_ON_FIRST_ENTRY_EVENT_GO);
    if (!late_data_binds_on_first_entry_in_state(&sm, LATE_DATA_BINDS_ON_FIRST_ENTRY_STATE_S)) {
        fprintf(stderr, "FAIL: the second `go` has to leave the machine in `s`\n");
        report(&sm);
        return 1;
    }

    int ok =
        expect_record(&sm, "entries", late_data_binds_on_first_entry_entries, 2, "both entries of `s` must have run");
    ok &= expect_record(&sm, "seen", late_data_binds_on_first_entry_seen, 15,
                        "`s` saw v=1 on its first entry and must see the 5 it was changed to on its second: "
                        "11 is a processor that binds late data on every entry, and no value at all one "
                        "that never binds it");
    ok &= expect_record(&sm, "contentSeen", late_data_binds_on_first_entry_content_seen, 7,
                        "`c` is bound from inline content, not an expr, and must be bound too");
    late_data_binds_on_first_entry_destroy(&sm);
    if (!ok) {
        return 1;
    }
    printf("PASS: a late-binding state bound its <data> on its first entry only\n");
    return 0;
}
