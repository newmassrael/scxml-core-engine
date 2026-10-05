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

static inline double filter_moving_average_update(filter_moving_average_t *sce_self, double raw_temp) {
    sce_self->buffer[sce_self->index] = (double)raw_temp;
    sce_self->index = (sce_self->index + 1) % 5;
    if (!sce_self->filled && sce_self->index == 0) {
        sce_self->filled = true;
    }
    size_t sce_count = sce_self->filled ? (size_t)5 : sce_self->index;
    double sce_sum = (double)0;
    for (size_t sce_i = 0; sce_i < sce_count; ++sce_i) {
        sce_sum += sce_self->buffer[sce_i];
    }
    return sce_sum / (double)sce_count;
}

static inline void filter_moving_average_reset(filter_moving_average_t *sce_self) {
    for (size_t sce_i = 0; sce_i < 5; ++sce_i) {
        sce_self->buffer[sce_i] = (double)0;
    }
    sce_self->index = 0;
    sce_self->filled = false;
}

#endif  /* SCE_FORGE_FILTER_MOVING_AVERAGE_H */
