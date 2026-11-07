// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package testprofile

// A port of a test component (docs/SCE_TEST_PROFILE.md §2.2): a FIFO queue of
// received messages and the snapshot of its head (ES 201 873-4 §8.3.3).
//
// Cross-language siblings: `sce_rust_runtime::test::Port` (Rust),
// `SCE::Test::Port` (C++), `com.sce.runtime.test.Port` (Kotlin).

// Instant is an instant of the logical clock `NOW`: microseconds since the test
// case started (docs/SCE_TEST_PROFILE.md §3.3).
type Instant = int64

// Duration is a length of logical time, in microseconds.
type Duration = int64

// Message is a message on a port: the event name of its record and the
// record's value.
//
// The value is the record type the statechart generator writes for the
// `event-schema` that declares it; a template's matcher reads it as that type
// (ValueAs). Matching is on the value and never on an encoding (ADR 0091 C2).
// A value is never changed once sent, so a message is copied freely: every
// copy reads the same record.
type Message struct {
	// Event is the event name of the message's record.
	Event string
	// Value is the record's value, read back by its type.
	Value any
}

// String shows the message's event alone: the value's type is not known here.
func (m Message) String() string {
	return "Message(event=" + m.Event + ", ..)"
}

// ValueAs is the value of message and true when it is a V, and the zero V and
// false when it is of another type, which a template of V does not match.
func ValueAs[V any](message Message) (V, bool) {
	value, ok := message.Value.(V)
	return value, ok
}

// Port is one component port: its queue and its snapshot (-4 §8.3.3).
//
// Only the component that owns the port removes from it, so between the
// snapshot and the removal of a branch the head is the message the snapshot
// saw; what others append goes to the back. The snapshot records only whether
// the queue held a message: a message that arrives in an empty queue after the
// snapshot is not in it. The zero value is an empty port whose snapshot saw
// nothing.
type Port struct {
	queue         []Message
	snapshotHolds bool
}

// Enqueue appends a message a connected port sent, or one from the SUT on a
// mapped port, in arrival order (-4 §8.3.3.1).
func (p *Port) Enqueue(message Message) {
	p.queue = append(p.queue, message)
}

// TakeSnapshot takes `SNAP-VALUE`, the head of the queue, in a round's snapshot
// step (docs/SCE_TEST_PROFILE.md §3.2 step 3).
func (p *Port) TakeSnapshot() {
	p.snapshotHolds = len(p.queue) > 0
}

// Snapshot is the message the last snapshot saw at the head and true, or the
// zero message and false when it saw none.
func (p *Port) Snapshot() (Message, bool) {
	if !p.snapshotHolds || len(p.queue) == 0 {
		return Message{}, false
	}
	return p.queue[0], true
}

// RemoveHead is `remove-from-port` of a receive branch that was taken on the
// snapshot (-4 §9.37.1, P4 step 1).
//
// It panics when the queue is empty. The caller holds that the snapshot saw a
// message, which a branch's guard established, so an empty queue here is a
// machine that took a receive branch its guard did not allow.
func (p *Port) RemoveHead() Message {
	if len(p.queue) == 0 {
		panic("a receive branch is taken only on a snapshot that saw a message")
	}
	head := p.queue[0]
	// The slot is cleared so that the queue's array does not keep the value
	// alive once the message is gone.
	p.queue[0] = Message{}
	p.queue = p.queue[1:]
	p.snapshotHolds = false
	return head
}

// Len is how many messages the queue holds.
func (p *Port) Len() int {
	return len(p.queue)
}

// IsEmpty reports whether the queue holds no message.
func (p *Port) IsEmpty() bool {
	return len(p.queue) == 0
}
