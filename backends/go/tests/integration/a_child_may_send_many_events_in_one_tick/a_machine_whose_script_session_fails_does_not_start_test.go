// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// docs/adr/0096 — Go AOT path: a generated machine whose script session
// cannot be established does not start, and its host is told why.
//
// The script engine is a double over the real Lua engine. It fails, on
// request, one of the three steps that establish a session — creating it,
// loading the shared library into it, binding the system variables — and
// counts every call that runs a document's expressions, so "no executable
// content ran" is read from the engine the machine runs its content in.
//
// Fixture: integration_resources/a_child_may_send_many_events_in_one_tick/a_child_may_send_many_events_in_one_tick.scxml,
// reused for its shape: a parent in `s0` with `<data ticks>` that invokes an
// inline child, whose `<onentry>` sends `tick` to the parent 110 times. The
// child declares `<data>` of its own, so it has a script session of its own to
// fail. A start that ran would initialize `ticks`, enter `s0` and start the
// child, asking the engine for the child's session; a child that ran would
// send `tick`.
//
// Regeneration: scripts/regen_a_child_may_send_many_events_in_one_tick_go.sh

package a_child_may_send_many_events_in_one_tick

import (
	"errors"
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
)

// failStep is which step of establishing a session the double fails.
type failStep int

const (
	// failNever: the double is the Lua engine, counted.
	failNever failStep = iota
	// failCreate: every session's creation fails as the engine's own failure.
	failCreate
	// failLibrary: every session's creation fails as a shared library that
	// did not load.
	failLibrary
	// failSystemVariables: every bind of the system variables fails.
	failSystemVariables
	// failCreateAfterFirst: the first session asked for, the parent's, is
	// created; every later one, a child's, fails to be.
	failCreateAfterFirst
)

func (f failStep) String() string {
	switch f {
	case failNever:
		return "never"
	case failCreate:
		return "create"
	case failLibrary:
		return "library"
	case failSystemVariables:
		return "system-variables"
	case failCreateAfterFirst:
		return "create-after-first"
	}
	return "unknown"
}

// sessionDouble is the Lua engine, with one step of establishing a session
// failing on request and every call that runs the document's expressions
// counted. The methods it does not name are the Lua engine's, unchanged.
type sessionDouble struct {
	sce.IScriptEngine
	fail failStep
	// asked is every session id CreateSession was asked for, in order.
	asked []string
	// destroyed is every session id DestroySession was asked for, in order.
	destroyed []string
	// ran counts the calls that run a document's expressions or write its
	// datamodel.
	ran int
}

func (d *sessionDouble) CreateSession(sessionID string) error {
	d.asked = append(d.asked, sessionID)
	switch {
	case d.fail == failCreate:
		return errors.New("the double creates no session")
	case d.fail == failLibrary:
		return &sce.ScriptLibraryNotLoaded{Library: "ECMAScript semantics", Err: errors.New("the double loads no library")}
	case d.fail == failCreateAfterFirst && len(d.asked) > 1:
		return errors.New("the double creates no second session")
	}
	return d.IScriptEngine.CreateSession(sessionID)
}

func (d *sessionDouble) DestroySession(sessionID string) {
	d.destroyed = append(d.destroyed, sessionID)
	d.IScriptEngine.DestroySession(sessionID)
}

func (d *sessionDouble) SetupSystemVariables(sessionID, sessionName string, ioProcessors []sce.IoProcessorDescriptor) error {
	if d.fail == failSystemVariables {
		return errors.New("the double binds no system variable")
	}
	return d.IScriptEngine.SetupSystemVariables(sessionID, sessionName, ioProcessors)
}

func (d *sessionDouble) ExecuteScript(sessionID, script string) error {
	d.ran++
	return d.IScriptEngine.ExecuteScript(sessionID, script)
}

func (d *sessionDouble) EvaluateExpression(sessionID, expr string) (interface{}, error) {
	d.ran++
	return d.IScriptEngine.EvaluateExpression(sessionID, expr)
}

func (d *sessionDouble) SetVariable(sessionID, name string, value interface{}) error {
	d.ran++
	return d.IScriptEngine.SetVariable(sessionID, name, value)
}

func (d *sessionDouble) SetVariableAsDOM(sessionID, name, xmlContent string) error {
	d.ran++
	return d.IScriptEngine.SetVariableAsDOM(sessionID, name, xmlContent)
}

func (d *sessionDouble) SetCurrentEvent(sessionID string, args sce.SetCurrentEventArgs) (sce.PayloadReading, error) {
	d.ran++
	return d.IScriptEngine.SetCurrentEvent(sessionID, args)
}

type doubledEngine = sce.Engine[AChildMaySendManyEventsInOneTickState, AChildMaySendManyEventsInOneTickEvent]

// overDouble is the fixture's machine over a double that fails as fail says,
// its policy, and the double, to read afterwards.
func overDouble(fail failStep) (*doubledEngine, *AChildMaySendManyEventsInOneTickPolicy, *sessionDouble) {
	double := &sessionDouble{IScriptEngine: scegotest.NewLuaEngine(), fail: fail}
	policy := NewAChildMaySendManyEventsInOneTickPolicy()
	policy.SessionID = sce.GenerateSessionID()
	policy.ScriptEngine = double
	engine := sce.NewEngine[AChildMaySendManyEventsInOneTickState, AChildMaySendManyEventsInOneTickEvent](&policy)
	return engine, &policy, double
}

// assertNothingStarted holds that a refused start left nothing behind: not
// running, no final state reached, no content run, no session kept, and no
// state entered — entering `s0` would have started its child and asked the
// engine for the child's session, so only the machine's own was asked for.
func assertNothingStarted(t *testing.T, engine *doubledEngine, policy *AChildMaySendManyEventsInOneTickPolicy, double *sessionDouble, fail failStep) {
	t.Helper()
	if engine.IsRunning() {
		t.Errorf("%s: a refused start leaves the machine not running", fail)
	}
	if ended, ok := engine.TerminalState(); ok {
		t.Errorf("%s: a refused start reaches no final state, and it reached %v", fail, ended)
	}
	if double.ran != 0 {
		t.Errorf("%s: a refused start evaluates nothing (no <data>, no <onentry>, no cond), and %d calls ran", fail, double.ran)
	}
	if double.HasSession(policy.SessionID) {
		t.Errorf("%s: a refused start keeps no session", fail)
	}
	for _, asked := range double.asked {
		if asked != policy.SessionID {
			t.Errorf("%s: a refused start enters no state, and the child session %q was asked for", fail, asked)
		}
	}
}

// failureOf is the *sce.ScriptSessionFailure err carries, failing the test
// when it carries none.
func failureOf(t *testing.T, err error, fail failStep) *sce.ScriptSessionFailure {
	t.Helper()
	var failure *sce.ScriptSessionFailure
	if !errors.As(err, &failure) {
		t.Fatalf("%s: the refusal %v carries no script session failure", fail, err)
	}
	return failure
}

func TestAStartOverTheDoubleThatFailsNothingRunsTheFixture(t *testing.T) {
	// The control: the double fails nothing, so the machine is the fixture's,
	// and its child's replies arrive. Without it the cases below could pass
	// over a double that breaks every machine.
	engine, policy, double := overDouble(failNever)
	if err := engine.Initialize(); err != nil {
		t.Fatalf("a session that is established starts the machine: %v", err)
	}
	engine.RaiseExternal(AChildMaySendManyEventsInOneTickEventFinish, "")
	engine.Step()
	if ended, ok := engine.TerminalState(); !ok || ended != AChildMaySendManyEventsInOneTickStateDone {
		t.Errorf("`finish` carries the run to `done`, and it ended in %v (%v)", ended, ok)
	}
	if ticks, ok := policy.Ticks(); !ok || ticks != 110 {
		t.Errorf("every tick the child sent arrived: ticks=%d (%v)", ticks, ok)
	}
	if double.ran == 0 {
		t.Error("the machine ran its content through the double")
	}
}

func TestAStartWhoseSessionCannotBeEstablishedIsRefusedWithTheStep(t *testing.T) {
	for _, round := range []struct {
		fail  failStep
		stage sce.ScriptSessionStage
	}{
		{failCreate, sce.ScriptSessionStageCreate},
		{failLibrary, sce.ScriptSessionStageLibrary},
		{failSystemVariables, sce.ScriptSessionStageSystemVariables},
	} {
		engine, policy, double := overDouble(round.fail)
		err := engine.Initialize()
		if err == nil {
			t.Fatalf("%s: a session that cannot be established refuses the start", round.fail)
		}
		if failure := failureOf(t, err, round.fail); failure.Stage != round.stage {
			t.Errorf("%s: the refusal names the step %s, not %s", round.fail, failure.Stage, round.stage)
		}
		assertNothingStarted(t, engine, policy, double, round.fail)
	}
}

func TestARefusedStartIsRefusedAgainRatherThanRunWithoutASession(t *testing.T) {
	// A session that failed is not marked established: the next start asks
	// for it again and is refused again, rather than finding it "done" and
	// running the machine over no session.
	for _, fail := range []failStep{failCreate, failLibrary, failSystemVariables} {
		engine, policy, double := overDouble(fail)
		if engine.Initialize() == nil {
			t.Fatalf("%s: the first start is refused", fail)
		}
		if engine.Initialize() == nil {
			t.Fatalf("%s: a second start over the same failing engine is refused too", fail)
		}
		if len(double.asked) != 2 {
			t.Errorf("%s: each start asks for the session once, and %d were asked for", fail, len(double.asked))
		}
		assertNothingStarted(t, engine, policy, double, fail)
	}
}

func TestASessionWhoseSystemVariablesCannotBeBoundIsReleased(t *testing.T) {
	engine, policy, double := overDouble(failSystemVariables)
	if engine.Initialize() == nil {
		t.Fatal("a session whose system variables cannot be bound refuses the start")
	}
	if len(double.asked) != 1 || double.asked[0] != policy.SessionID {
		t.Errorf("the machine asked for its own session once, and asked for %v", double.asked)
	}
	if len(double.destroyed) != 1 || double.destroyed[0] != policy.SessionID {
		t.Errorf("the session the failed bind was made in is destroyed, and %v were", double.destroyed)
	}
}

func TestARefusedMachineTakesNoEvent(t *testing.T) {
	engine, policy, double := overDouble(failCreate)
	if engine.Initialize() == nil {
		t.Fatal("a session that cannot be created refuses the start")
	}
	engine.ProcessEvent(AChildMaySendManyEventsInOneTickEventFinish)
	if unseen := engine.UnseenExternalEvents(); unseen != 1 {
		t.Errorf("the event is refused as one posted to a machine that is not running, and %d were", unseen)
	}
	assertNothingStarted(t, engine, policy, double, failCreate)
}

func TestTheCheckedRootStartIsRefusedForTheSession(t *testing.T) {
	engine, policy, double := overDouble(failLibrary)
	err := engine.InitializeAsRoot()
	if !errors.Is(err, sce.RootStartScriptSession) {
		t.Fatalf("the root start answered %v, not a refusal for its session", err)
	}
	if errors.Is(err, sce.RootStartNeedsParent) {
		t.Errorf("a refusal for the session is not one for a parent: %v", err)
	}
	if failure := failureOf(t, err, failLibrary); failure.Stage != sce.ScriptSessionStageLibrary {
		t.Errorf("the refusal names the step %s, not library", failure.Stage)
	}
	assertNothingStarted(t, engine, policy, double, failLibrary)
}

func TestAResumedStartWhoseSessionCannotBeEstablishedIsRefused(t *testing.T) {
	engine, policy, double := overDouble(failCreate)
	verdict := engine.EnterAt(
		[]AChildMaySendManyEventsInOneTickState{AChildMaySendManyEventsInOneTickStateS0},
		AChildMaySendManyEventsInOneTickStateS0,
	)
	if verdict.Rejection != sce.ConfigurationScriptSessionUnavailable {
		t.Fatalf("EnterAt answered %s, not a refusal for its session", verdict)
	}
	if verdict.ScriptSessionFailure == nil || verdict.ScriptSessionFailure.Stage != sce.ScriptSessionStageCreate {
		t.Errorf("the refusal carries the failure of the create step: %+v", verdict.ScriptSessionFailure)
	}
	assertNothingStarted(t, engine, policy, double, failCreate)
}

func TestAChildWhoseSessionFailsLeavesItsParentWithErrorExecutionAndNoChild(t *testing.T) {
	engine, policy, double := overDouble(failCreateAfterFirst)
	if err := engine.Initialize(); err != nil {
		t.Fatalf("the parent's own session is established: %v", err)
	}
	engine.Step()

	if len(double.asked) != 2 {
		t.Fatalf("the parent asked for its own session and the child's: %v", double.asked)
	}
	child := double.asked[1]
	if double.HasSession(child) {
		t.Error("the child whose session failed keeps none")
	}
	if policy.IsChildSessionRunning(child) {
		t.Error("the child whose start was refused is not kept as a session of the parent")
	}
	// The invoke ends without a child, with error.execution, which no
	// transition of `s0` takes.
	if last, ok := engine.LastUnhandledError(); !ok || last != AChildMaySendManyEventsInOneTickEventErrorExecution {
		t.Errorf("the refused child is reported as error.execution, and the last unhandled error is %v (%v)", last, ok)
	}
	if count := engine.UnhandledErrorEvents(); count != 1 {
		t.Errorf("one error.execution for the one refused child, and %d were raised", count)
	}
	// No child ran, so no `tick` and no `done.invoke` ever arrived.
	if ticks, ok := policy.Ticks(); !ok || ticks != 0 {
		t.Errorf("no child ran, so no tick arrived: ticks=%d (%v)", ticks, ok)
	}
	if discarded := engine.DiscardedExternalEvents(); discarded != 0 {
		t.Errorf("no done.invoke was raised for the refused child, and %d events were discarded", discarded)
	}

	// The parent itself runs on.
	if !engine.IsRunning() {
		t.Fatal("the parent runs on without its child")
	}
	if current := engine.GetCurrentState(); current != AChildMaySendManyEventsInOneTickStateS0 {
		t.Errorf("the parent stays in `s0`, and is in %v", current)
	}
	engine.RaiseExternal(AChildMaySendManyEventsInOneTickEventFinish, "")
	engine.Step()
	if ended, ok := engine.TerminalState(); !ok || ended != AChildMaySendManyEventsInOneTickStateDone {
		t.Errorf("`finish` carries the parent to `done`, and it ended in %v (%v)", ended, ok)
	}
}
