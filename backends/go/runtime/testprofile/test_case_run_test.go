// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The test profile's runtime (docs/SCE_TEST_PROFILE.md §12.2), held to the
// rules of §2 to §5 with components written here by hand, before any machine
// is generated: each one keeps its context from Start on, reads its snapshot
// and calls its context as a generated machine does (§12.3). The runner of a
// suite is held to P11 over test cases made of the same components.
//
// Each test is the twin of the Rust case of the same module and name in
// backends/rust/runtime/tests/a_test_case_runs_as_the_test_profile_says.rs,
// named `Test<Module>_<Case>` in Pascal case
// (`test_case_run_test::an_answered_request_passes_without_time_moving` is
// TestTestCaseRunTest_AnAnsweredRequestPassesWithoutTimeMoving), itself the
// twin of the C++ gtest in tests/test_runtime/TestCaseRunTest.cpp and of the
// Kotlin case in backends/kotlin/tests/src/test/kotlin/com/sce/runtime/test/,
// with the same scenario, verdict, `NOW` and log. The cases of TheSharedHandle
// hold what the Rust runtime's handle holds, which the Go context holds too:
// re-entrant use is refused, and a context outlives no component it names.
// TheRunIsRunOnce holds what only the Go and Kotlin runtimes have to, since
// their run is not consumed by running it. The case of TheDoors reaches the run
// only through the two host processors a generated component registers on its
// engine (§12.3), written here with the types the engine registers them under.
// The cases of TheTestLog hold the records of a run and its canonical text to
// the bytes the Rust module `the_test_log` expects. Of TestPort, the case of a
// message's value has no Rust twin: it holds what only a Go message can lack,
// a value at all.

package testprofile

import (
	"errors"
	"fmt"
	"reflect"
	"strings"
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"
	"github.com/newmassrael/sce-go-runtime/testprofile/canonical"
)

const second Duration = 1_000_000

// step is what a hand-written component does when it starts, or on a
// snapshot.
type step func(context *ComponentContext, self *scripted)

func idle() step {
	return func(*ComponentContext, *scripted) {}
}

// steps are a component's two steps: when it starts, and on each snapshot.
type steps struct {
	onStart    step
	onSnapshot step
}

// scripted is a component written by hand: what it does when it starts, and
// what it does on each snapshot. Either may end it, as entering a top-level
// final does.
type scripted struct {
	steps
	context *ComponentContext
	ended   bool
}

func (s *scripted) end() {
	s.ended = true
}

func (s *scripted) Start(context *ComponentContext) {
	s.context = context
	s.onStart(context, s)
}

func (s *scripted) DeliverSnapshot() {
	if s.context == nil {
		panic("a snapshot is delivered only to a started machine")
	}
	s.onSnapshot(s.context, s)
}

func (s *scripted) Terminated() bool {
	return s.ended
}

// machines are the machines of a test, by component type.
type machines map[string]steps

func (m machines) with(componentType string, s steps) machines {
	m[componentType] = s
	return m
}

func (m machines) factory() MachineFactory {
	known := map[string]steps{}
	for componentType, s := range m {
		known[componentType] = s
	}
	return func(componentType string) ComponentMachine {
		s, ok := known[componentType]
		if !ok {
			return nil
		}
		return &scripted{steps: s}
	}
}

// input is one input of a trace, or one send the adapter was handed.
type input struct {
	at         Instant
	systemPort string
	event      string
}

// traceAdapter replays a trace and records what was sent to it.
type traceAdapter struct {
	trace []input
	next  int
	sent  []input
}

func (a *traceAdapter) Send(systemPort string, message Message, now Instant) {
	a.sent = append(a.sent, input{at: now, systemPort: systemPort, event: message.Event})
}

func (a *traceAdapter) NextInput() (Instant, bool) {
	if a.next < len(a.trace) {
		return a.trace[a.next].at, true
	}
	return 0, false
}

func (a *traceAdapter) TakeInputsDue(now Instant) []SystemInput {
	var due []SystemInput
	for a.next < len(a.trace) && a.trace[a.next].at <= now {
		in := a.trace[a.next]
		due = append(due, SystemInput{SystemPort: in.systemPort, Message: message(in.event)})
		a.next++
	}
	return due
}

// message is a message of the record event that holds no field: `{}` in the
// test log, of the record type `empty`.
func message(event string) Message {
	return NewMessage(event, struct{}{}, "empty", func(struct{}) any { return canonical.Record() })
}

// writtenText is n as a text in the test log, of the record type `text`.
func writtenText(event, n string) Message {
	return NewMessage(event, n, "text", func(n string) any { return canonical.Text(n) })
}

// writtenNumber is n as a number in the test log, of the record type `number`.
func writtenNumber(event string, n int) Message {
	return NewMessage(event, n, "number", func(n int) any { return canonical.Integer(n) })
}

func logged(result TestCaseResult, component string, kind LogKind) bool {
	for _, record := range result.Log {
		if record.Component == component && record.Kind == kind {
			return true
		}
	}
	return false
}

// byType is the ids each component type declares, from (type, id) pairs.
func byType(entries ...[2]string) map[string][]string {
	ids := map[string][]string{}
	for _, entry := range entries {
		ids[entry[0]] = append(ids[entry[0]], entry[1])
	}
	return ids
}

// mtcAndEcho is an MTC and one PTC joined by one connection, the MTC's `p` to
// the PTC's `q`.
func mtcAndEcho() Configuration {
	return Configuration{
		MTCType:      "Tester",
		PTCTypes:     map[string]string{"echo": "Echo"},
		PortsOfType:  byType([2]string{"Tester", "p"}, [2]string{"Echo", "q"}),
		TimersOfType: byType([2]string{"Tester", "reply"}),
		Connections:  []Connection{{A: Endpoint{Component: "mtc", Port: "p"}, B: Endpoint{Component: "echo", Port: "q"}}},
	}
}

// mapped is an MTC alone, of the type componentType, whose port `p` is mapped
// to the system port `sut`.
func mapped(componentType string) Configuration {
	return Configuration{
		MTCType:     componentType,
		PortsOfType: byType([2]string{componentType, "p"}),
		Mappings:    []Mapping{{Port: Endpoint{Component: "mtc", Port: "p"}, SystemPort: "sut"}},
	}
}

// lone is an MTC alone, of the type componentType, with no port and no timer.
func lone(componentType string) Configuration {
	return Configuration{MTCType: componentType}
}

// tester is an MTC that creates the echo, sends to it, and waits for the
// answer within a second: a receive branch passes, a timeout branch fails.
func tester(t *testing.T) steps {
	return steps{
		onStart: func(context *ComponentContext, _ *scripted) {
			context.Create("echo")
			context.Send("p", message("ping"))
			context.StartTimer("reply", second)
		},
		onSnapshot: repliesInTime(t),
	}
}

func repliesInTime(t *testing.T) step {
	return func(context *ComponentContext, self *scripted) {
		if context.SnapshotPresent("p") {
			if got := context.Receive("p").Event; got != "pong" {
				t.Errorf("received %q, want pong", got)
			}
			context.StopTimer("reply")
			context.Setverdict(VerdictPass, "")
			self.end()
		} else if context.TimedOut("reply") {
			context.Timeout("reply")
			context.Setverdict(VerdictFail, "no reply in time")
			self.end()
		}
	}
}

func runCase(configuration Configuration, m machines, adapter SystemAdapter, guard Guard) TestCaseResult {
	return runCaseWith(configuration, m, adapter, guard, nil)
}

func runCaseWith(
	configuration Configuration,
	m machines,
	adapter SystemAdapter,
	guard Guard,
	parameters *ModuleParameters,
) TestCaseResult {
	return NewTestCaseRun(configuration, m.factory(), adapter, guard, parameters).Run()
}

// saved is a `uint32` module parameter's value, as a generated suite writes its
// default.
func saved(value uint32) ModuleParameterValue {
	return sce.SavedWriteUint32(value)
}

// slot is what a component read of a module parameter, if it read one.
type slot struct {
	value *uint32
}

// readsJob is a component that reads the module parameters of its run into its
// variables before it starts (§11.5, §11.8), as a generated machine reads each
// through the reader of the type it declares.
func readsJob(job *slot) steps {
	return steps{
		onStart: func(context *ComponentContext, self *scripted) {
			read := context.ReadModuleParameters(func(given *ModuleParameters) error {
				value, err := given.Of("job")
				if err != nil {
					return err
				}
				n, err := sce.SavedReadUint32(value, "job")
				if err != nil {
					return err
				}
				job.value = &n
				return nil
			})
			if read {
				context.Setverdict(VerdictPass, "")
				self.end()
			}
		},
		onSnapshot: idle(),
	}
}

// panicOf is what f panics with, as text, and whether it panicked.
func panicOf(f func()) (message string, panicked bool) {
	defer func() {
		if r := recover(); r != nil {
			message, panicked = fmt.Sprint(r), true
		}
	}()
	f()
	return "", false
}

// -- test_verdict ----------------------------------------------------------

func TestTestVerdict_OnlyEverBecomesWorse(t *testing.T) {
	cases := []struct{ a, b, want Verdict }{
		{VerdictPass, VerdictNone, VerdictPass},
		{VerdictPass, VerdictInconclusive, VerdictInconclusive},
		{VerdictFail, VerdictPass, VerdictFail},
		{VerdictFail, VerdictError, VerdictError},
	}
	for _, c := range cases {
		if got := Worse(c.a, c.b); got != c.want {
			t.Errorf("Worse(%v, %v) = %v, want %v", c.a, c.b, got, c.want)
		}
	}
}

// -- test_port ---------------------------------------------------------------

func TestTestPort_ASnapshotSeesOnlyWhatWasQueuedWhenItWasTaken(t *testing.T) {
	var port Port
	port.TakeSnapshot()
	port.Enqueue(message("late"))
	if _, saw := port.Snapshot(); saw {
		t.Fatal("a message that arrives after the snapshot is not in it")
	}
	port.TakeSnapshot()
	if _, saw := port.Snapshot(); !saw {
		t.Fatal("the second snapshot saw the message")
	}
	port.Enqueue(message("behind"))
	if head, _ := port.Snapshot(); head.Event != "late" {
		t.Fatalf("a later message goes behind the head: %v", head)
	}
	if head := port.RemoveHead(); head.Event != "late" {
		t.Fatalf("removed %v", head)
	}
	if port.Len() != 1 {
		t.Fatalf("the queue holds %d", port.Len())
	}
}

// A message's value is read back by its type, and carries its type's name and
// its canonical form; a message of another type, or one that carries no value,
// is no V.
func TestTestPort_AMessageCarriesItsValueItsTypeAndItsCanonicalForm(t *testing.T) {
	seven := writtenNumber("number", 7)
	if n, ok := ValueAs[int](seven); !ok || n != 7 {
		t.Fatalf("read back %d %v", n, ok)
	}
	if _, ok := ValueAs[string](seven); ok {
		t.Fatal("a number is no text")
	}
	if _, ok := ValueAs[int](Message{Event: "bare"}); ok {
		t.Fatal("a message that carries no value is no int")
	}
	if seven.TypeName() != "number" {
		t.Fatalf("type %q", seven.TypeName())
	}
	if text, err := sce.WriteJCS(seven.CanonicalValue()); err != nil || text != `"7"` {
		t.Fatalf("canonical %q %v", text, err)
	}
}

// -- test_timer --------------------------------------------------------------

func TestTestTimer_TimesOutAtItsInstantAndRestartsWhenStartedAgain(t *testing.T) {
	var timer Timer
	timer.Start(0, 10)
	timer.Update(9)
	if timer.Status() != TimerRunning {
		t.Fatalf("status %v", timer.Status())
	}
	timer.Start(5, 10)
	if at, running := timer.Expiry(); !running || at != 15 {
		t.Fatalf("a running timer that is started again restarts: %d %v", at, running)
	}
	timer.Update(15)
	if timer.Status() != TimerTimeout {
		t.Fatalf("status %v", timer.Status())
	}
	timer.TakeSnapshot()
	if !timer.SnapshotTimedOut() {
		t.Fatal("the snapshot saw the timeout")
	}
	timer.TimeoutTaken()
	if timer.Status() != TimerIdle {
		t.Fatalf("status %v", timer.Status())
	}
	timer.Stop()
	if timer.Status() != TimerIdle {
		t.Fatal("stopping a timer that is not running does nothing")
	}
}

// fourInputs are the SUT's four messages, all at `NOW = 0`: a record of
// another type, another event, another value, and the one the template
// matches.
type fourInputs struct {
	given bool
}

func (*fourInputs) Send(string, Message, Instant) {}

func (*fourInputs) NextInput() (Instant, bool) {
	return 0, false
}

func (f *fourInputs) TakeInputsDue(now Instant) []SystemInput {
	if now != 0 || f.given {
		return nil
	}
	f.given = true
	return []SystemInput{
		{SystemPort: "sut", Message: writtenText("number", "7")},
		{SystemPort: "sut", Message: writtenNumber("other", 7)},
		{SystemPort: "sut", Message: writtenNumber("number", 6)},
		{SystemPort: "sut", Message: writtenNumber("number", 7)},
	}
}

// -- test_case_run_test --------------------------------------------------------

// P6, P9, §3.2: a message sent in a round is taken in a later round, and the
// whole exchange happens at one instant, because a branch fired in each round.
func TestTestCaseRunTest_AnAnsweredRequestPassesWithoutTimeMoving(t *testing.T) {
	m := machines{}.with("Tester", tester(t)).with("Echo", steps{
		onStart: idle(),
		onSnapshot: func(context *ComponentContext, _ *scripted) {
			if context.SnapshotPresent("q") {
				if got := context.Receive("q").Event; got != "ping" {
					t.Errorf("received %q, want ping", got)
				}
				context.Send("q", message("pong"))
			}
		},
	})
	result := runCase(mtcAndEcho(), m, &traceAdapter{}, NoGuard)
	if result.Verdict != VerdictPass {
		t.Fatalf("verdict %v: %v", result.Verdict, result.Log)
	}
	if result.EndedAt != 0 {
		t.Fatalf("ended at %d", result.EndedAt)
	}
	if !logged(result, "echo", LogKill) {
		t.Fatal("the MTC's end removes the PTC (P10)")
	}
}

// P7, §3.3: with nothing to receive, time advances to the timer's instant.
func TestTestCaseRunTest_AnUnansweredRequestFailsWhenItsTimerExpires(t *testing.T) {
	m := machines{}.with("Tester", tester(t)).with("Echo", steps{onStart: idle(), onSnapshot: idle()})
	result := runCase(mtcAndEcho(), m, &traceAdapter{}, NoGuard)
	if result.Verdict != VerdictFail {
		t.Fatalf("verdict %v: %v", result.Verdict, result.Log)
	}
	if result.EndedAt != second {
		t.Fatalf("ended at %d", result.EndedAt)
	}
}

// P13: no branch can fire and no instant is pending, so the test case can
// never end; the runtime ends it with error.
func TestTestCaseRunTest_ATestCaseThatCanNeverEndIsAnError(t *testing.T) {
	m := machines{}.with("Waiter", steps{onStart: idle(), onSnapshot: idle()})
	result := runCase(mapped("Waiter"), m, &traceAdapter{}, NoGuard)
	if result.Verdict != VerdictError {
		t.Fatalf("verdict %v", result.Verdict)
	}
	if !logged(result, "mtc", LogError) {
		t.Fatalf("no error logged: %v", result.Log)
	}
}

// P11: the guard instant ends a test case that has not ended, with error.
func TestTestCaseRunTest_TheGuardEndsATestCaseWithError(t *testing.T) {
	m := machines{}.with("Waiter", steps{
		onStart:    func(context *ComponentContext, _ *scripted) { context.Setverdict(VerdictPass, "") },
		onSnapshot: idle(),
	})
	result := runCase(mapped("Waiter"), m, &traceAdapter{}, GuardOf(5*second))
	if result.Verdict != VerdictError {
		t.Fatalf("verdict %v", result.Verdict)
	}
	if result.EndedAt != 5*second {
		t.Fatalf("ended at %d", result.EndedAt)
	}
}

// §2.2, §12.2: a mapped port's sends reach the adapter, and the trace's inputs
// reach the port at their instants.
func TestTestCaseRunTest_AMappedPortTalksToTheSystemAdapterOnTheLogicalClock(t *testing.T) {
	configuration := mapped("Tester")
	configuration.TimersOfType = byType([2]string{"Tester", "reply"})
	m := machines{}.with("Tester", steps{
		onStart: func(context *ComponentContext, _ *scripted) {
			context.Send("p", message("ping"))
			context.StartTimer("reply", second)
		},
		onSnapshot: repliesInTime(t),
	})
	adapter := &traceAdapter{trace: []input{{at: 500, systemPort: "sut", event: "pong"}}}
	result := runCase(configuration, m, adapter, NoGuard)
	if result.Verdict != VerdictPass {
		t.Fatalf("verdict %v: %v", result.Verdict, result.Log)
	}
	if result.EndedAt != 500 {
		t.Fatalf("ended at %d", result.EndedAt)
	}
	want := []input{{at: 0, systemPort: "sut", event: "ping"}}
	if !reflect.DeepEqual(adapter.sent, want) {
		t.Fatalf("sent %v, want %v", adapter.sent, want)
	}
}

// P9, P14: a removed PTC's verdict still counts in the test case verdict.
func TestTestCaseRunTest_ARemovedPtcsVerdictStillCounts(t *testing.T) {
	m := machines{}.with("Tester", steps{
		onStart: func(context *ComponentContext, _ *scripted) {
			context.Create("echo")
			context.StartTimer("reply", 10)
		},
		onSnapshot: func(context *ComponentContext, self *scripted) {
			if context.TimedOut("reply") {
				context.Timeout("reply")
				context.Kill("echo")
				context.Setverdict(VerdictPass, "")
				self.end()
			}
		},
	}).with("Echo", steps{
		onStart:    func(context *ComponentContext, _ *scripted) { context.Setverdict(VerdictFail, "") },
		onSnapshot: idle(),
	})
	result := runCase(mtcAndEcho(), m, &traceAdapter{}, NoGuard)
	if result.Verdict != VerdictFail {
		t.Fatalf("verdict %v: %v", result.Verdict, result.Log)
	}
}

// §3.2 step 4: components select one at a time in creation order, so when the
// MTC terminates in a round, a PTC that would have selected after it in that
// round does not.
func TestTestCaseRunTest_NothingSelectsAfterTheMtcTerminatesInTheRound(t *testing.T) {
	configuration := mtcAndEcho()
	configuration.TimersOfType["Echo"] = []string{"late"}
	m := machines{}.with("Tester", steps{
		onStart: func(context *ComponentContext, _ *scripted) {
			context.Create("echo")
			context.StartTimer("reply", 10)
		},
		onSnapshot: func(context *ComponentContext, self *scripted) {
			if context.TimedOut("reply") {
				context.Timeout("reply")
				context.Setverdict(VerdictPass, "")
				self.end()
			}
		},
	}).with("Echo", steps{
		onStart: func(context *ComponentContext, _ *scripted) { context.StartTimer("late", 10) },
		onSnapshot: func(context *ComponentContext, _ *scripted) {
			if context.TimedOut("late") {
				context.Timeout("late")
				context.Setverdict(VerdictFail, "")
			}
		},
	})
	result := runCase(configuration, m, &traceAdapter{}, NoGuard)
	if result.Verdict != VerdictPass {
		t.Fatalf("verdict %v: %v", result.Verdict, result.Log)
	}
	if logged(result, "echo", LogTimeout) {
		t.Fatalf("the echo selected after the MTC ended: %v", result.Log)
	}
}

// P2, P10: a PTC that terminates is in DONE from the next snapshot on, and a
// done branch removes nothing.
func TestTestCaseRunTest_ADoneBranchSeesAPtcThatTerminated(t *testing.T) {
	m := machines{}.with("Tester", steps{
		onStart: func(context *ComponentContext, _ *scripted) { context.Create("echo") },
		onSnapshot: func(context *ComponentContext, self *scripted) {
			if context.Done("echo") {
				context.DoneTaken("echo")
				context.Kill("echo")
				self.end()
			}
		},
	}).with("Echo", steps{
		onStart: func(context *ComponentContext, self *scripted) {
			context.Setverdict(VerdictPass, "")
			self.end()
		},
		onSnapshot: idle(),
	})
	result := runCase(mtcAndEcho(), m, &traceAdapter{}, NoGuard)
	if result.Verdict != VerdictPass {
		t.Fatalf("verdict %v: %v", result.Verdict, result.Log)
	}
	if !logged(result, "echo", LogDone) {
		t.Fatalf("no done logged: %v", result.Log)
	}
}

// P2, §12.3: a receive branch with a template matches only a message of its
// record, under its record's event, whose value its `match` holds; the guard
// reads the snapshot and removes nothing.
func TestTestCaseRunTest_AReceiveWithATemplateMatchesOnlyItsRecordsValue(t *testing.T) {
	var seen []string
	isSeven := func(n int) bool { return n == 7 }
	m := machines{}.with("Reader", steps{
		onStart: idle(),
		onSnapshot: func(context *ComponentContext, self *scripted) {
			if Receives(context, "p", "number", isSeven) {
				seen = append(seen, "seven")
				if !Receives(context, "p", "number", isSeven) {
					t.Error("a guard that matched removed the message")
				}
				context.Receive("p")
				self.end()
			} else if context.SnapshotPresent("p") {
				context.WithSnapshot("p", func(head *Message) {
					if head != nil {
						seen = append(seen, head.Event)
					}
				})
				context.Receive("p")
			}
		},
	})
	result := runCase(mapped("Reader"), m, &fourInputs{}, NoGuard)
	want := []string{"number", "other", "number", "seven"}
	if !reflect.DeepEqual(seen, want) {
		t.Fatalf("a value of another type, another event and another value do not match: %v", seen)
	}
	if result.Verdict != VerdictNone {
		t.Fatalf("verdict %v", result.Verdict)
	}
}

// §11.8: the value the run is given is the one the component reads, and the
// component starts on it.
func TestTestCaseRunTest_AComponentReadsTheModuleParameterItsRunIsGiven(t *testing.T) {
	job := &slot{}
	m := machines{}.with("Reader", readsJob(job))
	parameters := &ModuleParameters{}
	parameters.Set("job", saved(7))
	result := runCaseWith(lone("Reader"), m, &traceAdapter{}, NoGuard, parameters)
	if job.value == nil || *job.value != 7 {
		t.Fatalf("read %v", job.value)
	}
	if result.Verdict != VerdictPass {
		t.Fatalf("verdict %v: %v", result.Verdict, result.Log)
	}
}

// P13: a module parameter the run has no value for is a dynamic error, named in
// the log, and the component goes no further than the read.
func TestTestCaseRunTest_AModuleParameterTheRunIsNotGivenIsAnError(t *testing.T) {
	job := &slot{}
	m := machines{}.with("Reader", readsJob(job))
	result := runCase(lone("Reader"), m, &traceAdapter{}, NoGuard)
	if result.Verdict != VerdictError {
		t.Fatalf("verdict %v", result.Verdict)
	}
	if job.value != nil {
		t.Fatalf("read %d", *job.value)
	}
	if len(result.Log) == 0 {
		t.Fatal("the run logged its error")
	}
	last := result.Log[len(result.Log)-1]
	if last.Kind != LogError || !strings.Contains(last.Detail, "'job'") {
		t.Fatalf("last record %v", last)
	}
}

// P13: a value that is not of the type the component declares is refused as a
// restored value of another type is, and not converted.
func TestTestCaseRunTest_AModuleParameterOfAnotherTypeIsAnError(t *testing.T) {
	job := &slot{}
	m := machines{}.with("Reader", readsJob(job))
	parameters := &ModuleParameters{}
	parameters.Set("job", sce.SavedWriteString("seven"))
	result := runCaseWith(lone("Reader"), m, &traceAdapter{}, NoGuard, parameters)
	if result.Verdict != VerdictError {
		t.Fatalf("verdict %v", result.Verdict)
	}
	if job.value != nil {
		t.Fatalf("read %d", *job.value)
	}
	if !logged(result, "mtc", LogError) {
		t.Fatalf("no error logged: %v", result.Log)
	}
}

// P13: a component type the factory of machines makes no machine for ends the
// run with error where a component of it is created.
func TestTestCaseRunTest_AComponentTypeWithNoMachineIsAnError(t *testing.T) {
	none := func(string) ComponentMachine { return nil }
	result := NewTestCaseRun(lone("Ghost"), none, &traceAdapter{}, NoGuard, nil).Run()
	if result.Verdict != VerdictError {
		t.Fatalf("verdict %v", result.Verdict)
	}
	if !logged(result, "mtc", LogError) {
		t.Fatalf("no error logged: %v", result.Log)
	}
	if logged(result, "mtc", LogStart) {
		t.Fatal("no component of the type was created")
	}
}

// P6, P13: a send on a port whose peer is not present is a dynamic error.
func TestTestCaseRunTest_ASendToAnAbsentPeerIsAnError(t *testing.T) {
	m := machines{}.with("Tester", steps{
		onStart:    func(context *ComponentContext, _ *scripted) { context.Send("p", message("ping")) },
		onSnapshot: idle(),
	})
	result := runCase(mtcAndEcho(), m, &traceAdapter{}, NoGuard)
	if result.Verdict != VerdictError {
		t.Fatalf("verdict %v", result.Verdict)
	}
	if result.EndedAt != 0 {
		t.Fatalf("ended at %d", result.EndedAt)
	}
}

// -- test_suite_run ------------------------------------------------------------

// endsWith is a test case whose MTC, of type componentType, sets verdict as it
// starts and ends there.
func endsWith(name, componentType string, verdict Verdict) TestCase {
	m := machines{}.with(componentType, steps{
		onStart: func(context *ComponentContext, self *scripted) {
			context.Setverdict(verdict, "")
			self.end()
		},
		onSnapshot: idle(),
	})
	return TestCase{Name: name, Configuration: lone(componentType), Machines: m.factory()}
}

// tracing is the adapters of a suite's run, each a fresh traceAdapter,
// recording the test case each was built for.
func tracing(built *[]string) func(string) SystemAdapter {
	return func(testCase string) SystemAdapter {
		*built = append(*built, testCase)
		return &traceAdapter{}
	}
}

func selectsX(world Features) bool { return Enabled(world, "x") }

func selectsY(world Features) bool { return Enabled(world, "y") }

func execute(testCase TestCase, guard Guard) Execute {
	return Execute{TestCase: testCase, Guard: guard}
}

// P11: the runner runs the test cases a world selects, in the suite's order,
// and skips the others; a test case with no selection runs in every world.
func TestTestSuiteRun_RunsTheTestCasesAWorldSelectsInTheSuitesOrder(t *testing.T) {
	onX := execute(endsWith("second", "Second", VerdictFail), NoGuard)
	onX.Selection = selectsX
	onY := execute(endsWith("third", "Third", VerdictInconclusive), NoGuard)
	onY.Selection = selectsY
	suite := Suite{
		Name:     "ordered",
		Executes: []Execute{execute(endsWith("first", "First", VerdictPass), NoGuard), onX, onY},
	}
	var built []string
	results, err := RunSuite(suite, FeaturesOf("y"), tracing(&built), nil)
	if err != nil {
		t.Fatal(err)
	}
	if len(results) != 2 {
		t.Fatalf("%d results: %v", len(results), results)
	}
	if results[0].TestCase != "first" || results[0].Result.Verdict != VerdictPass {
		t.Fatalf("first: %v", results[0])
	}
	if results[1].TestCase != "third" || results[1].Result.Verdict != VerdictInconclusive {
		t.Fatalf("second: %v", results[1])
	}
	if !reflect.DeepEqual(built, []string{"first", "third"}) {
		t.Fatalf("an adapter is built for each case run: %v", built)
	}
}

// P11: each test case is a fresh run from NOW = 0 under its own guard.
func TestTestSuiteRun_EachTestCaseIsAFreshRunUnderItsOwnGuard(t *testing.T) {
	waiting := machines{}.with("Waiter", steps{onStart: idle(), onSnapshot: idle()})
	stuck := TestCase{Name: "stuck", Configuration: lone("Waiter"), Machines: waiting.factory()}
	suite := Suite{
		Name: "guarded",
		Executes: []Execute{
			execute(stuck, GuardOf(5*second)),
			execute(endsWith("quick", "Quick", VerdictPass), NoGuard),
		},
	}
	var built []string
	results, err := RunSuite(suite, FeaturesOf(), tracing(&built), nil)
	if err != nil {
		t.Fatal(err)
	}
	if len(results) != 2 {
		t.Fatalf("%d results: %v", len(results), results)
	}
	if results[0].Result.Verdict != VerdictError || results[0].Result.EndedAt != 5*second {
		t.Fatalf("the guard ends the first: %v", results[0])
	}
	if results[1].Result.Verdict != VerdictPass || results[1].Result.EndedAt != 0 {
		t.Fatalf("the second starts again at NOW = 0: %v", results[1])
	}
}

// -1 §8.2.1: a value the caller gives a module parameter replaces the suite's
// default, and a parameter the suite does not declare cannot be given one.
func TestTestSuiteRun_AnOverrideReplacesTheDefaultOfAModuleParameterTheSuiteDeclares(t *testing.T) {
	job := &slot{}
	m := machines{}.with("Reader", readsJob(job))
	defaults := &ModuleParameters{}
	defaults.Set("job", saved(7))
	suite := Suite{
		Name:             "parameters",
		ModuleParameters: defaults,
		Executes: []Execute{execute(TestCase{
			Name:             "reads",
			Configuration:    lone("Reader"),
			Machines:         m.factory(),
			ModuleParameters: []string{"job"},
		}, NoGuard)},
	}
	var built []string
	results, err := RunSuite(suite, FeaturesOf(), tracing(&built), nil)
	if err != nil {
		t.Fatal(err)
	}
	if len(results) != 1 || results[0].Result.Verdict != VerdictPass {
		t.Fatalf("results %v", results)
	}
	if job.value == nil || *job.value != 7 {
		t.Fatalf("the default: %v", job.value)
	}

	overrides := &ModuleParameters{}
	overrides.Set("job", saved(9))
	overridden, err := RunSuite(suite, FeaturesOf(), tracing(&built), overrides)
	if err != nil {
		t.Fatal(err)
	}
	if len(overridden) != 1 || *job.value != 9 {
		t.Fatalf("the override: %v %v", overridden, *job.value)
	}

	stray := &ModuleParameters{}
	stray.Set("limit", saved(1))
	_, err = RunSuite(suite, FeaturesOf(), tracing(&built), stray)
	var refusal *OverrideRefusal
	if !errors.As(err, &refusal) {
		t.Fatalf("an override of an undeclared parameter is refused: %v", err)
	}
	if !strings.Contains(refusal.Reason, "'limit'") {
		t.Fatalf("reason %q", refusal.Reason)
	}
}

// P13: a test case whose components read a module parameter the run has no
// value for ends with error before anything of it runs, its adapter included.
func TestTestSuiteRun_ATestCaseThatReadsAModuleParameterWithNoValueIsAnError(t *testing.T) {
	reads := endsWith("reads", "Reader", VerdictPass)
	reads.ModuleParameters = []string{"job"}
	suite := Suite{Name: "undeclared", Executes: []Execute{execute(reads, NoGuard)}}
	var built []string
	results, err := RunSuite(suite, FeaturesOf(), tracing(&built), nil)
	if err != nil {
		t.Fatal(err)
	}
	if len(results) != 1 || results[0].Result.Verdict != VerdictError {
		t.Fatalf("results %v", results)
	}
	if log := results[0].Result.Log; len(log) != 1 || !strings.Contains(log[0].Detail, "'job'") {
		t.Fatalf("log %v", log)
	}
	if len(built) != 0 {
		t.Fatalf("no adapter is built for a case that cannot start: %v", built)
	}
}

// P13: a test case the caller gives no system adapter ends with error.
func TestTestSuiteRun_ATestCaseWithNoSystemAdapterIsAnError(t *testing.T) {
	suite := Suite{
		Name:     "unadapted",
		Executes: []Execute{execute(endsWith("only", "Only", VerdictPass), NoGuard)},
	}
	results, err := RunSuite(suite, FeaturesOf(), func(string) SystemAdapter { return nil }, nil)
	if err != nil {
		t.Fatal(err)
	}
	if len(results) != 1 || results[0].Result.Verdict != VerdictError {
		t.Fatalf("results %v", results)
	}
}

// -- the_shared_handle -------------------------------------------------------

// A snapshot reader runs while the run's state is held, so a call it makes into
// its context is re-entrant: it panics, naming the component, and does not go
// on inside the state.
func TestTheSharedHandle_ACallIntoTheContextFromASnapshotReaderPanics(t *testing.T) {
	m := machines{}.with("Reader", steps{
		onStart: func(context *ComponentContext, _ *scripted) {
			context.WithSnapshot("p", func(*Message) { context.Log("inside the reader") })
		},
		onSnapshot: idle(),
	})
	got, panicked := panicOf(func() { runCase(mapped("Reader"), m, &traceAdapter{}, NoGuard) })
	if !panicked || got != "re-entrant access to the test run from mtc" {
		t.Fatalf("panicked %v with %q", panicked, got)
	}
}

// P9, P13: a component whose machine is running is not removed under it. The
// PTC's start kills the MTC, whose own start is still running; that is a
// dynamic error, and the MTC stays.
func TestTheSharedHandle_AKillOfAComponentWhoseMachineRunsIsAnError(t *testing.T) {
	m := machines{}.with("Tester", steps{
		onStart: func(context *ComponentContext, _ *scripted) {
			context.Create("echo")
			context.Setverdict(VerdictPass, "")
		},
		onSnapshot: idle(),
	}).with("Echo", steps{
		onStart:    func(context *ComponentContext, _ *scripted) { context.Kill("mtc") },
		onSnapshot: idle(),
	})
	result := runCase(mtcAndEcho(), m, &traceAdapter{}, NoGuard)
	if result.Verdict != VerdictError {
		t.Fatalf("verdict %v", result.Verdict)
	}
	if !logged(result, "echo", LogError) {
		t.Fatalf("no error of the echo: %v", result.Log)
	}
	if logged(result, "mtc", LogKill) {
		t.Fatal("the MTC was not removed")
	}
	if !logged(result, "mtc", LogVerdict) {
		t.Fatal("the MTC's start ran on after the refused kill")
	}
}

// A context names one creation of its component: once the PTC is removed, a
// call through a context it handed out panics instead of reaching another
// component.
func TestTheSharedHandle_AContextDoesNotOutliveItsComponent(t *testing.T) {
	var kept *ComponentContext
	m := machines{}.with("Tester", steps{
		onStart: func(context *ComponentContext, _ *scripted) {
			context.Create("echo")
			context.Kill("echo")
			if kept == nil {
				t.Fatal("the echo kept its context")
			}
			kept.Setverdict(VerdictFail, "after its removal")
		},
		onSnapshot: idle(),
	}).with("Echo", steps{
		onStart:    func(context *ComponentContext, _ *scripted) { kept = context },
		onSnapshot: idle(),
	})
	got, panicked := panicOf(func() { runCase(mtcAndEcho(), m, &traceAdapter{}, NoGuard) })
	if !panicked || got != "the component echo was removed, and its context outlived it" {
		t.Fatalf("panicked %v with %q", panicked, got)
	}
}

// -- the_run_is_run_once -------------------------------------------------------

// A run has read its adapter and holds its components' end, so running it a
// second time is refused rather than run on that state.
func TestTheRunIsRunOnce_ARunThatHasRunIsNotRunAgain(t *testing.T) {
	m := machines{}.with("Tester", steps{
		onStart: func(context *ComponentContext, self *scripted) {
			context.Setverdict(VerdictPass, "")
			self.end()
		},
		onSnapshot: idle(),
	})
	run := NewTestCaseRun(lone("Tester"), m.factory(), &traceAdapter{}, NoGuard, nil)
	if verdict := run.Run().Verdict; verdict != VerdictPass {
		t.Fatalf("verdict %v", verdict)
	}
	got, panicked := panicOf(func() { run.Run() })
	if !panicked || got != "a test case run is run once; build another to run the test case again" {
		t.Fatalf("panicked %v with %q", panicked, got)
	}
}

// -- the_doors -----------------------------------------------------------------

// testPort is the Event I/O Processor whose addresses are ports
// (docs/SCE_TEST_PROFILE.md §11.1).
const testPort = "http://sce.dev/ext/test-port"

// testComponent is the invocation of a PTC (docs/SCE_TEST_PROFILE.md §11.1).
const testComponent = "http://sce.dev/ext/test-component"

// testPortProcessor is a send (P6): the request carries the template's value,
// with its type's name and the writer of its canonical form, which the run
// appends to the connected port, or hands the system adapter.
func testPortProcessor(context *ComponentContext) sce.HostSendHandler {
	return func(request sce.HostSendRequest) []sce.HostSendResponse {
		value, carried := request.Value.(TestValue)
		if !carried {
			panic("a test-port send carries its template's value")
		}
		context.Send(request.Target, Message{Event: request.EventName, Value: value})
		return nil
	}
}

// testComponentInvoker is a PTC (P9): the start creates it, the cancel removes
// it.
func testComponentInvoker(context *ComponentContext) sce.HostInvokeHandler {
	return func(event sce.HostInvokeEvent) *sce.HostInvokeResponse {
		if event.Start != nil {
			context.Create(event.Start.InvokeID)
		}
		if event.Cancel != nil {
			context.Kill(event.Cancel.InvokeID)
		}
		return nil
	}
}

// ping is the record a template of the port type reads.
type ping struct {
	n int
}

// pingCanonical is a ping in the canonical form of the test log, as the unit
// of a template over its record writes it.
func pingCanonical(p ping) any {
	return canonical.Record(canonical.Field("n", canonical.Integer(p.n)))
}

// doorsTester is an MTC wired as a generated one is: its start registers the
// two processors, each holding its context, and it reaches the doors of the
// run only through them. Entering its state invokes `echo` and sends it a
// ping; a timeout leaves the state, which cancels the invoke.
type doorsTester struct {
	invoker sce.HostInvokeHandler
	context *ComponentContext
	ended   bool
}

func (d *doorsTester) Start(context *ComponentContext) {
	sends := testPortProcessor(context)
	invokes := testComponentInvoker(context)
	invokes(sce.HostInvokeEvent{Start: &sce.HostInvokeRequest{ProcessorType: testComponent, InvokeID: "echo"}})
	sends(sce.HostSendRequest{
		ProcessorType: testPort,
		EventName:     "ping",
		Target:        "p",
		Value:         MessageValue(ping{n: 3}, "ping", pingCanonical),
	})
	context.StartTimer("reply", 10)
	d.invoker = invokes
	d.context = context
}

func (d *doorsTester) DeliverSnapshot() {
	if d.context == nil {
		panic("a snapshot is delivered only to a started machine")
	}
	if !d.context.TimedOut("reply") {
		return
	}
	d.context.Timeout("reply")
	d.invoker(sce.HostInvokeEvent{Cancel: &sce.HostInvokeCancel{ProcessorType: testComponent, InvokeID: "echo"}})
	d.context.Setverdict(VerdictPass, "")
	d.ended = true
}

func (d *doorsTester) Terminated() bool {
	return d.ended
}

func position(t *testing.T, result TestCaseResult, component string, kind LogKind) int {
	t.Helper()
	for at, record := range result.Log {
		if record.Component == component && record.Kind == kind {
			return at
		}
	}
	t.Fatalf("no %v of %s in %v", kind, component, result.Log)
	return -1
}

// §12.3: the PTC the invoker's start creates receives the value the
// processor's send carried, by its type, and the invoker's cancel removes it at
// the timeout, before the MTC's verdict.
func TestTheDoors_ASendAndAPtcGoThroughTheHostProcessors(t *testing.T) {
	echo := func(context *ComponentContext, _ *scripted) {
		if Receives(context, "q", "ping", func(p ping) bool { return p.n == 3 }) {
			context.Receive("q")
			context.Setverdict(VerdictPass, "the value arrived")
		} else if context.SnapshotPresent("q") {
			context.Receive("q")
			context.Setverdict(VerdictFail, "another value arrived")
		}
	}
	factory := func(componentType string) ComponentMachine {
		switch componentType {
		case "Tester":
			return &doorsTester{}
		case "Echo":
			return &scripted{steps: steps{onStart: idle(), onSnapshot: echo}}
		}
		return nil
	}
	result := NewTestCaseRun(mtcAndEcho(), factory, &traceAdapter{}, NoGuard, nil).Run()
	if result.Verdict != VerdictPass {
		t.Fatalf("verdict %v: %v", result.Verdict, result.Log)
	}
	created := position(t, result, "echo", LogCreate)
	sent := position(t, result, "mtc", LogSend)
	received := position(t, result, "echo", LogReceive)
	killed := position(t, result, "echo", LogKill)
	verdict := position(t, result, "mtc", LogVerdict)
	if !(created < sent && sent < received) {
		t.Fatalf("order: %v", result.Log)
	}
	if result.Log[sent].Detail != "p ping" || result.Log[received].Detail != "q ping" {
		t.Fatalf("details: %v", result.Log)
	}
	if result.Log[killed].At != 10 {
		t.Fatalf("the cancel came at the timeout: %v", result.Log[killed])
	}
	if killed >= verdict {
		t.Fatal("the cancel removed the PTC, not the end")
	}
	if !logged(result, "echo", LogVerdict) {
		t.Fatalf("the echo set no verdict: %v", result.Log)
	}
	// §12.5: the request carried the value with its type's name and the
	// writer of its canonical form, which the test log records.
	want := `{"at":"0","component":"mtc","kind":"message-sent","port":"p","type":"ping","value":{"n":"3"}}`
	if !tracedAs(t, result, want) {
		t.Fatalf("the test log holds no %s: %s", want, result.CanonicalLog("doors", "doors"))
	}
}

// tracedAs reports whether a record of the test log of result is written as
// want.
func tracedAs(t *testing.T, result TestCaseResult, want string) bool {
	t.Helper()
	for _, record := range result.Records {
		text, err := sce.WriteJCS(record.ToValue())
		if err != nil {
			t.Fatalf("a record is canonical: %v", err)
		}
		if text == want {
			return true
		}
	}
	return false
}

// -- the_test_log ----------------------------------------------------------------

// pinging is an MTC and its echo, joined `p` to `q`; the MTC has two timers.
func pinging() Configuration {
	return Configuration{
		MTCType:      "Tester",
		PTCTypes:     map[string]string{"echo": "Echo"},
		PortsOfType:  byType([2]string{"Tester", "p"}, [2]string{"Echo", "q"}),
		TimersOfType: byType([2]string{"Tester", "t"}, [2]string{"Tester", "s"}),
		Connections:  []Connection{{A: Endpoint{Component: "mtc", Port: "p"}, B: Endpoint{Component: "echo", Port: "q"}}},
	}
}

// pingOf is the ping's value, a record of one number of the record type
// `ping`, as the unit of a template over it writes it.
func pingOf(n uint32) Message {
	return NewMessage("ping", n, "ping", func(n uint32) any {
		return canonical.Record(canonical.Field("n", canonical.Integer(n)))
	})
}

// pingingMachines are the MTC, which creates the echo, pings it, starts two
// timers and logs, and at the first timer's timeout stops the second, and the
// first again, which no longer runs, and passes; and the echo, which takes the
// ping, passes with a reason, and terminates.
func pingingMachines() machines {
	return machines{}.with("Tester", steps{
		onStart: func(context *ComponentContext, _ *scripted) {
			context.Create("echo")
			context.Send("p", pingOf(3))
			context.StartTimer("t", 10)
			context.StartTimer("s", 20)
			context.Log(`asked "echo"`)
		},
		onSnapshot: func(context *ComponentContext, self *scripted) {
			if context.TimedOut("t") {
				context.Timeout("t")
				context.StopTimer("s")
				context.StopTimer("t")
				context.Setverdict(VerdictPass, "")
				self.end()
			}
		},
	}).with("Echo", steps{
		onStart: idle(),
		onSnapshot: func(context *ComponentContext, self *scripted) {
			if context.SnapshotPresent("q") {
				context.Receive("q")
				context.Setverdict(VerdictPass, "pinged")
				self.end()
			}
		},
	})
}

// loggedSuite is the suite of testCase alone.
func loggedSuite(testCase TestCase) Suite {
	return Suite{Name: "logged_suite", Executes: []Execute{execute(testCase, NoGuard)}}
}

// expectLog holds that got is the log whose parts, joined, are want.
func expectLog(t *testing.T, got string, want ...string) {
	t.Helper()
	if joined := strings.Join(want, ""); got != joined {
		t.Fatalf("the test log is\n%s\nnot\n%s", got, joined)
	}
}

// P4, P6, P7, P8, P9, P10, P12, P14: the whole log of a run through the suite's
// runner. The echo's work comes in the round after the MTC's start, at
// `NOW = 0`, the timeout at 10; the stop of a timer that no longer runs, and
// the removal of the terminated echo when the MTC ends, write no record.
func TestTheTestLog_ARunWritesEachRecordWhereItsEventHappens(t *testing.T) {
	suite := loggedSuite(TestCase{Name: "pings_the_echo", Configuration: pinging(), Machines: pingingMachines().factory()})
	results, err := RunSuite(suite, FeaturesOf(), func(string) SystemAdapter { return &traceAdapter{} }, nil)
	if err != nil || len(results) != 1 {
		t.Fatalf("the suite runs its one case: %v %v", results, err)
	}
	expectLog(t, results[0].CanonicalLog(suite.Name),
		`{"records":[`,
		`{"at":"0","kind":"test-case-started"},`,
		`{"at":"0","component":"mtc","kind":"component-created","type":"Tester"},`,
		`{"at":"0","component":"echo","kind":"component-created","type":"Echo"},`,
		`{"at":"0","component":"mtc","kind":"message-sent","port":"p",`,
		`"type":"ping","value":{"n":"3"}},`,
		`{"at":"0","component":"mtc","duration_us":"10","kind":"timer-started",`,
		`"timer":"t"},`,
		`{"at":"0","component":"mtc","duration_us":"20","kind":"timer-started",`,
		`"timer":"s"},`,
		`{"at":"0","component":"mtc","kind":"log","text":"asked \"echo\""},`,
		`{"at":"0","component":"echo","kind":"message-received","port":"q",`,
		`"type":"ping","value":{"n":"3"}},`,
		`{"at":"0","component":"echo","kind":"verdict-set","reason":"pinged",`,
		`"verdict":"pass"},`,
		`{"at":"0","component":"echo","kind":"component-done","verdict":"pass"},`,
		`{"at":"10","component":"mtc","kind":"timer-timed-out","timer":"t"},`,
		`{"at":"10","component":"mtc","kind":"timer-stopped","timer":"s"},`,
		`{"at":"10","component":"mtc","kind":"verdict-set","verdict":"pass"},`,
		`{"at":"10","component":"mtc","kind":"component-done","verdict":"pass"},`,
		`{"at":"10","kind":"test-case-ended","verdict":"pass"}`,
		`],"schema":"sce-test-log.v1","suite":"logged_suite",`,
		`"test_case":"pings_the_echo","verdict":"pass"}`,
	)
}

// P13: an error the runtime sets is a verdict without the runtime's own
// reason, which another runtime would not write alike.
func TestTheTestLog_AnErrorTheRuntimeSetsIsLoggedWithoutItsCause(t *testing.T) {
	m := machines{}.with("Waiter", steps{onStart: idle(), onSnapshot: idle()})
	result := runCase(lone("Waiter"), m, &traceAdapter{}, GuardOf(7))
	if result.Verdict != VerdictError {
		t.Fatalf("verdict %v: %v", result.Verdict, result.Log)
	}
	expectLog(t, result.CanonicalLog("logged_suite", "waits"),
		`{"records":[`,
		`{"at":"0","kind":"test-case-started"},`,
		`{"at":"0","component":"mtc","kind":"component-created","type":"Waiter"},`,
		`{"at":"7","component":"mtc","kind":"verdict-set","verdict":"error"},`,
		`{"at":"7","kind":"test-case-ended","verdict":"error"}`,
		`],"schema":"sce-test-log.v1","suite":"logged_suite",`,
		`"test_case":"waits","verdict":"error"}`,
	)
}

// P13: a test case that cannot start, for want of an adapter, logs its start
// and its end alone, since no component was created.
func TestTheTestLog_ATestCaseThatCannotStartLogsItsStartAndEnd(t *testing.T) {
	suite := loggedSuite(TestCase{Name: "unplayed", Configuration: pinging(), Machines: pingingMachines().factory()})
	results, err := RunSuite(suite, FeaturesOf(), func(string) SystemAdapter { return nil }, nil)
	if err != nil || len(results) != 1 {
		t.Fatalf("the suite reports its one case: %v %v", results, err)
	}
	expectLog(t, results[0].CanonicalLog(suite.Name),
		`{"records":[`,
		`{"at":"0","kind":"test-case-started"},`,
		`{"at":"0","kind":"test-case-ended","verdict":"error"}`,
		`],"schema":"sce-test-log.v1","suite":"logged_suite",`,
		`"test_case":"unplayed","verdict":"error"}`,
	)
}
