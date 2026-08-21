// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// §scxml-6.4.3: an `<invoke>` `<param>` seeds a declared `<data>` of the
// invoked session with the INVOKING session's value — C11 AOT channel.
//
// This backend is the one that already answered both halves of the clause:
// `lua_xfer_top_to_child` moves the evaluated VALUE across the two Lua
// states rather than re-evaluating source in the child, and the emission is
// gated on `param.name in invoke_info.child_datamodel_vars`. Its own
// comment records why ("a parent-side variable name is undefined in the
// child's state ... for literal exprs both shapes converge"), which is the
// rule the other channels are being brought up to. Without a driver here
// that correctness is an unasserted property — the shape a channel loses
// first is the one no test names.
//
// Fixture: integration_resources/invoke_param_seeds_declared_child_data/invoke_param_seeds_declared_child_data.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(invoke_param_seeds_declared_child_data ...)`
// in `backends/c/tests/CMakeLists.txt`. The build itself is the §6.2.6
// freshness invariant — there is no committed tree for the c11 backend.

#include <stdint.h>
#include <stdio.h>

#include "invoke_param_seeds_declared_child_data_sm.h"

int main(void) {
    invoke_param_seeds_declared_child_data_t sm;
    invoke_param_seeds_declared_child_data_init(&sm);

    // No `<send delay>` in this fixture — each inline child reaches its
    // top-level `<final>` during its own `_init`, the parent's
    // `execute_pending_invokes` raises `done.invoke` onto the external
    // queue, and the next drain dispatches it. Four times over, all
    // inside `_run`. No scheduler / polling needed.
    invoke_param_seeds_declared_child_data_run(&sm);

    const int pass =
        invoke_param_seeds_declared_child_data_in_state(&sm, INVOKE_PARAM_SEEDS_DECLARED_CHILD_DATA_STATE_PASS);
    if (!pass) {
        fprintf(stderr,
                "invoke_param_seeds_declared_child_data: FAIL — §scxml-6.4.3 is not held on the "
                "C11 AOT engine. Which verdict state was reached says which sentence broke: "
                "child_evaluated_the_expression=%d (the child evaluated the author's `<param "
                "expr>` text in its own Lua state instead of receiving the invoking session's "
                "value) parent_only_expr_lost=%d (same defect, no shadow to find) "
                "unmatched_param_entered_the_child=%d (a param naming no top-level `<data>` of "
                "the child became a global there) namelist_value_lost=%d (the namelist value did "
                "not arrive) shadow_seed_lost=%d declared_param_lost=%d (the child saw neither "
                "the parent's value nor a shadow, so its own `<data>` default stood — nothing "
                "was seeded at all)\n",
                invoke_param_seeds_declared_child_data_in_state(
                    &sm, INVOKE_PARAM_SEEDS_DECLARED_CHILD_DATA_STATE_FAILCHILDEVALUATEDTHEEXPRESSION),
                invoke_param_seeds_declared_child_data_in_state(
                    &sm, INVOKE_PARAM_SEEDS_DECLARED_CHILD_DATA_STATE_FAILPARENTONLYEXPRLOST),
                invoke_param_seeds_declared_child_data_in_state(
                    &sm, INVOKE_PARAM_SEEDS_DECLARED_CHILD_DATA_STATE_FAILUNMATCHEDPARAMENTEREDTHECHILD),
                invoke_param_seeds_declared_child_data_in_state(
                    &sm, INVOKE_PARAM_SEEDS_DECLARED_CHILD_DATA_STATE_FAILNAMELISTVALUELOST),
                invoke_param_seeds_declared_child_data_in_state(
                    &sm, INVOKE_PARAM_SEEDS_DECLARED_CHILD_DATA_STATE_FAILSHADOWSEEDLOST),
                invoke_param_seeds_declared_child_data_in_state(
                    &sm, INVOKE_PARAM_SEEDS_DECLARED_CHILD_DATA_STATE_FAILDECLAREDPARAMLOST));
    }
    invoke_param_seeds_declared_child_data_destroy(&sm);
    return pass ? 0 : 1;
}
