// SCE-MAP: filter_low_pass:1 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="filter") */
/* Runtime: sce_forge_runtime */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_FILTER_LOW_PASS_H
#define SCE_FORGE_FILTER_LOW_PASS_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

/* First-order exponential low-pass: y[n] = alpha * x[n] + (1 - alpha) * y[n-1].
 * On the first sample, y[0] = x[0] (no warm-up bias toward zero).
 * Operand order matches the cpp/Rust runtimes for byte-equal cross-language
 * conformance (alpha * input) + ((1 - alpha) * state). */
typedef struct {
    double state;
    bool initialized;
} filter_low_pass_t;

static inline double filter_low_pass_update(filter_low_pass_t *self, double raw_signal) {
    if (!self->initialized) {
        self->state = (double)raw_signal;
        self->initialized = true;
    } else {
        self->state = (double)0.1 * (double)raw_signal
                    + ((double)1 - (double)0.1) * self->state;
    }
    return self->state;
}

static inline void filter_low_pass_reset(filter_low_pass_t *self) {
    self->state = (double)0;
    self->initialized = false;
}

#endif  /* SCE_FORGE_FILTER_LOW_PASS_H */
