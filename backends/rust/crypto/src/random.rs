// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Where random bytes come from (RFC 4086; ADR 0109, D5).
//!
//! A machine is a function of its inputs, and a random value is one of them, as the time is: a host
//! draws it from a [`RandomSource`], the operating system's ([`SystemRandom`]) unless the embedding
//! gives another. A test gives one whose bytes it knows ([`ScriptedRandom`]), as it gives a scripted
//! clock, so that a scenario that draws bytes, generates a key or seals is judged by its exact
//! bytes.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex, PoisonError};

/// A source of random bytes.
pub trait RandomSource {
    /// Fill `out` with random bytes: `false`, with `out` unspecified, when the source cannot.
    fn fill(&mut self, out: &mut [u8]) -> bool;
}

/// The operating system's cryptographic generator (`getrandom(2)`, `BCryptGenRandom`, ...).
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemRandom;

impl RandomSource for SystemRandom {
    fn fill(&mut self, out: &mut [u8]) -> bool {
        getrandom::getrandom(out).is_ok()
    }
}

/// Bytes a test names, given before the operating system's: a draw takes them in order, and once
/// they are spent it draws the rest from [`SystemRandom`]. A test that fixes a draw queues every
/// byte the draw takes.
///
/// A source that gives known bytes gives known keys and nonces: it is for a test that holds a host
/// to its bytes, and for nothing else.
#[derive(Debug, Clone, Default)]
pub struct ScriptedRandom {
    queued: VecDeque<u8>,
}

impl ScriptedRandom {
    pub fn new() -> Self {
        Self::default()
    }

    /// Give `bytes` next, in place of what was queued before and not drawn.
    pub fn set(&mut self, bytes: &[u8]) {
        self.queued = bytes.iter().copied().collect();
    }

    /// How many queued bytes are not drawn yet.
    pub fn queued(&self) -> usize {
        self.queued.len()
    }
}

impl RandomSource for ScriptedRandom {
    fn fill(&mut self, out: &mut [u8]) -> bool {
        let taken = out.len().min(self.queued.len());
        for (slot, byte) in out.iter_mut().zip(self.queued.drain(..taken)) {
            *slot = byte;
        }
        SystemRandom.fill(&mut out[taken..])
    }
}

/// A source a host owns behind a box, as it holds the one its embedding gives.
impl<R: RandomSource + ?Sized> RandomSource for Box<R> {
    fn fill(&mut self, out: &mut [u8]) -> bool {
        (**self).fill(out)
    }
}

/// A source shared with whoever scripts it, as a test shares a scripted clock with the host it
/// gives it to.
impl<R: RandomSource + ?Sized> RandomSource for Arc<Mutex<R>> {
    fn fill(&mut self, out: &mut [u8]) -> bool {
        self.lock()
            .unwrap_or_else(PoisonError::into_inner)
            .fill(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_scripted_source_gives_its_bytes_in_order_and_then_the_systems() {
        let mut source = ScriptedRandom::new();
        source.set(&[1, 2, 3, 4, 5]);
        let mut first = [0u8; 2];
        assert!(source.fill(&mut first));
        assert_eq!(first, [1, 2]);
        let mut second = [0u8; 3];
        assert!(source.fill(&mut second));
        assert_eq!(second, [3, 4, 5]);
        assert_eq!(source.queued(), 0);
        source.set(&[9]);
        let mut longer = [0u8; 33];
        assert!(source.fill(&mut longer));
        assert_eq!(longer[0], 9, "the queued byte first");
        assert_eq!(source.queued(), 0);
    }

    #[test]
    fn two_draws_from_the_system_differ() {
        let mut one = [0u8; 32];
        let mut two = [0u8; 32];
        assert!(SystemRandom.fill(&mut one));
        assert!(SystemRandom.fill(&mut two));
        assert_ne!(one, two, "two draws of 256 bits from the operating system");
    }
}
