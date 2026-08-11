// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

package sce

// EventQueueManager is a FIFO queue for SCXML events with metadata.
//
// Ports Rust EventQueueManager<T> from backends/rust/runtime/src/helpers/event_queue.rs.
// §scxml-C-1: internal events are processed exhaustively before any external
// event. The engine's macrostep loop drains the internal queue first, then
// processes one external event, then re-drains the internal queue.
type EventQueueManager[T any] struct {
	queue []T
}

// NewEventQueueManager constructs an empty queue.
func NewEventQueueManager[T any]() *EventQueueManager[T] {
	return &EventQueueManager[T]{
		queue: make([]T, 0, 8),
	}
}

// Raise enqueues an event at the back of the FIFO queue (§scxml-3.12.1).
//
// Matches Rust raise(&mut self, event: T).
func (q *EventQueueManager[T]) Raise(event T) {
	q.queue = append(q.queue, event)
}

// Pop dequeues the next event (FIFO). Returns the event and true if the queue
// is non-empty, or the zero value and false if the queue is empty.
//
// Matches Rust pop(&mut self) -> Option<T>.
func (q *EventQueueManager[T]) Pop() (T, bool) {
	if len(q.queue) == 0 {
		var zero T
		return zero, false
	}
	event := q.queue[0]
	// Shift the slice forward. For small queues this is acceptable performance.
	// A ring buffer would be more efficient for high-throughput scenarios.
	q.queue = q.queue[1:]
	return event, true
}

// HasEvents returns whether the queue contains any events.
//
// Matches Rust has_events(&self) -> bool.
func (q *EventQueueManager[T]) HasEvents() bool {
	return len(q.queue) > 0
}

// Len returns the number of events currently queued.
func (q *EventQueueManager[T]) Len() int {
	return len(q.queue)
}

// IsEmpty returns whether the queue is empty (equivalent to !HasEvents()).
func (q *EventQueueManager[T]) IsEmpty() bool {
	return len(q.queue) == 0
}

// Clear removes all queued events.
func (q *EventQueueManager[T]) Clear() {
	q.queue = q.queue[:0]
}
