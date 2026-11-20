// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! An adapter of the metrics port that keeps every call in the order it was made: what a test of
//! a component reads to assert that an event moved the series that names it, and nothing else.

use std::sync::{Mutex, MutexGuard, PoisonError};

use crate::{Counter, Family, Gauge, Histogram, Metrics};

/// One call a component made.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Call {
    Count(Counter, u64),
    Adjust(Gauge, i64),
    Observe(Histogram, f64),
}

/// The recording adapter. See the module documentation.
#[derive(Debug, Default)]
pub struct Recording {
    calls: Mutex<Vec<Call>>,
}

impl Recording {
    /// An adapter that recorded nothing yet.
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> MutexGuard<'_, Vec<Call>> {
        self.calls.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Every call so far, in order.
    pub fn calls(&self) -> Vec<Call> {
        self.lock().clone()
    }

    /// How far `counter` was moved.
    pub fn counted(&self, counter: Counter) -> u64 {
        self.lock()
            .iter()
            .map(|call| match call {
                Call::Count(moved, by) if *moved == counter => *by,
                _ => 0,
            })
            .sum()
    }

    /// How far every counter series of `family` was moved together.
    pub fn counted_in(&self, family: &Family) -> u64 {
        self.lock()
            .iter()
            .map(|call| match call {
                Call::Count(moved, by) if moved.family().name == family.name => *by,
                _ => 0,
            })
            .sum()
    }

    /// The level `gauge` was moved to from zero.
    pub fn level(&self, gauge: Gauge) -> i64 {
        self.lock()
            .iter()
            .map(|call| match call {
                Call::Adjust(moved, delta) if *moved == gauge => *delta,
                _ => 0,
            })
            .sum()
    }

    /// The highest level `gauge` reached from zero.
    pub fn peak(&self, gauge: Gauge) -> i64 {
        let mut level = 0i64;
        let mut peak = 0i64;
        for call in self.lock().iter() {
            if let Call::Adjust(moved, delta) = call {
                if *moved == gauge {
                    level += delta;
                    peak = peak.max(level);
                }
            }
        }
        peak
    }

    /// Every observation of `histogram`, in order.
    pub fn observed(&self, histogram: Histogram) -> Vec<f64> {
        self.lock()
            .iter()
            .filter_map(|call| match call {
                Call::Observe(seen, value) if *seen == histogram => Some(*value),
                _ => None,
            })
            .collect()
    }

    /// Forget every call so far.
    pub fn clear(&self) {
        self.lock().clear();
    }
}

impl Metrics for Recording {
    fn count(&self, counter: Counter, by: u64) {
        self.lock().push(Call::Count(counter, by));
    }

    fn adjust(&self, gauge: Gauge, delta: i64) {
        self.lock().push(Call::Adjust(gauge, delta));
    }

    fn observe(&self, histogram: Histogram, value: f64) {
        self.lock().push(Call::Observe(histogram, value));
    }
}
