// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package sce

import "time"

// SceClock is the source of "now" behind every <send delay> an Engine arms and
// every due judgement its Tick makes.
//
// §scxml-6.2.2 says a delay "indicates how long the processor should wait
// before dispatching the message", and says nothing about where the processor
// reads the time from. Leaving that hardwired to the wall answers a question
// the spec left to the host, and answers it the one way that cannot be
// reproduced: a host descheduled between two statements of the same <onentry>
// gets two different readings for one instant, and the deadlines it computes
// from them can order the sends differently on every run.
//
// So the reading is a seam, not a constant. MonotonicClock is the default and
// is what a production host wants; ManualClock hands the clock to the host
// outright, which is what a simulation, a replay, a discrete-event scheduler
// and a deterministic test all want. Both are runtime types on the shipped
// surface — a consumer can install either, or write a third.
type SceClock interface {
	// ElapsedMs reports milliseconds since this clock's origin.
	//
	// Must be non-decreasing: the scheduler compares readings taken at
	// different moments, and a reading that went backwards would make an entry
	// that was due stop being due.
	ElapsedMs() int64
}

// MonotonicClock is the default SceClock — Go's monotonic reading of the
// host's clock, measured from the moment this clock was constructed.
//
// This is what an engine gets when nothing else is installed, and what a
// production host running against real time should keep. time.Since carries
// Go's monotonic component, so it is unaffected by wall-clock adjustments.
type MonotonicClock struct {
	origin time.Time
}

// NewMonotonicClock returns a MonotonicClock whose origin is now.
func NewMonotonicClock() *MonotonicClock {
	return &MonotonicClock{origin: time.Now()}
}

// ElapsedMs implements SceClock.
func (c *MonotonicClock) ElapsedMs() int64 {
	return int64(time.Since(c.origin) / time.Millisecond)
}

// ManualClock is a SceClock the host moves by hand.
//
// Time advances only when Advance is called, so a machine driven through one of
// these reaches the same configuration on every run regardless of what else the
// machine it runs on is doing. That is what makes it the right clock for a
// simulation, for replaying a recorded trace at a speed of the host's choosing,
// and for a test that wants a verdict about the engine rather than about the
// load on the build machine.
//
// Install it before Engine.Initialize, and drive the machine with
// Engine.AdvanceTimeMs rather than calling Advance directly — the engine's
// entry point moves this clock and then runs whatever that made due, which is
// the whole of the contract.
//
// One instance may be shared by several engines, so a parent and the sessions
// it invokes read the same absolute time (§scxml-6.4).
type ManualClock struct {
	nowMs int64
}

// NewManualClock returns a ManualClock reading startMs.
//
// Panics on a negative origin: ElapsedMs is required to be non-decreasing and
// the scheduler's deadlines are computed by addition, so a negative origin is a
// caller error rather than a coordinate choice.
func NewManualClock(startMs int64) *ManualClock {
	if startMs < 0 {
		panic("sce: ManualClock origin must not be negative")
	}
	return &ManualClock{nowMs: startMs}
}

// ElapsedMs implements SceClock.
func (c *ManualClock) ElapsedMs() int64 {
	return c.nowMs
}

// Advance moves this clock forward by ms milliseconds.
//
// Rejects a negative delta rather than accepting it: SceClock.ElapsedMs is
// required to be non-decreasing, and a clock that went backwards would un-due
// an entry the scheduler had already judged ready.
func (c *ManualClock) Advance(ms int64) {
	if ms < 0 {
		panic("sce: ManualClock.Advance requires a non-negative delta")
	}
	c.nowMs += ms
}
