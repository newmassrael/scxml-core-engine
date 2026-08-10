#![doc = "SCE-MAP: timer_diag_scheduler:1 :: _forge_body"]
// SCE-MAP: timer_diag_scheduler:1 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="timer")
// Shape: SCE Protocol-Synthesis RFC §synth-5-D line 880-886 — single timer per
// doc with event-driven reset / state-exit cancel / fire event.
// Runtime: sce_forge_runtime::hal
// Do not edit — regenerate from the source SCXML file.

use core::ffi::c_void;
use core::marker::PhantomData;
use sce_forge_runtime::timer::Timer;

/// Handler trait for [`TimerDiagScheduler`]. The user implements
/// `fire_diag_tick` on a state struct and passes a mutable
/// reference to [`TimerDiagScheduler::new`]. Called when the timer's
/// `<sce:period>` elapses, materializing the `<sce:fire-event>`
/// `diag.tick` event.
pub trait TimerDiagSchedulerHandler {
    fn fire_diag_tick(&mut self);
}

/// Period configured at compile time from the source SCXML
/// `<sce:period>`. Microseconds (u64) cover MCU microsecond ticks
/// and AP minute-scale watchdogs uniformly.
pub const PERIOD_US: u64 = 2000000;

/// Period in milliseconds (derived from `PERIOD_US`). Most runtime
/// `Timer` impls accept milliseconds; the constant is exposed so
/// the consumer can swap units without parsing the source again.
pub const PERIOD_MS: u32 = 2000u32;

/// `<sce:reset-on event="diag.heartbeat"/>` — when raised in
/// the host SCXML, the consumer calls
/// `TimerDiagScheduler::on_reset_diag_heartbeat()` to
/// restart the timer's deadline.
pub const RESET_ON_EVENT: &'static str = "diag.heartbeat";

/// `<sce:cancel-on state-exit="diag.idle"/>` — when
/// the host SCXML exits state `diag.idle`, the
/// consumer calls
/// `TimerDiagScheduler::on_cancel_diag_idle_exit()` to
/// cancel the timer.
pub const CANCEL_ON_STATE_EXIT: &'static str = "diag.idle";

/// Generated single-timer scheduler per SCE Protocol-Synthesis RFC §synth-5-D.
/// Generic over a concrete platform [`Timer`] type and a user-supplied
/// [`TimerDiagSchedulerHandler`]. The fire trampoline erases the
/// lifetime via raw pointer when crossing FFI; the scheduler must be
/// dropped (or `cancel()` called) before either borrow is invalidated.
pub struct TimerDiagScheduler<'a, H: TimerDiagSchedulerHandler, T: Timer> {
    handler: *mut H,
    timer: *mut T,
    _marker: PhantomData<&'a mut (H, T)>,
}

impl<'a, H: TimerDiagSchedulerHandler, T: Timer> TimerDiagScheduler<'a, H, T> {
    pub fn new(handler: &'a mut H, timer: &'a mut T) -> Self {
        Self {
            handler: handler as *mut H,
            timer: timer as *mut T,
            _marker: PhantomData,
        }
    }

    /// Start the periodic timer with the compile-time `PERIOD_MS`.
    /// SAFETY: handler and timer references are valid for `'a`,
    /// which outlives this method invocation by construction.
    pub fn start(&mut self) {
        unsafe {
            (*self.timer).start_periodic(
                PERIOD_MS,
                Self::on_fire_trampoline,
                self.handler as *mut c_void,
            );
        }
    }

    /// Cancel the timer. Safe to call whether or not the timer is
    /// active — the runtime `cancel()` is idempotent per the
    /// `sce_forge_runtime::timer::Timer` trait contract.
    pub fn cancel(&mut self) {
        // SAFETY: see `start()`.
        unsafe { (*self.timer).cancel(); }
    }

    /// `<sce:reset-on event="diag.heartbeat"/>` consumer hook.
    /// Restart the deadline by cancelling and re-arming with the
    /// compile-time `PERIOD_MS`. The consumer wires this into the
    /// host SCXML's `<transition event="diag.heartbeat">` body.
    pub fn on_reset_diag_heartbeat(&mut self) {
        self.cancel();
        self.start();
    }

    /// `<sce:cancel-on state-exit="diag.idle"/>`
    /// consumer hook. The consumer wires this into the host SCXML's
    /// `<onexit>` for state `diag.idle`.
    pub fn on_cancel_diag_idle_exit(&mut self) {
        self.cancel();
    }

    extern "C" fn on_fire_trampoline(ctx: *mut c_void) {
        // SAFETY: ctx came from `self.handler as *mut H` and is
        // valid for the lifetime of the enclosing scheduler. The
        // reference is dropped before this function returns.
        let h = unsafe { &mut *(ctx as *mut H) };
        h.fire_diag_tick();
    }
}
