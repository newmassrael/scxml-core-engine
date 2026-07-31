// SCE-MAP: timer_diag_scheduler:1

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="timer") */
/* Shape: SCE Protocol-Synthesis RFC §synth-5-D line 880-886 — single timer per
 * doc with event-driven reset / state-exit cancel / fire event. */
/* Runtime: sce_forge_runtime::hal */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_TIMER_DIAG_SCHEDULER_H
#define SCE_FORGE_TIMER_DIAG_SCHEDULER_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

/* Period configured at compile time from `<sce:period>`. */
#define TIMER_DIAG_SCHEDULER_PERIOD_US UINT64_C(2000000)
#define TIMER_DIAG_SCHEDULER_PERIOD_MS UINT32_C(2000)
#define TIMER_DIAG_SCHEDULER_RESET_ON_EVENT "diag.heartbeat"
#define TIMER_DIAG_SCHEDULER_CANCEL_ON_STATE_EXIT "diag.idle"

/* Timer HAL — vtable interface. C mirror of cpp `class ITimer`. */
typedef void (*timer_diag_scheduler_timer_cb_t)(void *ctx);

typedef struct timer_diag_scheduler_itimer {
    void (*start_periodic)(struct timer_diag_scheduler_itimer *self, uint32_t interval_ms,
                           timer_diag_scheduler_timer_cb_t cb, void *cb_ctx);
    void (*cancel)(struct timer_diag_scheduler_itimer *self);
} timer_diag_scheduler_itimer_t;

/* Handler — user-supplied fire callback. The §synth-5-D single-timer shape
 * has one fire-event per doc, so the handler struct carries a single
 * function pointer (NULL-checked at trampoline time). */
typedef struct {
    void *user_data;
    void (*fire_diag_tick)(void *user_data);
} timer_diag_scheduler_handler_t;

/* Scheduler state — wraps handler + single timer pointer. */
typedef struct {
    timer_diag_scheduler_handler_t handler;
    timer_diag_scheduler_itimer_t *timer;
} timer_diag_scheduler_t;

static inline void timer_diag_scheduler_init(timer_diag_scheduler_t *self,
        timer_diag_scheduler_handler_t handler,
        timer_diag_scheduler_itimer_t *timer) {
    self->handler = handler;
    self->timer = timer;
}

/* Fire trampoline — recovers scheduler from `ctx`, dispatches through
 * the handler function pointer (NULL-checked). */
static inline void timer_diag_scheduler_on_fire_diag_tick(void *ctx) {
    timer_diag_scheduler_t *self = (timer_diag_scheduler_t *)ctx;
    if (self->handler.fire_diag_tick != NULL) {
        self->handler.fire_diag_tick(self->handler.user_data);
    }
}

/* Start the periodic timer at compile-time `PERIOD_MS`. */
static inline void timer_diag_scheduler_start(timer_diag_scheduler_t *self) {
    self->timer->start_periodic(
        self->timer, TIMER_DIAG_SCHEDULER_PERIOD_MS,
        timer_diag_scheduler_on_fire_diag_tick, self);
}

/* Cancel the timer. Idempotent per the HAL contract. */
static inline void timer_diag_scheduler_cancel(timer_diag_scheduler_t *self) {
    self->timer->cancel(self->timer);
}

/* `<sce:reset-on event="diag.heartbeat"/>` consumer hook —
 * wire into the host SCXML transition body for this event. */
static inline void timer_diag_scheduler_on_reset_diag_heartbeat(timer_diag_scheduler_t *self) {
    timer_diag_scheduler_cancel(self);
    timer_diag_scheduler_start(self);
}

/* `<sce:cancel-on state-exit="diag.idle"/>` consumer
 * hook — wire into the host SCXML `<onexit>` for this state. */
static inline void timer_diag_scheduler_on_cancel_diag_idle_exit(timer_diag_scheduler_t *self) {
    timer_diag_scheduler_cancel(self);
}

#endif  /* SCE_FORGE_TIMER_DIAG_SCHEDULER_H */
