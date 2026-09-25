// SCE-MAP: algorithm_checked_arith:18 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm") */
/* Runtime: sce_forge_runtime */
/* Do not edit — regenerate from the source SCXML file. */
/* */
/* RFC §synth-5-A: free function with bounded loops, no allocs, no I/O. */
/* RFC §synth-5-J-5 emitter table: `static T <snake>(...)` with `bytes` */
/* lowered to the borrowed `sce_forge_bytes_view_t` (zero-copy). */

#ifndef SCE_FORGE_ALGORITHM_CHECKED_ARITH_H
#define SCE_FORGE_ALGORITHM_CHECKED_ARITH_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include "sce/forge/algorithm.h"  /* checked arithmetic, SCE_FORGE.md §3.4.1 */
typedef struct {
    int32_t value;
    bool ok;
    sce_forge_algorithm_error_t why;
} algorithm_checked_arith_result_t;

static inline algorithm_checked_arith_result_t algorithm_checked_arith(int32_t a, int32_t b, uint8_t op) {
    /* SCE_FORGE.md §3.4.1: each checked operation records a failure here,
     * and the statement around it returns it before the next statement runs. */
    sce_forge_algorithm_failure_t sce_failure_ = { false, SCE_FORGE_ALGORITHM_OVERFLOW };
    int32_t r = 0;
    if (sce_failure_.failed) { return (algorithm_checked_arith_result_t){ .ok = false, .why = sce_failure_.error }; }
    {
        const bool sce_cond1_ = op == 0;
        if (sce_failure_.failed) { return (algorithm_checked_arith_result_t){ .ok = false, .why = sce_failure_.error }; }
        if (sce_cond1_) {
            r = sce_forge_checked_add_i32(&sce_failure_, a, b);
            if (sce_failure_.failed) { return (algorithm_checked_arith_result_t){ .ok = false, .why = sce_failure_.error }; }
        }
    }
    {
        const bool sce_cond1_ = op == 1;
        if (sce_failure_.failed) { return (algorithm_checked_arith_result_t){ .ok = false, .why = sce_failure_.error }; }
        if (sce_cond1_) {
            r = sce_forge_checked_sub_i32(&sce_failure_, a, b);
            if (sce_failure_.failed) { return (algorithm_checked_arith_result_t){ .ok = false, .why = sce_failure_.error }; }
        }
    }
    {
        const bool sce_cond1_ = op == 2;
        if (sce_failure_.failed) { return (algorithm_checked_arith_result_t){ .ok = false, .why = sce_failure_.error }; }
        if (sce_cond1_) {
            r = sce_forge_checked_mul_i32(&sce_failure_, a, b);
            if (sce_failure_.failed) { return (algorithm_checked_arith_result_t){ .ok = false, .why = sce_failure_.error }; }
        }
    }
    {
        const bool sce_cond1_ = op == 3;
        if (sce_failure_.failed) { return (algorithm_checked_arith_result_t){ .ok = false, .why = sce_failure_.error }; }
        if (sce_cond1_) {
            r = sce_forge_checked_div_i32(&sce_failure_, a, b);
            if (sce_failure_.failed) { return (algorithm_checked_arith_result_t){ .ok = false, .why = sce_failure_.error }; }
        }
    }
    {
        const bool sce_cond1_ = op == 4;
        if (sce_failure_.failed) { return (algorithm_checked_arith_result_t){ .ok = false, .why = sce_failure_.error }; }
        if (sce_cond1_) {
            r = sce_forge_checked_rem_i32(&sce_failure_, a, b);
            if (sce_failure_.failed) { return (algorithm_checked_arith_result_t){ .ok = false, .why = sce_failure_.error }; }
        }
    }
    {
        const bool sce_cond1_ = op == 5;
        if (sce_failure_.failed) { return (algorithm_checked_arith_result_t){ .ok = false, .why = sce_failure_.error }; }
        if (sce_cond1_) {
            r = sce_forge_checked_neg_i32(&sce_failure_, a);
            if (sce_failure_.failed) { return (algorithm_checked_arith_result_t){ .ok = false, .why = sce_failure_.error }; }
        }
    }
    {
        const bool sce_cond1_ = op == 6;
        if (sce_failure_.failed) { return (algorithm_checked_arith_result_t){ .ok = false, .why = sce_failure_.error }; }
        if (sce_cond1_) {
            r = sce_forge_checked_sub_u8(&sce_failure_, op, 7);
            if (sce_failure_.failed) { return (algorithm_checked_arith_result_t){ .ok = false, .why = sce_failure_.error }; }
        }
    }
    {
        const int32_t sce_value_ = r;
        if (sce_failure_.failed) { return (algorithm_checked_arith_result_t){ .ok = false, .why = sce_failure_.error }; }
        return (algorithm_checked_arith_result_t){ .value = sce_value_, .ok = true };
    }
}

#endif  /* SCE_FORGE_ALGORITHM_CHECKED_ARITH_H */
