// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

// Package observer provides observer building blocks: hysteresis state and
// fixed-capacity event queue. See SCE_FORGE.md Section 4.11.
//
// Go has no associated types, so the cross-file event domain is expressed
// purely through the EventQueue type parameter — two queues with different
// Tag type parameters are incompatible types and the compiler rejects
// mixing them.
package observer

// ThresholdState models a 1-bit hysteresis state machine.
//
// The generated update loop calls EnterIf(highCondition) and
// LeaveIf(lowCondition); both methods return true exactly when a transition
// actually occurred, so the generated code can push the corresponding event
// without re-checking state.
type ThresholdState struct {
	active bool
}

func (s *ThresholdState) EnterIf(condition bool) bool {
	if !s.active && condition {
		s.active = true
		return true
	}
	return false
}

func (s *ThresholdState) LeaveIf(condition bool) bool {
	if s.active && condition {
		s.active = false
		return true
	}
	return false
}

func (s *ThresholdState) Active() bool { return s.active }
func (s *ThresholdState) Reset()       { s.active = false }

// EventQueue is a FIFO of domain-tagged events. Returned by value from
// observer Update methods. Backed by a slice — Go has no embedded heap
// constraint, so a slice is the natural data structure here. The cross-
// language behavioural contract (push, length, iteration order, clear)
// matches the C++/Rust/Python/Kotlin implementations exactly.
//
// Tag is constrained to comparable so the queue can be exported to consumers
// that need to dispatch on tag identity (e.g., a switch statement).
type EventQueue[Tag comparable] struct {
	buffer []Tag
}

// NewEventQueue constructs an empty EventQueue.
func NewEventQueue[Tag comparable]() *EventQueue[Tag] {
	return &EventQueue[Tag]{}
}

// Push appends a tag to the queue. Always returns true (the slice grows
// dynamically); the boolean return matches the C++/Rust API where push can
// fail on a fixed-capacity buffer.
func (q *EventQueue[Tag]) Push(tag Tag) bool {
	q.buffer = append(q.buffer, tag)
	return true
}

func (q *EventQueue[Tag]) Len() int        { return len(q.buffer) }
func (q *EventQueue[Tag]) IsEmpty() bool   { return len(q.buffer) == 0 }
func (q *EventQueue[Tag]) Get(i int) Tag   { return q.buffer[i] }
func (q *EventQueue[Tag]) AsSlice() []Tag  { return append([]Tag{}, q.buffer...) }
func (q *EventQueue[Tag]) Clear()          { q.buffer = q.buffer[:0] }
