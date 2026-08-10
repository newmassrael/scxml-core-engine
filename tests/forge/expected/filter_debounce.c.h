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

static inline bool filter_debounce_update(filter_debounce_t *self, bool raw_button) {
    self->buffer[self->index] = (bool)raw_button;
    self->index = (self->index + 1) % 3;
    if (!self->filled && self->index == 0) {
        self->filled = true;
    }
    if (self->filled) {
        bool stable = true;
        for (size_t i = 1; i < 3; ++i) {
            if (self->buffer[i] != self->buffer[0]) {
                stable = false;
                break;
            }
        }
        if (stable) {
            self->output = self->buffer[0];
        }
    } else {
        self->output = (bool)raw_button;
    }
    return self->output;
}

static inline void filter_debounce_reset(filter_debounce_t *self) {
    for (size_t i = 0; i < 3; ++i) {
        self->buffer[i] = (bool)0;
    }
    self->index = 0;
    self->filled = false;
    self->output = (bool)0;
}

#endif  /* SCE_FORGE_FILTER_DEBOUNCE_H */
