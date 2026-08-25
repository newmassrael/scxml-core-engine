// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML Appendix D: a `<parallel>` is not a transition domain — C11 AOT.
//
// `getTransitionDomain` sends an external transition to `findLCCA`, which
// filters the PROPER ancestors of the source with
// `isCompoundStateOrScxmlElement`. A `<parallel>` is neither a compound
// `<state>` nor the `<scxml>` element, so it is not a candidate: an external
// transition written on a REGION ROOT resolves to the document root. Every
// region exits and re-enters, and a sibling region's transition on the same
// event is preempted because the two exit sets intersect and the sibling's
// source is not a descendant of this one's.
//
// This engine answered the enclosing `<parallel>` instead. `find_lcca` walked
// the proper ancestors and returned the first one containing the target,
// whatever its kind — the `findLCA` the appendix distinguishes from
// `findLCCA`. The difference is invisible until a `<parallel>` sits between
// the source and the first compound `<state>` above it, which is exactly what
// a region root is.
//
// The C11 sibling of `tests/integration/ParallelRegionRootExternalDomain{,Aot}Test.cpp`,
// `backends/rust/tests/tests/parallel_region_root_external_domain.rs`,
// `backends/go/tests/integration/parallel_region_root_external_domain/` and
// `backends/python/tests/integration/parallel_region_root_external_domain/`,
// asking the same two questions of the same document so that a domain one
// engine resolves is the domain the others resolve.
//
// The observable is the WHOLE configuration, rendered and compared as text,
// not membership. The defect's actual shape is a `<parallel>` left holding a
// region that never came back — an illegal configuration in which a
// per-state `_in_state` answers true for every state the correct answer also
// holds. `_validate_configuration` is asked the same question a second way,
// because a set that is not a configuration of this document is a failure
// even when it happens to contain the state under test.
//
// Fixture: tests/integration/parallel_region_root_external_domain.scxml
// (shared verbatim with the four channels named above; it sits beside its
// C++ test rather than under `integration_resources/` because a stem there is
// a seven-channel commitment).
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(parallel_region_root_external_domain ...)`
// in `backends/c/tests/CMakeLists.txt`.

#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "parallel_region_root_external_domain_sm.h"

#define D_(suffix) PARALLEL_REGION_ROOT_EXTERNAL_DOMAIN_##suffix

typedef parallel_region_root_external_domain_state_t d_state_t;

static int failures = 0;

/* The active configuration as text, in enum order — which for this document is
   its states in alphabetical order, so the rendering is canonical without a
   sort. Ancestors are included exactly as the bitmap holds them: `run` and
   `drive` being present is part of what is under test, not scaffolding. */
static void describe(const parallel_region_root_external_domain_t *sm, char *out, size_t cap) {
    size_t used = 0u;
    out[0] = '\0';
    for (uint32_t i = 0u; i < (uint32_t)D_(STATE_COUNT); ++i) {
        if (!parallel_region_root_external_domain_in_state(sm, (d_state_t)i)) {
            continue;
        }
        const char *name = parallel_region_root_external_domain_state_name((d_state_t)i);
        const char *sep = (used == 0u) ? "" : " | ";
        int written = snprintf(out + used, cap - used, "%s%s", sep, name);
        if (written < 0 || (size_t)written >= cap - used) {
            return;
        }
        used += (size_t)written;
    }
}

/* Compares the whole configuration, and asks the document's own validator
   whether the set is a configuration at all. The two failures read
   differently on purpose: a mismatch names which states moved, a rejection
   names which rule of §3.2/3.3/3.4 the engine broke getting there. */
static void expect_configuration(const char *what, const parallel_region_root_external_domain_t *sm,
                                 const char *expected, const char *why) {
    char got[256];
    describe(sm, got, sizeof(got));

    d_state_t held[D_(STATE_COUNT)];
    size_t count = 0u;
    for (uint32_t i = 0u; i < (uint32_t)D_(STATE_COUNT); ++i) {
        if (parallel_region_root_external_domain_in_state(sm, (d_state_t)i)) {
            held[count++] = (d_state_t)i;
        }
    }
    parallel_region_root_external_domain_configuration_rejection_t verdict =
        parallel_region_root_external_domain_validate_configuration(held, count);

    if (verdict != D_(CONFIG_NONE)) {
        fprintf(stderr,
                "FAIL: %s left a set that is not a configuration of the document\n"
                "  active:   [%s]\n"
                "  rejected: %s\n"
                "  %s\n",
                what, got, parallel_region_root_external_domain_configuration_rejection_text(verdict), why);
        ++failures;
        return;
    }
    if (strcmp(got, expected) != 0) {
        fprintf(stderr,
                "FAIL: %s\n"
                "  expected: [%s]\n"
                "  active:   [%s]\n"
                "  %s\n",
                what, expected, got, why);
        ++failures;
    }
}

/* The clause itself: an external region-root transition has the DOCUMENT ROOT
   as its domain. `watch` exits with everything else and comes back at its
   default `alive`, and its own transition on `restart` is preempted. Reading
   `rebuilding` here means the domain came back as `run` (or as `drive`) and
   `watch` was never exited. */
static void an_external_region_root_transition_exits_every_region(void) {
    parallel_region_root_external_domain_t sm;
    parallel_region_root_external_domain_init(&sm);
    parallel_region_root_external_domain_run(&sm);

    expect_configuration("the fixture's initial configuration", &sm, "alive | drive | run | watch | working",
                         "nothing below tests what it claims if the machine did not start here");

    parallel_region_root_external_domain_event_with_meta_t restart = {0};
    restart.event = D_(EVENT_RESTART);
    parallel_region_root_external_domain_raise_external(&sm, &restart);
    parallel_region_root_external_domain_run(&sm);

    expect_configuration("`restart` on an external region-root transition", &sm,
                         "alive | drive | restarting | run | watch",
                         "Appendix D findLCCA filters <parallel> out of the candidates, so the domain is the "
                         "document root: every region exits and re-enters, `watch` is back at its default, and "
                         "`watch`'s own transition on `restart` is preempted by document order");
}

/* The contrast, and the reason the ai_loop document spells `type="internal"`.
   A test pinning only the external case would pass on an engine that sent
   EVERY region-root transition to the document root. Here the source is
   compound and the target is its descendant, so the domain is `drive`:
   `watch` never exits and answers the event itself. */
static void an_internal_region_root_transition_leaves_the_other_region(void) {
    parallel_region_root_external_domain_t sm;
    parallel_region_root_external_domain_init(&sm);
    parallel_region_root_external_domain_run(&sm);

    parallel_region_root_external_domain_event_with_meta_t hold = {0};
    hold.event = D_(EVENT_HOLD);
    parallel_region_root_external_domain_raise_external(&sm, &hold);
    parallel_region_root_external_domain_run(&sm);

    expect_configuration("`hold` on an internal region-root transition", &sm,
                         "drive | paused | rebuilding | run | watch",
                         "an internal transition whose target descends from its compound source has that source "
                         "as its domain, so the sibling region is untouched and keeps its own answer to `hold`");
}

int main(void) {
    an_external_region_root_transition_exits_every_region();
    an_internal_region_root_transition_leaves_the_other_region();

    if (failures != 0) {
        fprintf(stderr, "FAIL: %d configuration(s) were not what W3C SCXML Appendix D resolves\n", failures);
        return 1;
    }
    printf("PASS: a <parallel> was not a transition domain, and an internal region-root transition kept one\n");
    return 0;
}
