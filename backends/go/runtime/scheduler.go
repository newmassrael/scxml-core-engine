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

// PopReadyEvent pops the next ready event and its data. Returns (event, data, true)
// if an event is ready, or (zero, "", false) if nothing is ready.
//
// Matches Rust PullScheduler::pop_ready_event.
func (s *PullScheduler[E]) PopReadyEvent() (E, string, bool) {
	now := time.Now()
	for i, e := range s.entries {
		if !e.readyAt.After(now) {
			// Remove entry at index i
			s.entries = append(s.entries[:i], s.entries[i+1:]...)
			return e.event, e.eventData, true
		}
	}
	var zero E
	return zero, "", false
}

// HasPendingEvents returns whether there are any scheduled events (ready or not).
func (s *PullScheduler[E]) HasPendingEvents() bool {
	return len(s.entries) > 0
}
