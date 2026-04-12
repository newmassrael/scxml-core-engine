// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! Three signal-filter struct templates matching SCE_FORGE.md Section 4.8:
//! [`MovingAverage`], [`LowPass`], [`Debounce`].
//!
//! All filters keep state in a fixed-size internal buffer (no heap).

use core::ops::{Add, AddAssign, Div, Mul, Sub};

mod sealed {
    pub trait Sealed {}
    impl Sealed for f32 {}
    impl Sealed for f64 {}
}

/// Floating-point types accepted by [`MovingAverage`] and [`LowPass`].
/// Sealed: only `f32` and `f64` may implement it. Restricting filtering to
/// floating-point types avoids the truncation pitfalls of integer averaging.
pub trait Float:
    sealed::Sealed
    + Copy
    + Default
    + Add<Output = Self>
    + AddAssign
    + Sub<Output = Self>
    + Mul<Output = Self>
    + Div<Output = Self>
{
    fn from_count(c: usize) -> Self;
    fn one() -> Self;
}

impl Float for f32 {
    fn from_count(c: usize) -> Self {
        c as Self
    }
    fn one() -> Self {
        1.0
    }
}

impl Float for f64 {
    fn from_count(c: usize) -> Self {
        c as Self
    }
    fn one() -> Self {
        1.0
    }
}

/// Sliding-window arithmetic mean. Until the buffer is full, returns the mean
/// of samples seen so far; after fill, returns the mean of the most recent
/// `WINDOW` samples.
pub struct MovingAverage<T: Float, const WINDOW: usize> {
    buffer: [T; WINDOW],
    index: usize,
    filled: bool,
}

impl<T: Float, const WINDOW: usize> MovingAverage<T, WINDOW> {
    pub fn new() -> Self {
        Self {
            buffer: [T::default(); WINDOW],
            index: 0,
            filled: false,
        }
    }

    pub fn update(&mut self, input: T) -> T {
        self.buffer[self.index] = input;
        self.index = (self.index + 1) % WINDOW;
        if !self.filled && self.index == 0 {
            self.filled = true;
        }
        let count = if self.filled { WINDOW } else { self.index };
        let mut sum = T::default();
        let mut i = 0;
        while i < count {
            sum += self.buffer[i];
            i += 1;
        }
        sum / T::from_count(count)
    }

    pub fn reset(&mut self) {
        self.buffer = [T::default(); WINDOW];
        self.index = 0;
        self.filled = false;
    }
}

impl<T: Float, const WINDOW: usize> Default for MovingAverage<T, WINDOW> {
    fn default() -> Self {
        Self::new()
    }
}

/// First-order exponential low-pass: `y[n] = alpha * x[n] + (1 - alpha) * y[n-1]`.
/// On the first sample, `y[0] = x[0]` (no warm-up bias toward zero).
pub struct LowPass<T: Float> {
    alpha: T,
    state: T,
    initialized: bool,
}

impl<T: Float> LowPass<T> {
    pub fn new(alpha: T) -> Self {
        Self {
            alpha,
            state: T::default(),
            initialized: false,
        }
    }

    pub fn update(&mut self, input: T) -> T {
        if !self.initialized {
            self.state = input;
            self.initialized = true;
        } else {
            self.state = self.alpha * input + (T::one() - self.alpha) * self.state;
        }
        self.state
    }

    pub fn reset(&mut self) {
        self.state = T::default();
        self.initialized = false;
    }
}

/// Output latches to a new value only after `WINDOW` consecutive identical
/// samples. Until the buffer fills, the most recent input passes through.
pub struct Debounce<T: Copy + Default + PartialEq, const WINDOW: usize> {
    buffer: [T; WINDOW],
    index: usize,
    filled: bool,
    output: T,
}

impl<T: Copy + Default + PartialEq, const WINDOW: usize> Debounce<T, WINDOW> {
    pub fn new() -> Self {
        Self {
            buffer: [T::default(); WINDOW],
            index: 0,
            filled: false,
            output: T::default(),
        }
    }

    pub fn update(&mut self, input: T) -> T {
        self.buffer[self.index] = input;
        self.index = (self.index + 1) % WINDOW;
        if !self.filled && self.index == 0 {
            self.filled = true;
        }

        if self.filled {
            let mut stable = true;
            let mut i = 1;
            while i < WINDOW {
                if self.buffer[i] != self.buffer[0] {
                    stable = false;
                    break;
                }
                i += 1;
            }
            if stable {
                self.output = self.buffer[0];
            }
        } else {
            self.output = input;
        }
        self.output
    }

    pub fn reset(&mut self) {
        self.buffer = [T::default(); WINDOW];
        self.index = 0;
        self.filled = false;
        self.output = T::default();
    }
}

impl<T: Copy + Default + PartialEq, const WINDOW: usize> Default for Debounce<T, WINDOW> {
    fn default() -> Self {
        Self::new()
    }
}
