// SCE-MAP: filter_moving_average:1 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="filter") */
/* Runtime: sce_forge_runtime */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_FILTER_MOVING_AVERAGE_H
#define SCE_FORGE_FILTER_MOVING_AVERAGE_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

/* Sliding-window arithmetic mean. Until the buffer is full, returns the mean
 * of samples seen so far; after fill, returns the mean of the most recent
 * WINDOW samples. */
typedef struct {
    double buffer[5];
    size_t index;
    bool filled;
} filter_moving_average_t;

static inline double filter_moving_average_update(filter_moving_average_t *self, double raw_temp) {
    self->buffer[self->index] = (double)raw_temp;
    self->index = (self->index + 1) % 5;
    if (!self->filled && self->index == 0) {
        self->filled = true;
    }
    size_t count = self->filled ? (size_t)5 : self->index;
    double sum = (double)0;
    for (size_t i = 0; i < count; ++i) {
        sum += self->buffer[i];
    }
    return sum / (double)count;
}

static inline void filter_moving_average_reset(filter_moving_average_t *self) {
    for (size_t i = 0; i < 5; ++i) {
        self->buffer[i] = (double)0;
    }
    self->index = 0;
    self->filled = false;
}

#endif  /* SCE_FORGE_FILTER_MOVING_AVERAGE_H */
