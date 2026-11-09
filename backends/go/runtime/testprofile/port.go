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

// TestValue is a value a message carries, its type's name and its canonical
// form in the test log (docs/SCE_TEST_PROFILE.md §12.5).
//
// The value is read back by its type (ValueAs); its canonical form is written
// by the types its document declares (package canonical), which its Go type
// alone does not say. MessageValue makes one of a value, its type's name and
// the writer of its form, which is what a generated send and a system adapter
// give; a host may implement it on a type of its own.
//
// Cross-language siblings: `sce_rust_runtime::test::TestValue` (Rust),
// `com.sce.runtime.test.TestValue` (Kotlin).
type TestValue interface {
	// Any is the value, to be read back by its type.
	Any() any
	// TypeName is the name of the value's record as the types module of the
	// suite's TTCN-3 export names it (docs/SCE_TEST_PROFILE.md §9.5): the unit
	// name of its `event-schema` document. A value in TCI always carries its
	// type, and so does a message of the test log.
	TypeName() string
	// Canonical is the value in the canonical form of the test log: built
	// from texts, truth values, arrays and objects alone (package canonical),
	// so that the JSON Canonicalization Scheme (sce.WriteJCS) writes it as the
	// same bytes on every runtime. It runs while the run's state is held, so
	// it reads the value and nothing else.
	Canonical() any
}

// withCanonical is a value, its type's name and the writer of its canonical
// form: what MessageValue makes.
type withCanonical[V any] struct {
	value     V
	typeName  string
	canonical func(V) any
}

func (w withCanonical[V]) Any() any {
	return w.value
}

func (w withCanonical[V]) TypeName() string {
	return w.typeName
}

func (w withCanonical[V]) Canonical() any {
	return w.canonical(w.value)
}

// String shows the value's type's name alone: the value's type is not known
// here.
func (w withCanonical[V]) String() string {
	return "TestValue(typeName=" + w.typeName + ", ..)"
}

// MessageValue is value as a message's value, of the record typeName, whose
// canonical form canonical writes: the `TYPE` and the `Canonical` of the unit
// of a template over its record, as a generated send gives them
// (`<template>.TYPE`, `<template>.Canonical`), on the request's
// sce.HostSendRequest.Value.
func MessageValue[V any](value V, typeName string, canonical func(V) any) TestValue {
	return withCanonical[V]{value: value, typeName: typeName, canonical: canonical}
}

// Message is a message on a port: the event name of its record and the
// record's value.
//
// The value is of the record type the unit of its `event-schema` declares; a
// template's matcher reads it as that type (ValueAs). Matching is on the value
// and never on an encoding (ADR 0091 C2), and so is the test log
// (CanonicalValue). A value is never changed once sent, so a message is copied
// freely: every copy reads the same record.
type Message struct {
	// Event is the event name of the message's record.
	Event string
	// Value is the record's value, with its type's name and the writer of its
	// canonical form.
	Value TestValue
}

// NewMessage is a message of the record event whose value is value, of the
// record type typeName, whose canonical form canonical writes (MessageValue).
func NewMessage[V any](event string, value V, typeName string, canonical func(V) any) Message {
	return Message{Event: event, Value: MessageValue(value, typeName, canonical)}
}

// String shows the message's event alone: the value's type is not known here.
func (m Message) String() string {
	return "Message(event=" + m.Event + ", ..)"
}

// TypeName is the name of the value's record type in the test log
// (TestValue.TypeName).
func (m Message) TypeName() string {
	return m.Value.TypeName()
}

// CanonicalValue is the value in the canonical form of the test log
// (TestValue.Canonical).
func (m Message) CanonicalValue() any {
	return m.Value.Canonical()
}

// ValueAs is the value of message and true when it is a V, and the zero V and
// false when it is of another type, which a template of V does not match, or
// when the message carries no value at all.
func ValueAs[V any](message Message) (V, bool) {
	if message.Value == nil {
		var none V
		return none, false
	}
	value, ok := message.Value.Any().(V)
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
