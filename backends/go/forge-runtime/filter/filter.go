// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

// Package filter provides three signal-filter struct templates matching
// SCE_FORGE.md Section 4.8: MovingAverage, LowPass, Debounce.
//
// MovingAverage and LowPass are parameterized by a Float type constraint
// (~float32 | ~float64) so the same generic implementation serves both
// precisions without boxing. Debounce is parameterized by `comparable`,
// which covers Boolean, integer, string, and any user struct with value
// equality.
package filter

// Float is the type constraint for floating-point filters.
type Float interface {
	~float32 | ~float64
}

// MovingAverage is a sliding-window arithmetic mean.
type MovingAverage[T Float] struct {
	buffer []T
	index  int
	filled bool
	window int
}

// NewMovingAverage constructs a MovingAverage with the given window size.
// Panics if window < 1.
func NewMovingAverage[T Float](window int) *MovingAverage[T] {
	if window < 1 {
		panic("filter: MovingAverage window must be >= 1")
	}
	return &MovingAverage[T]{
		buffer: make([]T, window),
		window: window,
	}
}

func (m *MovingAverage[T]) Update(value T) T {
	m.buffer[m.index] = value
	m.index = (m.index + 1) % m.window
	if !m.filled && m.index == 0 {
		m.filled = true
	}
	count := m.window
	if !m.filled {
		count = m.index
	}
	var sum T
	for i := 0; i < count; i++ {
		sum += m.buffer[i]
	}
	return sum / T(count)
}

func (m *MovingAverage[T]) Reset() {
	for i := range m.buffer {
		m.buffer[i] = 0
	}
	m.index = 0
	m.filled = false
}

// LowPass is a first-order exponential low-pass filter.
// y[n] = alpha*x[n] + (1 - alpha)*y[n-1]. On the first sample, y[0] = x[0]
// (no warm-up bias toward zero).
type LowPass[T Float] struct {
	alpha       T
	state       T
	initialized bool
}

func NewLowPass[T Float](alpha T) *LowPass[T] {
	return &LowPass[T]{alpha: alpha}
}

func (l *LowPass[T]) Update(value T) T {
	if !l.initialized {
		l.state = value
		l.initialized = true
	} else {
		l.state = l.alpha*value + (1-l.alpha)*l.state
	}
	return l.state
}

func (l *LowPass[T]) Reset() {
	var zero T
	l.state = zero
	l.initialized = false
}

// Debounce latches its output to a new value only after `window` consecutive
// identical samples. Until the buffer fills, the most recent input passes
// through.
type Debounce[T comparable] struct {
	buffer []T
	index  int
	filled bool
	output T
	window int
}

func NewDebounce[T comparable](window int) *Debounce[T] {
	if window < 1 {
		panic("filter: Debounce window must be >= 1")
	}
	return &Debounce[T]{
		buffer: make([]T, window),
		window: window,
	}
}

func (d *Debounce[T]) Update(value T) T {
	d.buffer[d.index] = value
	d.index = (d.index + 1) % d.window
	if !d.filled && d.index == 0 {
		d.filled = true
	}
	if d.filled {
		stable := true
		first := d.buffer[0]
		for i := 1; i < d.window; i++ {
			if d.buffer[i] != first {
				stable = false
				break
			}
		}
		if stable {
			d.output = first
		}
	} else {
		d.output = value
	}
	return d.output
}

func (d *Debounce[T]) Reset() {
	var zero T
	for i := range d.buffer {
		d.buffer[i] = zero
	}
	d.index = 0
	d.filled = false
	d.output = zero
}
