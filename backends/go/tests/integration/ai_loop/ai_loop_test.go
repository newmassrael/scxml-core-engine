// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The AI supervision loop, driven through the Go AOT engine.
//
// `examples/ai_loop/ai_loop.scxml` is a worked example: a statechart that
// supervises a long-running session, with <parallel> splitting the turn cycle
// from the liveness watch and the turn budget. `tests/integration/
// AiLoopAotTest.cpp` drives it through the C++ AOT engine and
// `backends/rust/tests/tests/ai_loop.rs` through the Rust one; this file is
// the third channel asking the same document the same questions.
//
// Why a third: a clause asserted in one channel is that engine's word for the
// document rather than the document's own, and the parallel defect that
// shipped in `1419a050ed` (a self-transition whose exit set swallowed the
// parallel root) was invisible to every W3C fixture because they are all one
// region deep. This document is three. `sce-build/tests/
// ai_loop_channel_parity.rs` holds every registered channel to the same
// scenario set by name, so a scenario added here without its siblings fails
// there — which is the moment it is cheapest to fix.
//
// No sprag, no session, no pane: every effect the host would perform is
// replaced by the event that effect would have produced, so what is under test
// is the machine's topology rather than any driver's plumbing.
//
// Because the regions are orthogonal, a scenario asserts on the ACTIVE SET
// rather than on one state — "the cycle is working AND the budget is within"
// is the kind of claim a parallel machine makes, and asserting a single
// current state cannot express it.
//
// Fixture: examples/ai_loop/ai_loop.scxml
//
// Regeneration (after example or template edit):
//   scripts/regen_ai_loop_go.sh

package ai_loop

import (
	"strings"
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
)

// The processor type the tree beside this file was generated for.
// `scripts/regen_ai_loop_go.sh` passes this same string to `--host-processor`,
// and the C++ and Rust channels register the same one.
const declaredType = "x-sce-host"

// loop is an engine and the policy it was built over. The policy is where the
// datamodel accessors live, so scenarios that read what the machine holds need
// both halves.
type loop struct {
	engine *sce.Engine[AiLoopState, AiLoopEvent]
	policy *AiLoopPolicy
}

// notInitialised wires a machine the way every scenario below wires one, and
// stops short of booting it.
//
// The handler is registered before Initialize because `priming` performs its
// act on entry: a machine booted without one raises `error.execution` there
// instead of reaching a host.
//
// W3C SCXML 6.2.5: the document declares its acts as sends a host serves, so
// one has to be registered or the first act reaches nobody. This one performs
// nothing and reports nothing, which is deliberate — what these scenarios
// measure is the TOPOLOGY, and each supplies the events a host would have
// produced at exactly the point it wants them. A handler that answered would
// deliver the same events a second time.
//
// `examples/ai_loop/ai_loop_example.cpp` registers the real one, and
// TestTheDocumentDeclaresItsActsToTheHost below registers a recording one.
func notInitialised(handler sce.HostSendHandler) loop {
	policy := NewAiLoopPolicy()
	policy.SessionID = sce.GenerateSessionID()
	policy.ScriptEngine = scegotest.NewLuaEngine()
	engine := sce.NewEngine[AiLoopState, AiLoopEvent](&policy)
	engine.RegisterEventProcessor(declaredType, handler)
	return loop{engine: engine, policy: &policy}
}

func silent(sce.HostSendRequest) []sce.HostSendResponse { return nil }

// A booted machine, sitting in `priming` with nothing prompted yet.
func booted() loop {
	l := notInitialised(silent)
	l.engine.Initialize()
	l.engine.Step()
	return l
}

// A run whose first prompt has been sent — the state every scenario below
// starts from.
func started() loop {
	l := booted()
	l.step(AiLoopEventPromptSent)
	return l
}

// Every state currently active, across all three regions.
func (l loop) active() []AiLoopState {
	return l.engine.GetActiveStates()
}

func (l loop) holds(s AiLoopState) bool {
	for _, active := range l.active() {
		if active == s {
			return true
		}
	}
	return false
}

// The active set in the document's own words, for a failure a reader can act
// on: `[working alive within]` says where the machine is, `[24 1 23]` does not.
func (l loop) where() []string {
	return l.names(l.active())
}

func (l loop) names(states []AiLoopState) []string {
	spelled := make([]string, 0, len(states))
	for _, s := range states {
		spelled = append(spelled, l.policy.GetStateName(s))
	}
	return spelled
}

// One event, then a macrostep — the same two calls the Rust channel's helper
// makes, so the two drivers drive the document identically. Engine.ProcessEvent
// already runs a macrostep; the second Step drains what that macrostep queued.
func (l loop) step(event AiLoopEvent) {
	l.engine.ProcessEvent(event)
	l.engine.Step()
}

// The verdict a completed turn is judged on.
//
// `judging` branches on `_event.data.done`, so `judge` is one of the two events
// this document requires a payload from — the host in
// `examples/ai_loop/ai_loop_example.cpp` composes exactly this JSON. Sending
// the event bare is not a shortcut with the same meaning: `_event.data` is then
// nil, indexing it raises `error.execution` (W3C SCXML 5.9.1 has a failed
// `cond` raise and be treated as false), and the run takes the same third
// transition it would have taken on `done:false` while quietly counting an
// error per turn. TestAVerdictWithoutItsPayloadIsReported is that path, and
// TestACorrectlyDrivenRunReportsNoErrors is the floor that makes it a
// measurement.
func (l loop) verdict(done bool) {
	payload := `{"done":false}`
	if done {
		payload = `{"done":true}`
	}
	l.engine.RaiseExternal(AiLoopEventJudge, payload, "")
	l.engine.Step()
}

// One completed turn: the work finished, and the loop decides what next.
func (l loop) turn() {
	l.step(AiLoopEventTurnDone)
	l.verdict(false)
}

func TestAllThreeRegionsAreLiveAtOnce(t *testing.T) {
	l := started()
	if !l.holds(AiLoopStateWorking) || !l.holds(AiLoopStateAlive) || !l.holds(AiLoopStateWithin) {
		t.Fatalf("the cycle, the liveness watch and the budget are orthogonal regions and "+
			"must all be active at once; got %v", l.where())
	}
}

func TestReflectionFiresOnSchedule(t *testing.T) {
	l := started()
	at := 0
	for n := 1; n <= 10; n++ {
		l.turn()
		if l.holds(AiLoopStateReflecting) {
			at = n
			break
		}
	}
	if at != 8 {
		t.Fatalf("the document sets `reflect_every` to 8, so the eighth completed turn is "+
			"the one that reflects; reflection fired at turn %d", at)
	}
}

func TestReflectionGoesThroughARestartAndTheLoopRePrimes(t *testing.T) {
	l := started()
	for n := 1; n <= 8; n++ {
		l.turn()
	}

	l.step(AiLoopEventReflectApplied)
	if !l.holds(AiLoopStateRestarting) {
		t.Fatalf("a session reads its context, MCP config and memory once, when it starts, "+
			"so applying a reflection has to REPLACE the session rather than reconfigure "+
			"it; active: %v", l.where())
	}

	l.step(AiLoopEventSessionReady)
	if !l.holds(AiLoopStatePriming) {
		t.Fatalf("a replaced session starts empty and must be primed with the current "+
			"prompts before it can take a turn; active: %v", l.where())
	}
}

func TestTheBudgetEndsTheRunFromWhereverTheCycleIs(t *testing.T) {
	l := started()
	for n := 1; n <= 60; n++ {
		if l.holds(AiLoopStateReflecting) {
			l.step(AiLoopEventReflectNone)
		}
		if l.holds(AiLoopStateExhausted) {
			break
		}
		l.turn()
	}
	if !l.holds(AiLoopStateExhausted) {
		t.Fatalf("the budget is its own region precisely so the turn count is not something "+
			"`judging` has to remember to check; active: %v", l.where())
	}
}

func TestAStandingInstructionAnswersWithoutWakingAnybody(t *testing.T) {
	l := started()

	l.step(AiLoopEventTurnBlocked)
	if !l.holds(AiLoopStateScreening) {
		t.Fatalf("a dialog is screened against the rules the person wrote in advance "+
			"before anyone is woken; active: %v", l.where())
	}

	l.step(AiLoopEventScreenMatched)
	if !l.holds(AiLoopStateWorking) || l.holds(AiLoopStatePaused) {
		t.Fatalf("a matched rule is a decision the person already made, so the run carries "+
			"on and nobody is woken; active: %v", l.where())
	}
}

func TestAnUnmatchedDialogWakesThePersonWhoAnswers(t *testing.T) {
	l := started()

	l.step(AiLoopEventTurnBlocked)
	l.step(AiLoopEventScreenNone)
	if !l.holds(AiLoopStatePaused) {
		t.Fatalf("the loop answers only what the person decided in advance; anything else "+
			"stops it and waits; active: %v", l.where())
	}

	l.step(AiLoopEventTurnDone)
	if !l.holds(AiLoopStateJudging) {
		t.Fatalf("once the person has answered, the turn completes where it left off; "+
			"active: %v", l.where())
	}
}

// A person answering does not re-introduce the session to itself.
//
// `paused` is a sibling of `running`, so answering targets `judging` and enters
// `running` on the way — as an ANCESTOR. W3C SCXML Appendix D
// addAncestorStatesToEnter adds such a state without its default initial child,
// and here the default is `priming`, whose <onentry> sends the opening prompt.
// An engine that gives every entered compound state its default leaves the
// cycle in two states at once and the host, reading the configuration, sends
// the start prompt again — measured 2026-08-15 on both AOT engines, with every
// W3C fixture green and this document's other seventeen clauses green with it.
//
// The clause itself is pinned across all seven channels by
// `integration_resources/ancestor_entry_is_not_default_entry/`. This scenario is
// the worked example's own stake in it: the document that made the defect
// visible asserts the shape it was found in, so a regression here fails as a
// supervision bug rather than as an abstract entry-set one.
func TestAnsweringAQuestionDoesNotRePrimeTheSession(t *testing.T) {
	l := started()
	l.step(AiLoopEventTurnBlocked)
	l.step(AiLoopEventScreenNone)
	l.step(AiLoopEventTurnDone)

	if !l.holds(AiLoopStateJudging) {
		t.Fatalf("the answered turn has to land in `judging`; active: %v", l.where())
	}
	if l.holds(AiLoopStatePriming) {
		t.Fatalf("`running` has two children active at once: %v. `priming` sends "+
			"`prompt.start`, so a host driving this configuration re-sends the opening "+
			"prompt every time a person answers a dialog", l.where())
	}
}

func TestHoldAndResumeReturnToExactlyWhereTheCycleWas(t *testing.T) {
	l := started()
	l.turn()

	l.step(AiLoopEventHold)
	if !l.holds(AiLoopStatePaused) {
		t.Fatalf("a person looking at the work holds the cycle; active: %v", l.where())
	}

	l.step(AiLoopEventResume)
	if !l.holds(AiLoopStateWorking) {
		t.Fatalf("resuming puts the cycle back to work rather than ending the run; "+
			"active: %v", l.where())
	}
}

// `<history id="where">` declares `<transition target="working"/>` as its
// default, so a hold taken while the cycle is in `working` resumes there whether
// history recorded anything or not — the scenario above cannot tell a working
// history from one that records nothing. Measured: deleting the recording filter
// left it green.
//
// `priming` is the one place the two answers differ. The machine comes up there,
// `hold` is declared above the cycle so it reaches, and the history default
// names `working` — so resuming into `priming` is only possible if the
// configuration was really recorded.
func TestResumeReturnsSomewhereTheHistoryDefaultDoesNot(t *testing.T) {
	l := booted()
	if !l.holds(AiLoopStatePriming) {
		t.Fatalf("the run starts with a session that exists and has not been prompted; "+
			"active: %v", l.where())
	}

	l.step(AiLoopEventHold)
	if !l.holds(AiLoopStatePaused) {
		t.Fatalf("a person can take over before the first prompt as readily as after one; "+
			"active: %v", l.where())
	}

	l.step(AiLoopEventResume)
	if !l.holds(AiLoopStatePriming) || l.holds(AiLoopStateWorking) {
		t.Fatalf("`<history>` must restore the state the cycle was actually in; landing in "+
			"`working` here is the history default answering instead, which is what a "+
			"history that records nothing looks like; active: %v", l.where())
	}
}

func TestThePersonInterruptsTheInnerSessionByHand(t *testing.T) {
	l := started()

	l.step(AiLoopEventTurnInterrupted)
	if !l.holds(AiLoopStatePaused) || l.holds(AiLoopStateScreening) {
		t.Fatalf("a person typing into the session directly is not a dialog to screen — "+
			"the loop stops driving and stays out of the way; active: %v", l.where())
	}

	l.step(AiLoopEventTurnInterrupted)
	if !l.holds(AiLoopStatePaused) {
		t.Fatalf("further interruptions keep it paused rather than fighting the person for "+
			"the session; active: %v", l.where())
	}
}

func TestNobodyComes(t *testing.T) {
	l := started()

	l.step(AiLoopEventTurnBlocked)
	l.step(AiLoopEventScreenNone)
	l.step(AiLoopEventUnattended)
	if !l.holds(AiLoopStateBlocked) {
		t.Fatalf("a question nobody answers ends the run in an outcome the document names, "+
			"rather than leaving it prompting into the dark; active: %v", l.where())
	}
}

func TestAPaneThatDiesMidTurnIsNoticedAndRebuilt(t *testing.T) {
	l := started()

	// The cycle is sitting in `working`, waiting for a turn that will never come
	// because the process is gone. `watch` is the region that sees it.
	l.step(AiLoopEventSessionLost)
	if !l.holds(AiLoopStateRestarting) || !l.holds(AiLoopStateRebuilding) {
		t.Fatalf("a dead session has to be noticed independently of where the turn cycle "+
			"happens to be, which is why the watch is its own region; active: %v", l.where())
	}

	l.step(AiLoopEventSessionReady)
	if !l.holds(AiLoopStatePriming) || !l.holds(AiLoopStateAlive) {
		t.Fatalf("both regions recover together: the run re-primes and the watch goes back "+
			"to alive; active: %v", l.where())
	}
}

func TestOneCancelReachesEveryRegion(t *testing.T) {
	l := started()

	l.step(AiLoopEventCancel)
	if !l.holds(AiLoopStateCancelled) {
		t.Fatalf("cancel is one transition on the `<parallel>` itself rather than one per "+
			"region, so a single event ends all three; active: %v", l.where())
	}
}

// W3C SCXML 5.3: the machine answers what its own datamodel holds.
//
// A host supervising this loop has to size its own work against the budget the
// document declares. Without an accessor the only readable copy is the script
// engine's, reached with an engine handle, a session id and the variable's name
// spelled as a string — three things a consumer should not need, none of them
// checked by a compiler.
//
// The half that decides the shape is `turns`: it is authored 0 and assigned on
// every completed turn, so an accessor that answered the AUTHORED literal would
// keep saying 0 for the whole run.
func TestTheMachineAnswersWhatItsOwnDatamodelHolds(t *testing.T) {
	l := started()

	if got, ok := l.policy.MaxTurns(); !ok || got != 40 {
		t.Fatalf("the authored budget must be readable off the machine itself, in the "+
			"host's own type; got %d (readable: %t)", got, ok)
	}
	if got, ok := l.policy.ReflectEvery(); !ok || got != 8 {
		t.Fatalf("so must the reflection cadence; got %d (readable: %t)", got, ok)
	}
	if got, ok := l.policy.ScreenPermissions(); !ok || got {
		t.Fatalf("a standing answer to permission dialogs is a promise about what the loop "+
			"may do unattended, and a host must be able to inspect it; got %t "+
			"(readable: %t)", got, ok)
	}

	if got, ok := l.policy.Turns(); !ok || got != 0 {
		t.Fatalf("no turn has completed yet, so the bookkeeping still reads its authored "+
			"value; got %d (readable: %t)", got, ok)
	}
	l.turn()
	if got, ok := l.policy.Turns(); !ok || got != 1 {
		t.Fatalf("the accessor must report what the datamodel HOLDS, not what the document "+
			"authored — a value frozen at generation time would still say 0 here; got %d "+
			"(readable: %t)", got, ok)
	}
}

// The strategy a host edits is the strategy it can read back.
//
// The budget above is the numeric half of the datamodel. This is the other half,
// and it is the half the example's own comment calls editable: the north star,
// the milestone, the prompts built from them, the marker that ends the run. A
// supervisor that is going to send `start_prompt` has to be able to see what it
// is about to send, and a UI over this loop has nothing to display without these.
//
// `start_prompt` is asserted through its parts rather than as one literal,
// because it is a concatenation: it exists to prove that a value the document
// COMPUTES from its strings is readable too, not only the ones it spells out.
func TestTheStrategyAHostEditsIsTheStrategyItCanReadBack(t *testing.T) {
	l := started()

	if got, ok := l.policy.DoneMarker(); !ok || got != "MILESTONE REACHED" {
		t.Fatalf("the marker that decides when the run has converged must be readable off "+
			"the machine — a host matching the session's report against it cannot ask the "+
			"document; got %q (readable: %t)", got, ok)
	}
	if got, ok := l.policy.NorthStar(); !ok || got != "(edit me) the outcome this loop exists to reach" {
		t.Fatalf("the goal the author edits is the first thing a supervisor displays; "+
			"got %q (readable: %t)", got, ok)
	}
	if got, ok := l.policy.Milestone(); !ok || got != "(edit me) the next checkpoint on the way there" {
		t.Fatalf("so is the checkpoint it is working toward; got %q (readable: %t)", got, ok)
	}

	start, ok := l.policy.StartPrompt()
	if !ok {
		t.Fatal("the prompt the loop sends into a fresh session must be readable before " +
			"it is sent")
	}
	if !strings.Contains(start, "(edit me) the outcome this loop exists to reach") ||
		!strings.Contains(start, "Report what you did") {
		t.Fatalf("the composed prompt must carry the authored strings it was built from, "+
			"so a host reading it sees what the session will receive: %q", start)
	}
}

// The standing instructions are readable, which is what makes them standing.
//
// `screen_rules` is the block that decides when a person is NOT woken. The
// document keeps it in the authored half deliberately — its own comment says the
// loop is carrying out a decision made in advance and written down — and a
// decision written down where nobody can read it back is indistinguishable from
// the loop deciding on its own authority.
//
// The parts asserted here are the ones a reader acts on — which question is
// matched and what answer it gets — rather than the whole text, so reformatting
// the block inside the document does not fail this.
func TestTheStandingInstructionsCanBeReadBackOffTheMachine(t *testing.T) {
	l := started()

	rules, ok := l.policy.ScreenRules()
	if !ok {
		t.Fatal("the standing-instruction table must be readable off the machine — a host " +
			"that cannot list it cannot show anyone which questions are being answered " +
			"without them")
	}

	if !strings.HasPrefix(rules, "[") {
		t.Fatalf("the block is authored as an array and must come back as one: %q", rules)
	}
	for _, question := range []string{"design-decision", "design-proposal", "multiple-choice"} {
		if !strings.Contains(rules, question) {
			t.Fatalf("`%s` is screened by the document but absent from what the machine "+
				"reports: %q", question, rules)
		}
	}
	if !strings.Contains(rules, "Rethink for the most durable answer") {
		t.Fatalf("the reply a screened question receives is the half a person most needs "+
			"to see, and it is what distinguishes carrying out a decision from making "+
			"one: %q", rules)
	}
}

// A structured variable answers with what it is holding, not with what it was
// declared as.
//
// The scalar readers refuse a value of another type, and this asserts the JSON
// one does too — from both directions. A write into the session must be visible,
// because a reader frozen at generation time would answer the document's literal
// for the whole run; and a scalar written into a variable declared structured
// must read as "cannot answer" rather than as the scalar's own JSON.
//
// The writes go through SetVariable, which takes a value rather than source
// text. That is the half of the engine interface that is the same whichever
// engine a deployment injected — EvaluateExpression takes the ENGINE's language,
// and this runtime is given a Lua one — so a test written in either language
// would be asserting about the injection rather than about the reader.
func TestAStructuredReadFollowsTheAssignmentAndRefusesAnotherType(t *testing.T) {
	l := started()

	engine := l.policy.ScriptEngine
	sid := l.policy.SessionID
	if sid == "" {
		t.Fatal("a started machine holds a session")
	}

	later := []interface{}{map[string]interface{}{"when": "later"}}
	if err := engine.SetVariable(sid, "screen_rules", later); err != nil {
		t.Fatalf("the session takes a structured value: %v", err)
	}

	after, ok := l.policy.ScreenRules()
	if !ok {
		t.Fatal("a reassigned structured variable is still readable")
	}
	if !strings.Contains(after, "later") || strings.Contains(after, "design-decision") {
		t.Fatalf("the reader answered with the authored table after the session was "+
			"assigned another one: %q", after)
	}

	if err := engine.SetVariable(sid, "screen_rules", int64(5)); err != nil {
		t.Fatalf("the session takes a scalar too: %v", err)
	}
	if got, ok := l.policy.ScreenRules(); ok {
		t.Fatalf("a variable declared structured and now holding a number must report that "+
			"the machine cannot answer. `5` is valid JSON, so a reader that forwarded "+
			"whatever the serializer produced would hand a consumer a document shape that "+
			"no longer exists; got %q", got)
	}
}

// What a reflection writes is what the restarted session is primed with.
//
// This is the loop's whole reason for having a restart state: `reflecting`
// rewrites the prompts and `restarting` replaces the session so a fresh one
// reads them. Both halves are invisible to an outcome — a run converges just the
// same whether the text it sent afterwards was the reflection's, the author's,
// or empty.
//
// It is asserted because the example was wrong here: its host wrote
// `{"start_prompt":"","turn_prompt":"","milestone":"refined"}`, so the document
// came back holding two empty strings and the fresh session was primed with
// nothing at all, under a scenario titled "restarts into the improved prompts".
// Measured 2026-08-15 in the example's own output.
func TestWhatAReflectionWritesIsWhatTheMachineThenHolds(t *testing.T) {
	l := started()

	authored, ok := l.policy.StartPrompt()
	if !ok {
		t.Fatal("a started loop can read its opening prompt")
	}

	for n := 1; n <= 8; n++ {
		l.turn()
	}
	if !l.holds(AiLoopStateReflecting) {
		t.Fatalf("the document sets `reflect_every` to 8, so the eighth completed turn "+
			"reflects; active: %v", l.where())
	}

	l.engine.RaiseExternal(
		AiLoopEventReflectApplied,
		`{"start_prompt":"Resuming. Milestone: refined","turn_prompt":"Continue toward: refined","milestone":"refined"}`,
		"",
	)
	l.engine.Step()

	if got, ok := l.policy.Milestone(); !ok || got != "refined" {
		t.Fatalf("the reflection's milestone did not reach the datamodel, so the restart it "+
			"is about to pay for improves nothing; got %q (readable: %t)", got, ok)
	}
	after, ok := l.policy.StartPrompt()
	if !ok {
		t.Fatal("the prompt a restarted session is primed with must still be readable")
	}
	if after != "Resuming. Milestone: refined" {
		t.Fatalf("the machine is not holding what the reflection wrote; got %q", after)
	}
	if after == authored {
		t.Fatal("the reflection has to have changed something, or this scenario would pass " +
			"against a machine that ignored it")
	}
	if after == "" {
		t.Fatal("an empty prompt is what a host sends when reflection erased it, and the " +
			"run still converges — which is why this is asserted rather than watched")
	}
}

// A machine that has not been booted cannot answer, and says so.
//
// The failure this refuses is the one a default-valued field would produce: a
// freshly constructed machine reporting the document's literal as though a
// session had been created and initialised it. Nothing has read the document at
// this point, so "cannot answer" is the only honest response.
func TestAnUninitialisedMachineSaysItCannotAnswer(t *testing.T) {
	policy := NewAiLoopPolicy()
	policy.ScriptEngine = scegotest.NewLuaEngine()

	if got, ok := policy.MaxTurns(); ok {
		t.Fatalf("before Initialize() there is no session holding a datamodel, and "+
			"answering %d would be a claim about a run that has not started", got)
	}
}

// The outcome the loop exists to reach, and the report it asks for first.
//
// The document's opening comment claims the outcomes are enumerated, and five
// finals spell them. Measured 2026-08-23: `converged` — the one a successful run
// ends in — was reached by no scenario in any channel, and neither was the
// `closing` state on the way to it.
//
// `closing` is asserted separately from the terminal because it is the whole
// reason the document does not send `judge` straight to a final: the session is
// asked for a closing report, and only the turn that answers it ends the run. A
// machine that jumped from the verdict to `converged` would satisfy a
// terminal-only check and lose the report.
func TestTheRunConvergesThroughAClosingReport(t *testing.T) {
	l := started()
	l.step(AiLoopEventTurnDone)
	l.verdict(true)

	if !l.holds(AiLoopStateClosing) {
		t.Fatalf("a `done` verdict asks for the closing report before ending the run; "+
			"active: %v", l.where())
	}

	l.step(AiLoopEventTurnDone)

	if !l.holds(AiLoopStateConverged) {
		t.Fatalf("the turn that answers the closing report reaches `reported`, whose "+
			"<raise> is what takes all three regions out at once; active: %v", l.where())
	}
}

// W3C SCXML 5.9.1: a host that forgets the verdict can find out.
//
// `judging` reads `_event.data.done`. A `judge` that carries nothing leaves
// `_event.data` nil, indexing it fails, and the clause says a failed `cond`
// raises `error.execution` and is treated as false — so the run does exactly
// what a `done:false` verdict would do and heads into another turn. The two
// deliveries are indistinguishable from the configuration, from the datamodel
// and from the outcome: a loop driven this way never converges, however finished
// the session reports itself to be, and nothing says why.
//
// What tells them apart is the engine's own count. This is the same shape as
// `unhandled_error_is_observable` and `undecodable_payload_is_reported`: the
// behaviour is correct per the spec, and the defect would be that it is
// unobservable.
func TestAVerdictWithoutItsPayloadIsReported(t *testing.T) {
	l := started()
	l.step(AiLoopEventTurnDone)

	l.step(AiLoopEventJudge)

	if !l.holds(AiLoopStateWorking) {
		t.Fatalf("a `cond` that could not be evaluated is treated as false, so the cycle "+
			"takes the unconditional third transition and works another turn; active: %v",
			l.where())
	}
	if got := l.engine.UnhandledErrorEvents(); got != 1 {
		t.Fatalf("the payload-less verdict raised no error a host could count, so a run "+
			"that will never converge looks exactly like one that has not converged yet; "+
			"unhandled errors = %d", got)
	}
	last, ok := l.engine.LastUnhandledError()
	if !ok || last != AiLoopEventErrorExecution {
		t.Fatalf("the count has to name what it counted; a host reading only a number "+
			"cannot tell a failed `cond` from a failed action; got %v (present: %t)",
			last, ok)
	}
}

// The floor that makes the count above a measurement.
//
// A counter asserted only where it is expected to move measures half of what it
// claims: TestAVerdictWithoutItsPayloadIsReported would pass just as well
// against an engine that raised `error.execution` on every event. So the same
// run, driven the way `ai_loop_example.cpp` drives it, has to raise nothing at
// all — through the reflection and the restart it pays for, which is where the
// document's other payload-carrying event lands.
func TestACorrectlyDrivenRunReportsNoErrors(t *testing.T) {
	l := started()
	for n := 1; n <= 8; n++ {
		l.turn()
	}
	if !l.holds(AiLoopStateReflecting) {
		t.Fatalf("the eighth completed turn reflects; active: %v", l.where())
	}

	l.engine.RaiseExternal(
		AiLoopEventReflectApplied,
		`{"start_prompt":"Resuming. Milestone: refined","turn_prompt":"Continue toward: refined","milestone":"refined"}`,
		"",
	)
	l.engine.Step()
	l.step(AiLoopEventSessionReady)
	l.step(AiLoopEventPromptSent)
	l.turn()

	if got := l.engine.UnhandledErrorEvents(); got != 0 {
		t.Fatalf("a run driven the way the document's own host drives it raises nothing; an "+
			"error here means the channels are not asking the machine the same thing, and "+
			"this one would be asserting clauses about a path no deployment takes; "+
			"unhandled errors = %d", got)
	}
}

// Rebuilding more often than the author allowed is a spent budget, not a broken
// document.
//
// `max_restarts` bounds how many times a session may be replaced. Measured
// 2026-08-23: no channel named it, so `stuck` — one of the two states that reach
// `exhausted` — was reachable only in prose. The budget region's `max_turns` had
// a witness; this one had none, and the two are different mechanisms that happen
// to share a terminal.
//
// A lost session is the cheap way in: `drive` answers `session.lost` with a
// restart from wherever the cycle is, which is the same door reflection uses and
// the one a real deployment hits when a process dies.
func TestASessionReplacedPastItsBudgetReportsStuck(t *testing.T) {
	l := started()
	allowed, ok := l.policy.MaxRestarts()
	if !ok {
		t.Fatal("the document declares a restart budget")
	}

	for n := int64(1); n <= allowed; n++ {
		l.step(AiLoopEventSessionLost)
		l.step(AiLoopEventSessionReady)
		if !l.holds(AiLoopStatePriming) {
			t.Fatalf("replacement %d of %d is within the budget, so the fresh session is "+
				"primed with whatever the loop has written by now; active: %v",
				n, allowed, l.where())
		}
	}

	l.step(AiLoopEventSessionLost)
	l.step(AiLoopEventSessionReady)

	if !l.holds(AiLoopStateExhausted) {
		t.Fatalf("the replacement past `max_restarts` reaches `stuck`, which reports the "+
			"run as exhausted rather than failed; active: %v", l.where())
	}
}

// W3C SCXML 6.2.5: the document tells a host what to do, in its own words.
//
// Every scenario above registers a handler that answers nothing, because what
// they measure is the topology and each supplies its own events. That makes them
// blind to the thing this scenario asserts: with a silent handler, a <send> that
// LOST its `type="x-sce-host"` behaves exactly like one that kept it — nothing
// is delivered either way — so the whole conversion could rot back to targetless
// sends with every channel green.
//
// So this one records instead of ignoring. It pins that entering `priming` asks
// the host to prompt, and that the prompt text rides ON the act: the host is TOLD
// what to send rather than reaching into the datamodel behind the machine to find
// out, which is the difference the conversion bought.
func TestTheDocumentDeclaresItsActsToTheHost(t *testing.T) {
	type act struct {
		event string
		text  string
	}
	var seen []act

	l := notInitialised(func(req sce.HostSendRequest) []sce.HostSendResponse {
		text := ""
		if values := req.Params["text"]; len(values) > 0 {
			text = values[0]
		}
		seen = append(seen, act{event: req.EventName, text: text})
		return nil
	})
	l.engine.Initialize()

	if len(seen) == 0 {
		t.Fatal("entering `priming` asked the host to perform nothing at all")
	}
	if seen[0].event != "prompt.start" {
		t.Fatalf("entering `priming` did not ask the host to prompt; the acts seen were %v",
			seen)
	}
	if !strings.Contains(seen[0].text, "North star:") {
		t.Fatalf("the act carried no prompt, so a host would have to reach past the machine "+
			"for one: %q", seen[0].text)
	}
}

// The sibling of TestOneCancelReachesEveryRegion.
//
// The document writes `fail` and `cancel` once each on the <parallel> and says
// so in a comment — one transition rather than one per region, because a run
// ends as a whole. Only `cancel` was asserted, and the two are not the same
// claim: they are separate transitions to separate terminals, and a consumer
// distinguishing "the run broke" from "somebody stopped it" reads which final it
// ended in.
func TestAFailureEndsTheWholeRun(t *testing.T) {
	l := started()

	l.step(AiLoopEventFail)

	if !l.holds(AiLoopStateFailed) {
		t.Fatalf("`fail` is written on the `<parallel>` itself, so one event takes all "+
			"three regions to `failed` — a different outcome from `cancelled`, which is "+
			"what tells a broken run from a stopped one; active: %v", l.where())
	}
}

// ══════════════════════════════════════════════════════════════════
// A run that outlived its process
//
// Engine.EnterAt takes states, and nothing that crosses a process boundary can
// carry one: a journal, a wire and a file all carry STRINGS. GetStateName writes
// that record and GetStateFromName reads it back, and until the second existed
// the door could be called and its argument could not be built — a supervisor
// coming back had to Initialize() instead, which is a replay rather than a
// resume: `priming` performs its prompt on entry, so the restored loop typed the
// first prompt again.
//
// A consumer-side table mapping the names it knows to states would compile and
// would age silently — the document gains a state, the table does not, the name
// reads back as absent, and the resume quietly becomes a fresh start. Only the
// generator writes the half that ages with the document, which is why these two
// scenarios drive a GENERATED policy rather than a hand-written one.
// ══════════════════════════════════════════════════════════════════

func TestARunJournalledAsNamesResumesWhereItStopped(t *testing.T) {
	ran := started()
	ran.turn()
	ran.turn()

	// Everything a host can persist. Not states, not a configuration: text.
	journal := ran.names(ran.active())
	journalledCurrent := ran.policy.GetStateName(ran.engine.GetCurrentState())
	if !contains(journal, "working") {
		t.Fatalf("the journal is meant to be taken mid-run, with the cycle at work; it "+
			"reads %v", journal)
	}

	// A new process, holding nothing but those strings.
	var acts []string
	resumed := notInitialised(func(req sce.HostSendRequest) []sce.HostSendResponse {
		acts = append(acts, req.EventName)
		return nil
	})

	configuration := make([]AiLoopState, 0, len(journal))
	for _, name := range journal {
		state, ok := resumed.policy.GetStateFromName(name)
		if !ok {
			t.Fatalf("`%s` is a name this policy published through GetStateName and it did "+
				"not read back, so a configuration cannot survive its own record", name)
		}
		configuration = append(configuration, state)
	}
	current, ok := resumed.policy.GetStateFromName(journalledCurrent)
	if !ok {
		t.Fatalf("the current state's own name `%s` did not read back", journalledCurrent)
	}

	if rejection := resumed.engine.EnterAt(configuration, current); rejection != sce.ConfigurationAccepted {
		t.Fatalf("a configuration this document published is one it can be put back into; "+
			"the engine refused %v with rejection %v", journal, rejection)
	}

	if got := resumed.active(); !sameStates(got, configuration) {
		t.Fatalf("the machine came back somewhere other than where the journal said it "+
			"was: %v against %v", resumed.names(got), journal)
	}
	if got := resumed.engine.GetCurrentState(); got != current {
		t.Fatalf("the resumed machine's current state is %s, not the journalled %s",
			resumed.policy.GetStateName(got), journalledCurrent)
	}
	if !resumed.engine.IsRunning() {
		t.Fatal("a machine put back into a configuration it published is running in it")
	}
	if len(acts) != 0 {
		t.Fatalf("resuming performed acts, which is the replay EnterAt exists to avoid — a "+
			"host would see the run's earlier prompts sent a second time; performed %v",
			acts)
	}
}

func TestEveryStateARunReachesReadsBackFromItsOwnName(t *testing.T) {
	var seen []AiLoopState
	record := func(l loop) {
		for _, s := range l.active() {
			if !holdsState(seen, s) {
				seen = append(seen, s)
			}
		}
	}

	// Every outcome the document names, walked rather than listed: a state is
	// recorded here only because a run actually stood in it, and a written-out
	// list of states is the thing GetStateFromName exists to replace.
	l := started()
	record(l)
	for n := 1; n <= 60; n++ {
		if l.holds(AiLoopStateReflecting) {
			record(l)
			l.step(AiLoopEventReflectApplied)
			record(l)
			l.step(AiLoopEventSessionReady)
		}
		if l.holds(AiLoopStateExhausted) {
			break
		}
		l.turn()
		record(l)
	}
	record(l)

	l = started()
	l.step(AiLoopEventTurnDone)
	l.verdict(true)
	record(l)
	l.step(AiLoopEventTurnDone)
	record(l)

	l = started()
	l.step(AiLoopEventTurnBlocked)
	record(l)
	l.step(AiLoopEventScreenNone)
	record(l)
	l.step(AiLoopEventUnattended)
	record(l)

	l = started()
	l.step(AiLoopEventHold)
	record(l)
	l.step(AiLoopEventResume)
	record(l)

	l = started()
	l.step(AiLoopEventSessionLost)
	record(l)
	l.step(AiLoopEventSessionReady)
	record(l)

	l = started()
	l.step(AiLoopEventCancel)
	record(l)

	l = started()
	l.step(AiLoopEventFail)
	record(l)

	// A floor, not a target: without one, a table that had lost every arm but the
	// first would pass this by being asked about a single state.
	if len(seen) < 20 {
		t.Fatalf("these scenarios are meant to stand in most of the document; they reached "+
			"%d states: %v", len(seen), l.names(seen))
	}

	for _, state := range seen {
		name := l.policy.GetStateName(state)
		back, ok := l.policy.GetStateFromName(name)
		if !ok || back != state {
			t.Fatalf("`%s` did not read back as the state that published it", name)
		}
	}

	// The other half of the contract: a name the document does not carry is
	// refused rather than guessed at. A table that answers anyway turns a stale
	// journal into a plausible-looking resume, which is the one outcome a host
	// has no way to detect afterwards.
	for _, absent := range []string{"no-such-state", ""} {
		if state, ok := l.policy.GetStateFromName(absent); ok {
			t.Fatalf("`%s` is not a state this document carries, and it read back as %s",
				absent, l.policy.GetStateName(state))
		}
	}
	if state, ok := l.policy.GetStateFromName("turn.done"); ok {
		t.Fatalf("an event name is not a state name; the two tables are separate on "+
			"purpose, and `turn.done` read back as %s", l.policy.GetStateName(state))
	}
}

func contains(haystack []string, needle string) bool {
	for _, candidate := range haystack {
		if candidate == needle {
			return true
		}
	}
	return false
}

func holdsState(haystack []AiLoopState, needle AiLoopState) bool {
	for _, candidate := range haystack {
		if candidate == needle {
			return true
		}
	}
	return false
}

func sameStates(a, b []AiLoopState) bool {
	if len(a) != len(b) {
		return false
	}
	for i := range a {
		if a[i] != b[i] {
			return false
		}
	}
	return true
}
