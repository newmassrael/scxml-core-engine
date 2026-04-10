// SCE Forge: Auto-generated from Extended SCXML (sce:kind="observer")
// Do not edit — regenerate from the source SCXML file.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    EMIT_WARNING,
    CLEAR_WARNING,
    EMERGENCY_SHUTDOWN,
}

pub struct ObserverCoolant {
    warning_active: bool,
    critical_active: bool,
}

impl ObserverCoolant {
    pub fn new() -> Self {
        Self {
            warning_active: false,
            critical_active: false,
        }
    }

    pub fn update(&mut self, coolant_temp: f64) -> Vec<Event> {
        let mut events = Vec::new();
        if !self.warning_active && (coolantTemp > 110.0) {
            self.warning_active = true;
            events.push(Event::EMIT_WARNING);
        } else if self.warning_active && (coolantTemp < 100.0) {
            self.warning_active = false;
            events.push(Event::CLEAR_WARNING);
        }
        if !self.critical_active && (coolantTemp > 120.0) {
            self.critical_active = true;
            events.push(Event::EMERGENCY_SHUTDOWN);
        } else if self.critical_active && (coolantTemp < 105.0) {
            self.critical_active = false;
        }
        events
    }
}