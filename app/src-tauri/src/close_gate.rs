// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Whether a window that was asked to close may.
//!
//! A browser tab asks before it goes (`beforeunload`); a desktop window has no such
//! event, so the shell does it: the screen reports whether it holds anything the
//! core has not been given ([`CloseGate::unsaved`]), and when the window is asked to
//! close with something unsaved the shell holds the close and asks the screen to put
//! the question to the person. What the person decides is the screen's to say
//! ([`CloseGate::allow`]); the shell only keeps the window from closing under text
//! it has been told is unsaved.
//!
//! ⚠ A window that cannot be closed is its own failure. The screen that said it held
//! unsaved text may have crashed or never loaded the code that answers, and then
//! "hold the close until it answers" is a window nobody can close. So a close that is
//! asked again, after the screen was asked and said nothing for [`UNRESPONSIVE`],
//! goes through: a screen that does not answer protects nothing. This is decided here,
//! from the times, so it is a test and not a hope.

use std::sync::Mutex;
use std::time::{Duration, Instant};

/// How long a screen that was asked about a close may say nothing before it is taken
/// to be unable to.
pub const UNRESPONSIVE: Duration = Duration::from_secs(3);

/// What to do with a request to close the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    /// Let the window close.
    Close,
    /// Hold the close and ask the screen to put the question to the person.
    Ask,
}

#[derive(Debug)]
struct State {
    /// What the screen last reported.
    unsaved: bool,
    /// When the screen last said anything about it.
    heard_at: Instant,
    /// When the screen was last asked to put the question, if it has been.
    asked_at: Option<Instant>,
    /// The person said the window may close.
    allowed: bool,
}

/// The shell's memory of whether the screen holds unsaved changes.
#[derive(Debug)]
pub struct CloseGate {
    state: Mutex<State>,
}

impl CloseGate {
    /// A gate at `now`, with nothing unsaved: a window closes freely until the screen
    /// says it holds something.
    pub fn new(now: Instant) -> Self {
        CloseGate {
            state: Mutex::new(State {
                unsaved: false,
                heard_at: now,
                asked_at: None,
                allowed: false,
            }),
        }
    }

    fn state(&self) -> std::sync::MutexGuard<'_, State> {
        // A panic while holding this lock leaves a state that is still a valid one.
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// The screen says whether it holds changes the core has not been given. Saying
    /// so again while a close is being asked about is how it shows it is answering.
    pub fn unsaved(&self, unsaved: bool, now: Instant) {
        let mut state = self.state();
        state.unsaved = unsaved;
        state.heard_at = now;
    }

    /// The person decided the window may close.
    pub fn allow(&self) {
        self.state().allowed = true;
    }

    /// The window was asked to close at `now`.
    pub fn on_close_requested(&self, now: Instant) -> Decision {
        let mut state = self.state();
        if state.allowed || !state.unsaved {
            return Decision::Close;
        }
        if let Some(asked_at) = state.asked_at {
            let answered = state.heard_at > asked_at;
            if !answered && now.saturating_duration_since(asked_at) >= UNRESPONSIVE {
                return Decision::Close;
            }
        }
        state.asked_at = Some(now);
        Decision::Ask
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(start: Instant, seconds: u64) -> Instant {
        start + Duration::from_secs(seconds)
    }

    #[test]
    fn a_window_with_nothing_unsaved_closes() {
        let start = Instant::now();
        let gate = CloseGate::new(start);
        assert_eq!(gate.on_close_requested(at(start, 1)), Decision::Close);
        gate.unsaved(true, at(start, 2));
        gate.unsaved(false, at(start, 3));
        assert_eq!(gate.on_close_requested(at(start, 4)), Decision::Close);
    }

    #[test]
    fn a_window_with_something_unsaved_asks_before_it_closes() {
        let start = Instant::now();
        let gate = CloseGate::new(start);
        gate.unsaved(true, at(start, 1));
        assert_eq!(gate.on_close_requested(at(start, 2)), Decision::Ask);
    }

    #[test]
    fn once_the_person_allows_it_the_window_closes() {
        let start = Instant::now();
        let gate = CloseGate::new(start);
        gate.unsaved(true, at(start, 1));
        assert_eq!(gate.on_close_requested(at(start, 2)), Decision::Ask);
        gate.allow();
        assert_eq!(gate.on_close_requested(at(start, 3)), Decision::Close);
    }

    #[test]
    fn a_screen_that_answers_keeps_the_window_open_however_often_it_is_asked() {
        let start = Instant::now();
        let gate = CloseGate::new(start);
        gate.unsaved(true, at(start, 1));
        assert_eq!(gate.on_close_requested(at(start, 2)), Decision::Ask);
        // The screen put its question to the person and said it still holds changes.
        gate.unsaved(true, at(start, 3));
        // Asked again long after, still protected: it has spoken since it was asked.
        assert_eq!(gate.on_close_requested(at(start, 60)), Decision::Ask);
    }

    #[test]
    fn a_screen_that_does_not_answer_is_not_allowed_to_hold_the_window_for_ever() {
        let start = Instant::now();
        let gate = CloseGate::new(start);
        gate.unsaved(true, at(start, 1));
        assert_eq!(gate.on_close_requested(at(start, 2)), Decision::Ask);
        // Asked again at once: it may be still thinking.
        assert_eq!(gate.on_close_requested(at(start, 3)), Decision::Ask);
        // Asked again after it has been silent for the whole wait: it cannot answer.
        assert_eq!(gate.on_close_requested(at(start, 8)), Decision::Close);
    }

    #[test]
    fn the_silence_is_counted_from_the_last_ask_and_not_from_the_first() {
        let start = Instant::now();
        let gate = CloseGate::new(start);
        gate.unsaved(true, at(start, 1));
        assert_eq!(gate.on_close_requested(at(start, 2)), Decision::Ask);
        assert_eq!(gate.on_close_requested(at(start, 4)), Decision::Ask);
        // Four seconds after the FIRST ask but only two after the last.
        assert_eq!(gate.on_close_requested(at(start, 6)), Decision::Ask);
        assert_eq!(gate.on_close_requested(at(start, 9)), Decision::Close);
    }
}
