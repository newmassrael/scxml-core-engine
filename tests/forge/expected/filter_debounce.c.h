// SCE-MAP: filter_debounce:1 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="filter") */
/* Runtime: sce_forge_runtime */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_FILTER_DEBOUNCE_H
#define SCE_FORGE_FILTER_DEBOUNCE_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

/* Output latches to a new value only after WINDOW consecutive identical
 * samples. Until the buffer fills, the most recent input passes through. */
typedef struct {
    bool buffer[3];
    size_t index;
    bool filled;
    bool output;
} filter_debounce_t;

static inline bool filter_debounce_update(filter_debounce_t *sce_self, bool raw_button) {
    sce_self->buffer[sce_self->index] = (bool)raw_button;
    sce_self->index = (sce_self->index + 1) % 3;
    if (!sce_self->filled && sce_self->index == 0) {
        sce_self->filled = true;
    }
    if (sce_self->filled) {
        bool sce_stable = true;
        for (size_t sce_i = 1; sce_i < 3; ++sce_i) {
            if (sce_self->buffer[sce_i] != sce_self->buffer[0]) {
                sce_stable = false;
                break;
            }
        }
        if (sce_stable) {
            sce_self->output = sce_self->buffer[0];
        }
    } else {
        sce_self->output = (bool)raw_button;
    }
    return sce_self->output;
}

static inline void filter_debounce_reset(filter_debounce_t *sce_self) {
    for (size_t sce_i = 0; sce_i < 3; ++sce_i) {
        sce_self->buffer[sce_i] = (bool)0;
    }
    sce_self->index = 0;
    sce_self->filled = false;
    sce_self->output = (bool)0;
}

#endif  /* SCE_FORGE_FILTER_DEBOUNCE_H */
