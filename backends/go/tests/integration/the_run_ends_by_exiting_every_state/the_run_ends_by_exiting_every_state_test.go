// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// W3C SCXML Appendix D exitInterpreter: a run ends by exiting every state it
// is still in, the way exitStates exits one — Go AOT path.
//
// Reached two ways, and both are driven here: the run enters a top-level
// <final> after a step, or the host stops it. Measured 2026-09-26, this
// channel ran the final's <onexit> only when the run ended during Initialize
// with a completion callback set, kept the final in the configuration
// afterwards, and ran no <onexit> at all on Stop.
//
// Fixture: integration_resources/the_run_ends_by_exiting_every_state/the_run_ends_by_exiting_every_state.scxml
// (canonical, shared with the C++ / C11 / Rust / Kotlin / Python channels).
//
// Regeneration: scripts/regen_the_run_ends_by_exiting_every_state_go.sh

package the_run_ends_by_exiting_every_state

import (
	"fmt"
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
)

type runEngine = sce.Engine[TheRunEndsByExitingEveryStateState, TheRunEndsByExitingEveryStateEvent]

func started(t *testing.T) (*runEngine, *TheRunEndsByExitingEveryStatePolicy) {
	t.Helper()
	policy := NewTheRunEndsByExitingEveryStatePolicy()
	policy.SessionID = sce.GenerateSessionID()
	// The handlers record with <assign>, so this is an ECMAScript-datamodel
	// machine.
	policy.ScriptEngine = scegotest.NewLuaEngine()
	engine := sce.NewEngine[TheRunEndsByExitingEveryStateState, TheRunEndsByExitingEveryStateEvent](&policy)
	engine.Initialize()
	entry := engine.GetActiveStates()
	inner := false
	for _, s := range entry {
		if s == TheRunEndsByExitingEveryStateStateInner {
			inner = true
		}
	}
	if !inner {
		t.Fatalf("fixture came up as %v; the run has to start inside `inner`", entry)
	}
	return engine, &policy
}

// record reads one value the handlers wrote (W3C SCXML 5.3 readers).
func record(v int64, ok bool) string {
	if !ok {
		return "<unreadable>"
	}
	return fmt.Sprint(v)
}

// describe prints what the run left behind, so a failure says what happened
// and not only which clause broke.
func describe(engine *runEngine, policy *TheRunEndsByExitingEveryStatePolicy) string {
	ended, ok := engine.TerminalState()
	endedIn := "<none>"
	if ok {
		endedIn = fmt.Sprint(ended)
	}
	return fmt.Sprintf("active: %v, ended in %s, running=%v, order=%s finalExits=%s selfInFinal=%s",
		engine.GetActiveStates(), endedIn, engine.IsRunning(),
		record(policy.Order()), record(policy.FinalExits()), record(policy.SelfInFinal()))
}

// expectRecord fails unless a handler record holds the value wanted.
func expectRecord(t *testing.T, name string, want int64, v int64, ok bool, why, seen string) {
	t.Helper()
	if !ok || v != want {
		t.Errorf("%s = %s, want %d: %s (%s)", name, record(v, ok), want, why, seen)
	}
}

// The run ends in a top-level <final> after a step: the final is exited too.
func TestARunThatReachesItsFinalExitsTheFinal(t *testing.T) {
	engine, policy := started(t)
	engine.RaiseExternal(TheRunEndsByExitingEveryStateEventFinish, "", "")
	engine.Step()

	seen := describe(engine, policy)
	if ended, ok := engine.TerminalState(); !ok || ended != TheRunEndsByExitingEveryStateStateDone {
		t.Errorf("the run did not end in `done` (%s)", seen)
	}
	if engine.IsRunning() {
		t.Errorf("a run that ended is still running (%s)", seen)
	}
	if n := len(engine.GetActiveStates()); n != 0 {
		t.Errorf("exitInterpreter deletes every state it exits, the final included; %d remain (%s)", n, seen)
	}
	v, ok := policy.FinalExits()
	expectRecord(t, "finalExits", 1, v, ok, "the final's own <onexit> must run exactly once as the run ends", seen)
	v, ok = policy.SelfInFinal()
	expectRecord(t, "selfInFinal", 1, v, ok, "`done` must still be in the configuration during its own <onexit>", seen)
	v, ok = policy.Order()
	expectRecord(t, "order", 12, v, ok, "`finish` exits `inner` then `outer`", seen)
}

// The host stops a run that has not ended: every state is exited, innermost first.
func TestAStoppedRunExitsEveryStateInnermostFirst(t *testing.T) {
	engine, policy := started(t)
	engine.Stop()

	seen := describe(engine, policy)
	if _, ok := engine.TerminalState(); ok {
		t.Errorf("a stopped run did not end in a final (%s)", seen)
	}
	if engine.IsRunning() {
		t.Errorf("a stopped run is still running (%s)", seen)
	}
	if n := len(engine.GetActiveStates()); n != 0 {
		t.Errorf("stop() must leave the configuration empty; %d remain (%s)", n, seen)
	}
	v, ok := policy.Order()
	expectRecord(t, "order", 12, v, ok, "stop() must run `inner`'s <onexit> and then `outer`'s: "+
		"0 is a stop that exited nothing, 21 one that exited in document order instead of exit order", seen)
	v, ok = policy.FinalExits()
	expectRecord(t, "finalExits", 0, v, ok, "the run never entered `done`", seen)
}
