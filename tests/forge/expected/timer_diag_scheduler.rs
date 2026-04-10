// SCE Forge: Auto-generated from Extended SCXML (sce:kind="timer")
// Do not edit — regenerate from the source SCXML file.

use core::ffi::c_void;
use core::marker::PhantomData;
use sce_forge_runtime::timer::Timer;

/// Handler trait for [`TimerDiagScheduler`]. The user implements these methods
/// on a state struct and passes a mutable reference to [`TimerDiagScheduler::new`].
/// The compiler enforces that every method exists; there is no silent fallback.
pub trait TimerDiagSchedulerHandler {
    fn fire_tester_present(&mut self);
    fn fire_handle_timeout(&mut self);
    fn fire_retry_security_access(&mut self);
}

/// Generated scheduler. Generic over a concrete platform [`Timer`] type and a
/// user-supplied [`TimerDiagSchedulerHandler`]. References to both are taken
/// for the lifetime `'a`; the borrow checker keeps them alive for as long as
/// this scheduler exists. Trampolines erase the lifetime via raw pointer when
/// crossing the FFI boundary, but the contract is that the scheduler must be
/// dropped (or all timers cancelled) before any borrow is invalidated.
pub struct TimerDiagScheduler<'a, H: TimerDiagSchedulerHandler, T: Timer> {
    handler: *mut H,
    tester_present_timer: *mut T,
    response_timeout_timer: *mut T,
    retry_delay_timer: *mut T,
    _marker: PhantomData<&'a mut (H, T)>,
}

impl<'a, H: TimerDiagSchedulerHandler, T: Timer> TimerDiagScheduler<'a, H, T> {
    pub fn new(handler: &'a mut H, tester_present_timer: &'a mut T, response_timeout_timer: &'a mut T, retry_delay_timer: &'a mut T) -> Self {
        Self {
            handler: handler as *mut H,
            tester_present_timer: tester_present_timer as *mut T,
            response_timeout_timer: response_timeout_timer as *mut T,
            retry_delay_timer: retry_delay_timer as *mut T,
            _marker: PhantomData,
        }
    }

    pub fn start_tester_present(&mut self) {
        // SAFETY: handler and timer references are valid for `'a`, which
        // outlives this method invocation by construction.
        unsafe {
            (*self.tester_present_timer).start_periodic(2000, Self::on_tester_present_trampoline, self.handler as *mut c_void);
        }
    }

    pub fn cancel_tester_present(&mut self) {
        // SAFETY: see start_tester_present
        unsafe { (*self.tester_present_timer).cancel(); }
    }

    pub fn start_response_timeout(&mut self) {
        // SAFETY: handler and timer references are valid for `'a`, which
        // outlives this method invocation by construction.
        unsafe {
            (*self.response_timeout_timer).start_one_shot(5000, Self::on_handle_timeout_trampoline, self.handler as *mut c_void);
        }
    }

    pub fn cancel_response_timeout(&mut self) {
        // SAFETY: see start_response_timeout
        unsafe { (*self.response_timeout_timer).cancel(); }
    }

    pub fn start_retry_delay(&mut self) {
        // SAFETY: handler and timer references are valid for `'a`, which
        // outlives this method invocation by construction.
        unsafe {
            (*self.retry_delay_timer).start_one_shot(10000, Self::on_retry_security_access_trampoline, self.handler as *mut c_void);
        }
    }

    pub fn cancel_retry_delay(&mut self) {
        // SAFETY: see start_retry_delay
        unsafe { (*self.retry_delay_timer).cancel(); }
    }

    extern "C" fn on_tester_present_trampoline(ctx: *mut c_void) {
        // SAFETY: ctx came from `self.handler as *mut H` and is valid for the
        // entire lifetime of the enclosing scheduler. The reference is dropped
        // before this function returns, so it cannot outlive the borrow.
        let h = unsafe { &mut *(ctx as *mut H) };
        h.fire_tester_present();
    }

    extern "C" fn on_handle_timeout_trampoline(ctx: *mut c_void) {
        // SAFETY: ctx came from `self.handler as *mut H` and is valid for the
        // entire lifetime of the enclosing scheduler. The reference is dropped
        // before this function returns, so it cannot outlive the borrow.
        let h = unsafe { &mut *(ctx as *mut H) };
        h.fire_handle_timeout();
    }

    extern "C" fn on_retry_security_access_trampoline(ctx: *mut c_void) {
        // SAFETY: ctx came from `self.handler as *mut H` and is valid for the
        // entire lifetime of the enclosing scheduler. The reference is dropped
        // before this function returns, so it cannot outlive the borrow.
        let h = unsafe { &mut *(ctx as *mut H) };
        h.fire_retry_security_access();
    }
}