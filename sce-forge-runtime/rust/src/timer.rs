//! Timer HAL — pure trait that the user implements once per platform.
//! See SCE_FORGE.md Section 4.10.
//!
//! The callback signature uses `extern "C" fn(*mut c_void)` rather than
//! `Box<dyn Fn>` to avoid heap allocation and to map 1:1 to native RTOS timer
//! APIs (POSIX `timer_create`, FreeRTOS `xTimerCreate`, Zephyr `k_timer_init`).
//! Generated code holds raw pointers to the platform timer objects so that
//! the generated struct does not carry a lifetime parameter.

use core::ffi::c_void;

/// C-style callback signature.
pub type TimerCallback = extern "C" fn(ctx: *mut c_void);

/// Platform timer interface.
///
/// Implementations must be re-entrant safe: a previously scheduled callback
/// must not run after `cancel()` returns. Timer object lifetime is owned by
/// the user (typically static storage); generated code stores raw pointers and
/// never destroys them.
pub trait Timer {
    fn start_periodic(&mut self, interval_ms: u32, callback: TimerCallback, ctx: *mut c_void);
    fn start_one_shot(&mut self, delay_ms: u32, callback: TimerCallback, ctx: *mut c_void);
    fn cancel(&mut self);
}
