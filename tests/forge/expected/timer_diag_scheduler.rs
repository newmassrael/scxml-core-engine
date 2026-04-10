// SCE Forge: Auto-generated from Extended SCXML (sce:kind="timer")
// Do not edit — regenerate from the source SCXML file.

/// Platform timer trait (injected at runtime).
pub trait Timer {
    fn start_periodic(&mut self, interval_ms: u32, callback: Box<dyn Fn()>);
    fn start_one_shot(&mut self, delay_ms: u32, callback: Box<dyn Fn()>);
    fn cancel(&mut self);
}

pub struct TimerDiagScheduler {
    pub testerPresent_timer: Option<Box<dyn Timer>>,
    pub responseTimeout_timer: Option<Box<dyn Timer>>,
    pub retryDelay_timer: Option<Box<dyn Timer>>,
}

impl TimerDiagScheduler {
    pub fn new() -> Self {
        Self {
            testerPresent_timer: None,
            responseTimeout_timer: None,
            retryDelay_timer: None,
        }
    }

    pub fn start_testerPresent(&mut self) {
        if let Some(ref mut timer) = self.testerPresent_timer {
            timer.start_periodic(2000, Box::new(|| { /* TesterPresent */ }));
        }
    }

    pub fn cancel_testerPresent(&mut self) {
        if let Some(ref mut timer) = self.testerPresent_timer {
            timer.cancel();
        }
    }

    pub fn start_responseTimeout(&mut self) {
        if let Some(ref mut timer) = self.responseTimeout_timer {
            timer.start_one_shot(5000, Box::new(|| { /* handleTimeout */ }));
        }
    }

    pub fn cancel_responseTimeout(&mut self) {
        if let Some(ref mut timer) = self.responseTimeout_timer {
            timer.cancel();
        }
    }

    pub fn start_retryDelay(&mut self) {
        if let Some(ref mut timer) = self.retryDelay_timer {
            timer.start_one_shot(10000, Box::new(|| { /* retrySecurityAccess */ }));
        }
    }

    pub fn cancel_retryDelay(&mut self) {
        if let Some(ref mut timer) = self.retryDelay_timer {
            timer.cancel();
        }
    }
}