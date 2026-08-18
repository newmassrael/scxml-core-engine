// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

package sce

import (
	"fmt"
	"time"
)

// PullScheduler manages delayed event scheduling (§scxml-6.2).
//
// 1:1 port of Rust PullScheduler<E> from backends/rust/runtime/src/engine.rs.
// Stores delayed events with their ready-at time. The engine polls via
// PopReadyEvent during tick() to fire events whose delay has elapsed.
type PullScheduler[E any] struct {
	entries        []scheduledEntry[E]
	nextAutoSendID uint64
}

// scheduledEntry is a single scheduled event with its metadata.
type scheduledEntry[E any] struct {
	event     E
	eventData string
	sendID    string
	readyAt   time.Time
}

// NewPullScheduler constructs an empty scheduler.
func NewPullScheduler[E any]() *PullScheduler[E] {
	return &PullScheduler[E]{
		entries: make([]scheduledEntry[E], 0, 4),
	}
}

// ScheduleEvent schedules an event for delayed delivery (§scxml-6.2).
//
// If sendID is empty, an automatic ID is generated. Returns the ID used
// (caller can use it to cancel). Matches Rust PullScheduler::schedule_event.
func (s *PullScheduler[E]) ScheduleEvent(event E, delay time.Duration, sendID, eventData string) string {
	effectiveSendID := sendID
	if effectiveSendID == "" {
		s.nextAutoSendID++
		effectiveSendID = fmt.Sprintf("auto_send_%d", s.nextAutoSendID)
	}
	s.entries = append(s.entries, scheduledEntry[E]{
		event:     event,
		eventData: eventData,
		sendID:    effectiveSendID,
		readyAt:   time.Now().Add(delay),
	})
	return effectiveSendID
}

// CancelEvent cancels a scheduled event by send ID (§scxml-6.2.5).
// Returns true if the event was found and removed.
//
// Matches Rust PullScheduler::cancel_event.
func (s *PullScheduler[E]) CancelEvent(sendID string) bool {
	before := len(s.entries)
	n := 0
	for _, e := range s.entries {
		if e.sendID != sendID {
			s.entries[n] = e
			n++
		}
	}
	s.entries = s.entries[:n]
	return n < before
}

// HasReadyEvents returns whether any scheduled events are ready to fire
// (readyAt <= now).
//
// Matches Rust PullScheduler::has_ready_events.
func (s *PullScheduler[E]) HasReadyEvents() bool {
	now := time.Now()
	for _, e := range s.entries {
		if !e.readyAt.After(now) {
			return true
		}
	}
	return false
}

// PopReadyEvent pops the ready event that came due first and its data. Returns
// (event, data, true) if an event is ready, or (zero, "", false) if nothing is.
//
// Deadline order, not insertion order: the caller dispatches these one at a
// time and runs a macrostep between them, so whichever comes out first is the
// one whose transitions run first. Picking by insertion would let a
// later-scheduled event be delivered ahead of an earlier one whenever the host
// woke after both came due, which is the difference between a <cancel> landing
// and being lost. Ties keep insertion order.
//
// Matches Rust PullScheduler::pop_ready_event.
func (s *PullScheduler[E]) PopReadyEvent() (E, string, bool) {
	now := time.Now()
	best := -1
	for i, e := range s.entries {
		if e.readyAt.After(now) {
			continue
		}
		if best < 0 || e.readyAt.Before(s.entries[best].readyAt) {
			best = i
		}
	}
	if best < 0 {
		var zero E
		return zero, "", false
	}
	entry := s.entries[best]
	s.entries = append(s.entries[:best], s.entries[best+1:]...)
	return entry.event, entry.eventData, true
}

// HasPendingEvents returns whether there are any scheduled events (ready or not).
func (s *PullScheduler[E]) HasPendingEvents() bool {
	return len(s.entries) > 0
}

// NextReadyAt reports when the earliest still-queued entry comes due, whether
// or not it is ready yet. The bool is false when nothing is scheduled.
//
// The queue has always known this; nothing could ask. A host driving the
// machine has to decide when to call Tick again, and without this it can only
// guess an interval — see Engine.TimeUntilNextScheduled for what guessing
// costs. Matches Rust PullScheduler::next_ready_at.
func (s *PullScheduler[E]) NextReadyAt() (time.Time, bool) {
	if len(s.entries) == 0 {
		return time.Time{}, false
	}
	next := s.entries[0].readyAt
	for _, e := range s.entries[1:] {
		if e.readyAt.Before(next) {
			next = e.readyAt
		}
	}
	return next, true
}
