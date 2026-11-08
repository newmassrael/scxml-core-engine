// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// Test components generated in Go (docs/SCE_TEST_PROFILE.md §12.3, §12.6
// slice 6), run end to end by the profile's runtime, testprofile.TestCaseRun.
// The machines are the statecharts sce-codegen writes from the fixtures under
// `sce-build/tests/fixtures/test_profile/runtime/` into a package each beside
// this file, and the records and templates they name into packages of their
// own beside them (regen: `scripts/regen_static_datamodel_go.sh`), driven only
// through the doors of §12.3; the configuration, the factory of machines and
// the system adapter are written here.
//
// Each test is the twin of the Rust case of the same name in
// backends/rust/tests/tests/a_test_component_runs_through_its_doors.rs, named
// `TestATestComponentRunsThroughItsDoors_<Case>` after it in Pascal case
// (`a_test_component_that_is_answered_with_a_matching_reply_passes` is
// TestATestComponentRunsThroughItsDoors_ATestComponentThatIsAnsweredWithAMatchingReplyPasses),
// itself the twin of the C++ gtest in
// tests/integration/ATestComponentRunsThroughItsDoorsTest.cpp and of the Kotlin
// ATestComponentRunsThroughItsDoorsTest.kt, with the same scenario, verdict,
// `NOW` and log.

package static_datamodel

import (
	"fmt"
	"testing"

	"github.com/newmassrael/sce-go-runtime/testprofile"

	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/answerer"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/asker"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/coordinator"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/idler"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/job_reply"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/job_request"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/ptc_keeper"
)

// componentMillisecond is a millisecond of `NOW`, which counts microseconds.
const componentMillisecond testprofile.Duration = 1000

// componentSecond is a second of `NOW`.
const componentSecond testprofile.Duration = 1000 * componentMillisecond

// componentAnswerDelay is how long the system under test takes to answer a
// request.
const componentAnswerDelay testprofile.Duration = 500

// componentMachineOf is the generated machine of each component type a
// configuration here names, and none for another type, which ends the run
// with `error`.
func componentMachineOf(componentType string) testprofile.ComponentMachine {
	switch componentType {
	case "asker":
		return &asker.AskerComponent{}
	case "coordinator":
		return &coordinator.CoordinatorComponent{}
	case "answerer":
		return &answerer.AnswererComponent{}
	case "ptc_keeper":
		return &ptc_keeper.PtcKeeperComponent{}
	case "idler":
		return &idler.IdlerComponent{}
	}
	return nil
}

// componentSent is one send the system under test was handed: its system port
// and its event.
type componentSent struct {
	systemPort string
	event      string
}

// componentSeen is what the system under test was handed: each send, and the
// job each request named.
type componentSeen struct {
	sent      []componentSent
	requested []uint32
}

// componentPending is a reply the system under test will hand the run at its
// instant.
type componentPending struct {
	at      testprofile.Instant
	message testprofile.Message
}

// componentResponder is the system under test on the mapped port `sut`: it
// answers each request, componentAnswerDelay after it, with a reply of code
// for the requested job, or for job when answersJob is set. It records what it
// was sent in seen, which the test keeps while the run holds the adapter.
type componentResponder struct {
	job        uint32
	answersJob bool
	code       uint32
	pending    []componentPending
	seen       *componentSeen
}

func (r *componentResponder) Send(systemPort string, message testprofile.Message, now testprofile.Instant) {
	r.seen.sent = append(r.seen.sent, componentSent{systemPort: systemPort, event: message.Event})
	request, ok := testprofile.ValueAs[job_request.JobRequestPayload](message)
	if !ok {
		return
	}
	r.seen.requested = append(r.seen.requested, request.Job)
	job := request.Job
	if r.answersJob {
		job = r.job
	}
	reply := job_reply.JobReplyPayload{Job: job, Code: r.code}
	r.pending = append(r.pending, componentPending{
		at:      now + componentAnswerDelay,
		message: testprofile.Message{Event: job_reply.EventName, Value: reply},
	})
}

func (r *componentResponder) NextInput() (testprofile.Instant, bool) {
	if len(r.pending) == 0 {
		return 0, false
	}
	return r.pending[0].at, true
}

func (r *componentResponder) TakeInputsDue(now testprofile.Instant) []testprofile.SystemInput {
	var due []testprofile.SystemInput
	for len(r.pending) > 0 && r.pending[0].at <= now {
		due = append(due, testprofile.SystemInput{SystemPort: "sut", Message: r.pending[0].message})
		r.pending = r.pending[1:]
	}
	return due
}

// componentSilent is a system under test nothing is mapped to.
type componentSilent struct{}

func (componentSilent) Send(systemPort string, message testprofile.Message, _ testprofile.Instant) {
	panic(fmt.Sprintf("no port is mapped, so nothing reaches the system adapter, yet %s was sent on %s",
		message.Event, systemPort))
}

func (componentSilent) NextInput() (testprofile.Instant, bool) {
	return 0, false
}

func (componentSilent) TakeInputsDue(testprofile.Instant) []testprofile.SystemInput {
	return nil
}

// componentFirstRecord is the index of the first record of kind by component,
// and false when there is none.
func componentFirstRecord(result testprofile.TestCaseResult, component string, kind testprofile.LogKind) (int, bool) {
	for at, record := range result.Log {
		if record.Component == component && record.Kind == kind {
			return at, true
		}
	}
	return 0, false
}

// componentRecord is the index of the first record of kind by component,
// which the log holds.
func componentRecord(t *testing.T, result testprofile.TestCaseResult, component string, kind testprofile.LogKind) int {
	t.Helper()
	at, ok := componentFirstRecord(result, component, kind)
	if !ok {
		t.Fatalf("%s logs %v: %v", component, kind, result.Log)
	}
	return at
}

func componentRun(configuration testprofile.Configuration, adapter testprofile.SystemAdapter) testprofile.TestCaseResult {
	return testprofile.NewTestCaseRun(configuration, componentMachineOf, adapter, testprofile.NoGuard, nil).Run()
}

// componentAskerConfiguration is the asker as the MTC, its port `sut` mapped to
// the system port `sut`.
func componentAskerConfiguration() testprofile.Configuration {
	return testprofile.Configuration{
		MTCType:      "asker",
		PortsOfType:  map[string][]string{"asker": {"sut"}},
		TimersOfType: map[string][]string{"asker": {"reply"}},
		Mappings:     []testprofile.Mapping{{Port: testprofile.Endpoint{Component: testprofile.MTCName, Port: "sut"}, SystemPort: "sut"}},
	}
}

// componentRunAsker runs the asker against a responder that answers with
// code, for the requested job or for job when answersJob is set.
func componentRunAsker(job uint32, answersJob bool, code uint32) (testprofile.TestCaseResult, *componentSeen) {
	seen := &componentSeen{}
	responder := &componentResponder{job: job, answersJob: answersJob, code: code, seen: seen}
	return componentRun(componentAskerConfiguration(), responder), seen
}

// P6, P2, P4: the send carries the template's value bound to the component's
// variable, a reply its receive branch's template matches is taken when it
// arrives, and the content reads the reply as `_event.data`.
func TestATestComponentRunsThroughItsDoors_ATestComponentThatIsAnsweredWithAMatchingReplyPasses(t *testing.T) {
	result, seen := componentRunAsker(0, false, 0)
	if result.Verdict != testprofile.VerdictPass {
		t.Fatalf("verdict %v: %v", result.Verdict, result.Log)
	}
	if result.EndedAt != componentAnswerDelay {
		t.Errorf("the reply is taken in the round it arrives in: ended at %d", result.EndedAt)
	}
	if len(seen.sent) != 1 || seen.sent[0] != (componentSent{systemPort: "sut", event: job_request.EventName}) {
		t.Errorf("a message travels under its template's record's event: %v", seen.sent)
	}
	if len(seen.requested) != 1 || seen.requested[0] != 7 {
		t.Errorf("the template's value, given `job`: %v", seen.requested)
	}
	componentRecord(t, result, testprofile.MTCName, testprofile.LogReceive)
	// P8: the `<log>` of the branch's content is in the test log, with the
	// instant and the component, its label and the job it read.
	logged := componentRecord(t, result, testprofile.MTCName, testprofile.LogLog)
	if got := result.Log[logged].Detail; got != "answered: 7" {
		t.Errorf("the log reads %q", got)
	}
	if got := result.Log[logged].At; got != componentAnswerDelay {
		t.Errorf("the log is recorded at %d", got)
	}
}

// P2, P7, §7: a reply the template does not match enables no branch and is not
// removed, and the timer's timeout branch fails the test case.
func TestATestComponentRunsThroughItsDoors_ATestComponentThatIsAnsweredWithAReplyItsTemplateDoesNotMatchFailsWhenItsTimerFires(t *testing.T) {
	result, _ := componentRunAsker(0, false, 3)
	if result.Verdict != testprofile.VerdictFail {
		t.Fatalf("verdict %v: %v", result.Verdict, result.Log)
	}
	if result.EndedAt != componentSecond {
		t.Errorf("time advances to the timer's instant: ended at %d", result.EndedAt)
	}
	if _, ok := componentFirstRecord(result, testprofile.MTCName, testprofile.LogReceive); ok {
		t.Errorf("a reply that does not match is never taken: %v", result.Log)
	}
	componentRecord(t, result, testprofile.MTCName, testprofile.LogTimeout)
}

// P4: the content reads the message the branch took: a reply of success for
// another job matches the template, which takes any job, and the content's
// comparison of the job it read fails the test case.
func TestATestComponentRunsThroughItsDoors_ATestComponentReadsTheReplyItTookAsTheEventsData(t *testing.T) {
	result, _ := componentRunAsker(8, true, 0)
	if result.Verdict != testprofile.VerdictFail {
		t.Fatalf("verdict %v: %v", result.Verdict, result.Log)
	}
	if result.EndedAt != componentAnswerDelay {
		t.Errorf("ended at %d", result.EndedAt)
	}
	componentRecord(t, result, testprofile.MTCName, testprofile.LogReceive)
}

// P9, P6, P2: the MTC creates a PTC connected to it, which asks it for job 42
// as it starts; the MTC answers with the job it read, the PTC passes on the
// reply and terminates, and the MTC's done branch passes on the next snapshot.
// Leaving the invoking state kills the PTC, before the done branch's content
// runs.
func TestATestComponentRunsThroughItsDoors_ATestComponentCreatesAPtcExchangesAMessageWithItAndReadsItsDone(t *testing.T) {
	configuration := testprofile.Configuration{
		MTCType:     "coordinator",
		PTCTypes:    map[string]string{"answerer": "answerer"},
		PortsOfType: map[string][]string{"coordinator": {"peer"}, "answerer": {"peer"}},
		Connections: []testprofile.Connection{{A: testprofile.Endpoint{Component: testprofile.MTCName, Port: "peer"}, B: testprofile.Endpoint{Component: "answerer", Port: "peer"}}},
	}
	result := componentRun(configuration, componentSilent{})

	if result.Verdict != testprofile.VerdictPass {
		t.Fatalf("verdict %v: %v", result.Verdict, result.Log)
	}
	if result.EndedAt != 0 {
		t.Errorf("a branch fired in every round, so time never moved: ended at %d", result.EndedAt)
	}
	created := componentRecord(t, result, "answerer", testprofile.LogCreate)
	asked := componentRecord(t, result, "answerer", testprofile.LogSend)
	answered := componentRecord(t, result, testprofile.MTCName, testprofile.LogSend)
	done := componentRecord(t, result, "answerer", testprofile.LogDone)
	killed := componentRecord(t, result, "answerer", testprofile.LogKill)
	passed := componentRecord(t, result, testprofile.MTCName, testprofile.LogVerdict)
	if created >= asked {
		t.Errorf("the PTC is created when the macrostep that entered its state ends: %v", result.Log)
	}
	// P8: a `<log>` that writes only a label is recorded, by the PTC.
	noted := componentRecord(t, result, "answerer", testprofile.LogLog)
	if got := result.Log[noted].Detail; got != "asked" {
		t.Errorf("the PTC's log reads %q", got)
	}
	if asked >= noted {
		t.Errorf("the log runs in its place, after the send before it: %v", result.Log)
	}
	if got := result.Log[asked].Detail; got != "peer job.request" {
		t.Errorf("the PTC sends %q", got)
	}
	if got := result.Log[answered].Detail; got != "peer job.reply" {
		t.Errorf("the MTC sends %q", got)
	}
	if answered >= done {
		t.Errorf("the MTC answers before the PTC is done: %v", result.Log)
	}
	if done >= killed {
		t.Errorf("a done branch fires on a snapshot taken after the PTC terminated: %v", result.Log)
	}
	if killed >= passed {
		t.Errorf("the invoking state is exited before the done branch's content runs: %v", result.Log)
	}
}

// P9, P14: leaving the state that invoked a PTC removes it while it still runs,
// at that instant, and its local verdict still counts.
func TestATestComponentRunsThroughItsDoors_ATestComponentKillsAPtcOnLeavingTheStateThatInvokedIt(t *testing.T) {
	configuration := testprofile.Configuration{
		MTCType:      "ptc_keeper",
		PTCTypes:     map[string]string{"idler": "idler"},
		PortsOfType:  map[string][]string{"idler": {"peer"}},
		TimersOfType: map[string][]string{"ptc_keeper": {"patience", "linger"}},
	}
	result := componentRun(configuration, componentSilent{})

	killed := componentRecord(t, result, "idler", testprofile.LogKill)
	if got := result.Log[killed].At; got != 10*componentMillisecond {
		t.Errorf("the PTC is removed when `patience` runs out, not at %d", got)
	}
	if _, ok := componentFirstRecord(result, "idler", testprofile.LogDone); ok {
		t.Errorf("the idler never terminates: %v", result.Log)
	}
	if result.EndedAt != 15*componentMillisecond {
		t.Errorf("ended at %d", result.EndedAt)
	}
	if result.Verdict != testprofile.VerdictInconclusive {
		t.Errorf("the removed PTC's inconclusive outweighs the MTC's pass: %v", result.Verdict)
	}
}
