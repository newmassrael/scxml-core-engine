// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package testprofile

import (
	"fmt"

	sce "github.com/newmassrael/sce-go-runtime"
	"github.com/newmassrael/sce-go-runtime/testprofile/canonical"
)

// The test log of a test case run (docs/SCE_TEST_PROFILE.md §12.5;
// docs/adr/0091, C3 and answer 8): the records of TCI's test logging interface
// (ETSI ES 201 873-6), each written where the profile's semantics places its
// event, in the order the round scheduler produces them, and the one canonical
// text of a run's log, `schemas/sce-test-log.v1.schema.json`.
//
// The text is the JSON Canonicalization Scheme of the log (sce.WriteJCS), and
// it holds no number: an instant, a duration and every integer of a value are
// texts of their digits (package canonical). So two runs agree, under C3,
// exactly when their logs are the same bytes, whichever runtime wrote each.
//
// Cross-language siblings: `sce_rust_runtime::test::TestLogRecord` and
// `TestCaseResult::canonical_log` (Rust), whose text this one writes byte for
// byte, `SCE::Test::TestLogRecord` (C++), `com.sce.runtime.test.TestLogRecord`
// (Kotlin).

// TestLogSchema is the name of the test log's shape, its `schema`.
const TestLogSchema = "sce-test-log.v1"

// TestLogRecord is one record of the test log: an event and `NOW` when it
// happened.
type TestLogRecord struct {
	// At is `NOW` when it was recorded.
	At Instant
	// Event is what happened.
	Event TestLogEvent
}

// TestLogEvent is an event of TCI's test logging interface (ES 201 873-6), as
// the profile's semantics places it (docs/SCE_TEST_PROFILE.md §12.5): one of
// the types of this file, each a record's `kind`. A component is named `mtc`
// or by the PTC's name in its configuration.
type TestLogEvent interface {
	// Kind is the record's `kind` in the test log: the TCI event in kebab
	// case.
	Kind() string
	// writeMembers sets the members of its kind, beside `at` and `kind`, on
	// members. Unexported, so the kinds are this file's alone.
	writeMembers(members *sce.JSONObject)
}

// TestCaseStarted is the test case started, at `NOW = 0` (P11).
type TestCaseStarted struct{}

// Kind is `test-case-started`.
func (TestCaseStarted) Kind() string { return "test-case-started" }

func (TestCaseStarted) writeMembers(*sce.JSONObject) {}

// TestCaseEnded is the test case ended with its verdict (P14).
type TestCaseEnded struct {
	// Verdict is the test case verdict.
	Verdict Verdict
}

// Kind is `test-case-ended`.
func (TestCaseEnded) Kind() string { return "test-case-ended" }

func (e TestCaseEnded) writeMembers(members *sce.JSONObject) {
	members.Set("verdict", e.Verdict.String())
}

// ComponentCreated is a component created: the MTC as the test case starts, a
// PTC by the invocation that creates it (P9).
type ComponentCreated struct {
	// Component is the component.
	Component string
	// ComponentType is its component type.
	ComponentType string
}

// Kind is `component-created`.
func (ComponentCreated) Kind() string { return "component-created" }

func (e ComponentCreated) writeMembers(members *sce.JSONObject) {
	members.Set("component", e.Component)
	members.Set("type", e.ComponentType)
}

// ComponentDone is a component terminated, with its local verdict (P10).
type ComponentDone struct {
	// Component is the component.
	Component string
	// Verdict is its local verdict when it terminated.
	Verdict Verdict
}

// Kind is `component-done`.
func (ComponentDone) Kind() string { return "component-done" }

func (e ComponentDone) writeMembers(members *sce.JSONObject) {
	members.Set("component", e.Component)
	members.Set("verdict", e.Verdict.String())
}

// MessageSent is a component's send of a message on a port (P6).
type MessageSent struct {
	// Component is the sender.
	Component string
	// Port is the port it sent on.
	Port string
	// MessageType is the name of the message's record type
	// (TestValue.TypeName).
	MessageType string
	// Value is the message's value in canonical form (TestValue.Canonical).
	Value any
}

// Kind is `message-sent`.
func (MessageSent) Kind() string { return "message-sent" }

func (e MessageSent) writeMembers(members *sce.JSONObject) {
	writeMessage(members, e.Component, e.Port, e.MessageType, e.Value)
}

// MessageReceived is a receive branch's taking of the head of a port's queue
// (P4 step 1): the message was received and matched.
type MessageReceived struct {
	// Component is the receiver.
	Component string
	// Port is the port it took the message from.
	Port string
	// MessageType is the name of the message's record type
	// (TestValue.TypeName).
	MessageType string
	// Value is the message's value in canonical form (TestValue.Canonical).
	Value any
}

// Kind is `message-received`.
func (MessageReceived) Kind() string { return "message-received" }

func (e MessageReceived) writeMembers(members *sce.JSONObject) {
	writeMessage(members, e.Component, e.Port, e.MessageType, e.Value)
}

// writeMessage sets the members a sent and a received message share.
func writeMessage(members *sce.JSONObject, component, port, messageType string, value any) {
	members.Set("component", component)
	members.Set("port", port)
	members.Set("type", messageType)
	members.Set("value", value)
}

// TimerStarted is a component's start of a timer, for a duration in
// microseconds (P7).
type TimerStarted struct {
	// Component is the component.
	Component string
	// Timer is the timer.
	Timer string
	// Duration is the duration, in microseconds.
	Duration Duration
}

// Kind is `timer-started`.
func (TimerStarted) Kind() string { return "timer-started" }

func (e TimerStarted) writeMembers(members *sce.JSONObject) {
	members.Set("component", e.Component)
	members.Set("timer", e.Timer)
	members.Set("duration_us", canonical.Integer(e.Duration))
}

// TimerStopped is a component's stop of a running timer (P7). A stop of a timer
// that is not running changes nothing, and is not recorded.
type TimerStopped struct {
	// Component is the component.
	Component string
	// Timer is the timer.
	Timer string
}

// Kind is `timer-stopped`.
func (TimerStopped) Kind() string { return "timer-stopped" }

func (e TimerStopped) writeMembers(members *sce.JSONObject) {
	members.Set("component", e.Component)
	members.Set("timer", e.Timer)
}

// TimerTimedOut is a timeout branch's taking of a timer that had timed out (P4
// step 1).
type TimerTimedOut struct {
	// Component is the component.
	Component string
	// Timer is the timer.
	Timer string
}

// Kind is `timer-timed-out`.
func (TimerTimedOut) Kind() string { return "timer-timed-out" }

func (e TimerTimedOut) writeMembers(members *sce.JSONObject) {
	members.Set("component", e.Component)
	members.Set("timer", e.Timer)
}

// VerdictSet is a component's local verdict set: by `<sce:setverdict>` (P12),
// or to `error` by the runtime (P13). The reason is the document's, and none is
// recorded for an empty one or for the runtime's `error`, whose cause is the
// runtime's own prose, which another runtime would not write alike.
type VerdictSet struct {
	// Component is the component.
	Component string
	// Verdict is the verdict it was given.
	Verdict Verdict
	// Reason is the reason the document gave, empty for none.
	Reason string
}

// Kind is `verdict-set`.
func (VerdictSet) Kind() string { return "verdict-set" }

func (e VerdictSet) writeMembers(members *sce.JSONObject) {
	members.Set("component", e.Component)
	members.Set("verdict", e.Verdict.String())
	if e.Reason != "" {
		members.Set("reason", e.Reason)
	}
}

// Logged is a `<log>` (P8). It is not named Log, as the other runtimes name
// the event, since a TestCaseResult's lines for a reader are its Log and a
// `<log>`'s line of them is LogLog.
type Logged struct {
	// Component is the component.
	Component string
	// Text is the text.
	Text string
}

// Kind is `log`.
func (Logged) Kind() string { return "log" }

func (e Logged) writeMembers(members *sce.JSONObject) {
	members.Set("component", e.Component)
	members.Set("text", e.Text)
}

// ToValue is the record as the test log writes it: `at`, `kind`, and the
// members of its kind.
func (r TestLogRecord) ToValue() *sce.JSONObject {
	members := sce.NewJSONObject()
	members.Set("at", canonical.Integer(r.At))
	members.Set("kind", r.Event.Kind())
	r.Event.writeMembers(members)
	return members
}

// CanonicalLog is the test log of this run of the test case testCase of the
// suite suite, in its one canonical text: the JSON Canonicalization Scheme of
// `schema`, `suite`, `test_case`, `verdict` and `records`, each record in the
// order the run produced it.
//
// It panics when a message's value is not in the canonical form
// (TestValue.Canonical) but holds a number past the range of a double, a text
// that is not UTF-8 or a value of a type no JSON value is held as, which no
// writer of package canonical builds.
func (r TestCaseResult) CanonicalLog(suite, testCase string) string {
	records := make([]any, 0, len(r.Records))
	for _, record := range r.Records {
		records = append(records, record.ToValue())
	}
	log := sce.NewJSONObject()
	log.Set("schema", TestLogSchema)
	log.Set("suite", suite)
	log.Set("test_case", testCase)
	log.Set("verdict", r.Verdict.String())
	log.Set("records", records)
	text, err := sce.WriteJCS(log)
	if err != nil {
		panic(fmt.Sprintf("the test log of %s %s is not canonical: %v", suite, testCase, err))
	}
	return text
}

// CanonicalLog is the test log of this run of the test case in the suite
// suite, in its one canonical text (TestCaseResult.CanonicalLog).
func (c CaseResult) CanonicalLog(suite string) string {
	return c.Result.CanonicalLog(suite, c.TestCase)
}
