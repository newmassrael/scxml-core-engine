/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="timer") */
/* Runtime: sce_forge_runtime::hal */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_TIMER_DIAG_SCHEDULER_H
#define SCE_FORGE_TIMER_DIAG_SCHEDULER_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

/* Timer HAL — vtable interface. cpp uses `class ITimer { virtual ... };`;
 * the C11 mirror is a struct of method pointers. Concrete timer impls
 * embed this struct as their first member and downcast inside method
 * bodies (offsetof = 0 by struct-layout convention). */
typedef void (*timer_diag_scheduler_timer_cb_t)(void *ctx);

typedef struct timer_diag_scheduler_itimer {
    void (*start_periodic)(struct timer_diag_scheduler_itimer *self, uint32_t interval_ms,
                           timer_diag_scheduler_timer_cb_t cb, void *cb_ctx);
    void (*start_one_shot)(struct timer_diag_scheduler_itimer *self, uint32_t delay_ms,
                           timer_diag_scheduler_timer_cb_t cb, void *cb_ctx);
    void (*cancel)(struct timer_diag_scheduler_itimer *self);
} timer_diag_scheduler_itimer_t;

/* Handler — user-supplied per-callback function pointers. cpp uses duck
 * typing on a Handler template parameter (compile-time check) and
 * imposes a `fire<Pascal>` method naming convention. C11 mirrors that
 * convention with `fire_<snake>` field names so the manifest oracle
 * keys (`"callback": "fireTesterPresent"`) map 1:1 to handler slots
 * after snake_case normalisation. NULL pointers are dropped silently
 * at trampoline time (cpp's duck-typed missing method would not
 * compile, but C11 has no equivalent compile-time check). */
typedef struct {
    void *user_data;
    void (*fire_tester_present)(void *user_data);
    void (*fire_handle_timeout)(void *user_data);
    void (*fire_retry_security_access)(void *user_data);
} timer_diag_scheduler_handler_t;

/* Scheduler state — wraps handler + N timer pointers (one per <data> with
 * sce:timer). cpp keeps `Handler&` and `ITimer&` references; C11 stores
 * the handler by value (function-pointer struct, cheap to copy) and the
 * itimer instances by pointer (ownership stays with the user). */
typedef struct {
    timer_diag_scheduler_handler_t handler;
    timer_diag_scheduler_itimer_t *tester_present_timer;
    timer_diag_scheduler_itimer_t *response_timeout_timer;
    timer_diag_scheduler_itimer_t *retry_delay_timer;
} timer_diag_scheduler_t;

static inline void timer_diag_scheduler_init(timer_diag_scheduler_t *self,
        timer_diag_scheduler_handler_t handler,
        timer_diag_scheduler_itimer_t *tester_present_timer,
        timer_diag_scheduler_itimer_t *response_timeout_timer,
        timer_diag_scheduler_itimer_t *retry_delay_timer) {
    self->handler = handler;
    self->tester_present_timer = tester_present_timer;
    self->response_timeout_timer = response_timeout_timer;
    self->retry_delay_timer = retry_delay_timer;
}

/* Per-callback trampolines. cpp uses `static_cast<Self*>(ctx)->handler_.<cb>()`;
 * the C11 mirror recovers the scheduler from `ctx`, then dispatches
 * through the user's handler function pointer (NULL-checked). The
 * trampoline name is `<snake>_on_fire_<callback_snake>` to mirror the
 * handler field's `fire_<snake>` form. */
static inline void timer_diag_scheduler_on_fire_tester_present(void *ctx) {
    timer_diag_scheduler_t *self = (timer_diag_scheduler_t *)ctx;
    if (self->handler.fire_tester_present != NULL) {
        self->handler.fire_tester_present(self->handler.user_data);
    }
}
static inline void timer_diag_scheduler_on_fire_handle_timeout(void *ctx) {
    timer_diag_scheduler_t *self = (timer_diag_scheduler_t *)ctx;
    if (self->handler.fire_handle_timeout != NULL) {
        self->handler.fire_handle_timeout(self->handler.user_data);
    }
}
static inline void timer_diag_scheduler_on_fire_retry_security_access(void *ctx) {
    timer_diag_scheduler_t *self = (timer_diag_scheduler_t *)ctx;
    if (self->handler.fire_retry_security_access != NULL) {
        self->handler.fire_retry_security_access(self->handler.user_data);
    }
}

/* Per-timer start/cancel pairs. cpp emits `start<TimerPascal>()` /
 * `cancel<TimerPascal>()`; C11 uses `<snake>_start_<id_snake>` /
 * `<snake>_cancel_<id_snake>` to fit C's lower_snake_case convention
 * without name-mangling. */

static inline void timer_diag_scheduler_start_tester_present(timer_diag_scheduler_t *self) {
    self->tester_present_timer->start_periodic(
        self->tester_present_timer, 2000,
        timer_diag_scheduler_on_fire_tester_present, self);
}

static inline void timer_diag_scheduler_cancel_tester_present(timer_diag_scheduler_t *self) {
    self->tester_present_timer->cancel(self->tester_present_timer);
}

static inline void timer_diag_scheduler_start_response_timeout(timer_diag_scheduler_t *self) {
    self->response_timeout_timer->start_one_shot(
        self->response_timeout_timer, 5000,
        timer_diag_scheduler_on_fire_handle_timeout, self);
}

static inline void timer_diag_scheduler_cancel_response_timeout(timer_diag_scheduler_t *self) {
    self->response_timeout_timer->cancel(self->response_timeout_timer);
}

static inline void timer_diag_scheduler_start_retry_delay(timer_diag_scheduler_t *self) {
    self->retry_delay_timer->start_one_shot(
        self->retry_delay_timer, 10000,
        timer_diag_scheduler_on_fire_retry_security_access, self);
}

static inline void timer_diag_scheduler_cancel_retry_delay(timer_diag_scheduler_t *self) {
    self->retry_delay_timer->cancel(self->retry_delay_timer);
}

#endif  /* SCE_FORGE_TIMER_DIAG_SCHEDULER_H */
