// SCE-MAP: observer_coolant:1 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="observer") */
/* Runtime: sce_forge_runtime */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_OBSERVER_COOLANT_H
#define SCE_FORGE_OBSERVER_COOLANT_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

/* Event tag enum — the C global namespace has no nested-class scoping,
 * so each tag is prefixed with the observer's UPPER_SNAKE name. cpp uses
 * `ForgeDomain::Tag::EMIT_WARNING`; the C11 mirror is
 * `<UPPER>_TAG_EMIT_WARNING` (no member access form available). */
typedef enum {
    OBSERVER_COOLANT_TAG_EMIT_WARNING,
    OBSERVER_COOLANT_TAG_CLEAR_WARNING,
    OBSERVER_COOLANT_TAG_EMERGENCY_SHUTDOWN
} observer_coolant_tag_t;

/* Fixed-capacity event FIFO (cpp parity: SCE::Forge::EventQueue<Domain, 8>).
 * Returned by value from update(); the harness inspects `size` and
 * iterates `buffer[0..size]`. */
#define OBSERVER_COOLANT_QUEUE_CAPACITY 8
typedef struct {
    observer_coolant_tag_t buffer[OBSERVER_COOLANT_QUEUE_CAPACITY];
    size_t size;
} observer_coolant_queue_t;

/* Observer state — one bool per monitor (ThresholdState parity). */
typedef struct {
    bool warning_active;
    bool critical_active;
} observer_coolant_t;

static inline observer_coolant_queue_t observer_coolant_update(observer_coolant_t *sce_self, double coolant_temp) {
    observer_coolant_queue_t sce_events;
    sce_events.size = 0;
    if (!sce_self->warning_active && (coolant_temp > 110.0)) {
        sce_self->warning_active = true;
        if (sce_events.size < OBSERVER_COOLANT_QUEUE_CAPACITY) {
            sce_events.buffer[sce_events.size++] = OBSERVER_COOLANT_TAG_EMIT_WARNING;
        }
    }
    else if (sce_self->warning_active && (coolant_temp < 100.0)) {
        sce_self->warning_active = false;
        if (sce_events.size < OBSERVER_COOLANT_QUEUE_CAPACITY) {
            sce_events.buffer[sce_events.size++] = OBSERVER_COOLANT_TAG_CLEAR_WARNING;
        }
    }
    if (!sce_self->critical_active && (coolant_temp > 120.0)) {
        sce_self->critical_active = true;
        if (sce_events.size < OBSERVER_COOLANT_QUEUE_CAPACITY) {
            sce_events.buffer[sce_events.size++] = OBSERVER_COOLANT_TAG_EMERGENCY_SHUTDOWN;
        }
    }
    else if (sce_self->critical_active && (coolant_temp < 105.0)) {
        sce_self->critical_active = false;
    }
    return sce_events;
}

#endif  /* SCE_FORGE_OBSERVER_COOLANT_H */
