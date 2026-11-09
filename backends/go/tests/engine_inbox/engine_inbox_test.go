// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// docs/adr/0084, ARCHITECTURE.md "Engine Threading": a Go engine belongs to the
// goroutine that drives it, and what another goroutine has for it enters through
// its inbox (Engine.Inbox).
//
// The Go sibling of the Kotlin channel's
// WhatAnotherThreadPostsEntersThroughTheInboxTest and the generated C++ engine's
// WhatAnotherThreadPostsEntersThroughTheInboxAotTest. The cases hold the inbox to
// its contract:
//
//   - many goroutines post endings and external events while the owner drives
//     macrosteps, and what the machine takes, and what is left running, are
//     those of a single-goroutine replay of the same posts. Under
//     `go test -race` a post that reached the engine's state other than through
//     the inbox is reported as a race;
//   - an ending posted for a start that was cancelled, or started again, is
//     refused by the token check, as a direct one is (§scxml-6.4);
//   - a posted ending carries the origin it names, to this machine and to a
//     session named by its id;
//   - a Mesh delivery is posted and applied as ApplyMeshCall applies it;
//   - a post is taken before the run starts, and refused once the run is
//     stopped or has ended, after what the inbox still held is applied;
//   - the external queue takes events in the order they arrived: a post made
//     during a macrostep is applied before an event that macrostep sends to its
//     own session after it (W3C test 201's shape);
//   - an ending posted for a child session, from inside the parent's macrostep,
//     is taken by the child before an event the parent sends it later in that
//     macrostep (docs/adr/0084, D2 as amended at step 10);
//   - a post to a child's inbox wakes the tree's owner, whose next tick applies
//     it, and a post naming a session the tree does not hold, or a child that
//     has ended, is refused.
//
// The probe machine is written by hand, as the Kotlin one is: the smallest one
// with a host-run invocation. The tree cases drive generated machines instead,
// since a child session is the generated parent's to start. `working` (initial) starts `job` when the macrostep that
// entered it ends and cancels it on exit; `leave` goes to `idle` and `go` back,
// which starts `job` again under a new token; `finish` goes to the top-level
// <final> `over`. `ping`, `done.invoke.job` and `error.invoke.job` are taken in
// `working` and `idle` and recorded as `ping:<data>`, `done:<data>` and
// `failed:<data>`, with the `_event.origin` of each beside it. `call` is taken
// there too: it calls the host's callback, and then sends `ping` with the data
// `own` to the machine's own session, as a `<send>` with no target does.
//
// This package is deliberately NOT a fixture stem: the claim is about a runtime
// door and not about a topology. See `docs/SCE_INTEGRATION_FIXTURE_LAYOUT.md` for
// what a stem commits to.
package engine_inbox

import (
	"fmt"
	"slices"
	"strconv"
	"strings"
	"sync"
	"sync/atomic"
	"testing"
	"time"

	sce "github.com/newmassrael/sce-go-runtime"

	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_invoke_ending_order"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_invoke_tree"
)

type probeState int

const (
	probeWorking probeState = iota
	probeIdle
	probeOver
)

type probeEvent int

const (
	// probeNull is the null event eventless selection is asked with; the probe
	// has no eventless transition.
	probeNull probeEvent = iota
	probeGo
	probeLeave
	probeFinish
	probePing
	probeJobDone
	probeJobFailed
	probeCall
)

const (
	// probeType is the `<invoke type>` the host serves.
	probeType = "x-sce-probe"
	// probeJob is the invocation's id.
	probeJob = "job"
	// bound is far above anything a case waits for, so a stall fails rather
	// than hangs.
	bound = 10 * time.Second
)

// The transition indices FirstEnabledTransition hands out and
// ExecuteTransitionContent is handed back.
const (
	transitionGo = iota
	transitionLeave
	transitionFinish
	transitionPing
	transitionDone
	transitionFailed
	transitionCall
)

var probeEventNames = map[probeEvent]string{
	probeGo:        "go",
	probeLeave:     "leave",
	probeFinish:    "finish",
	probePing:      "ping",
	probeJobDone:   sce.CreateDoneInvokeEventName(probeJob),
	probeJobFailed: sce.ErrorInvokePrefix + probeJob,
	probeCall:      "call",
}

var probeStateNames = map[probeState]string{
	probeWorking: "working",
	probeIdle:    "idle",
	probeOver:    "over",
}

type probeEngine = sce.Engine[probeState, probeEvent]

type probeTarget = sce.EntryTarget[probeState, sce.HistoryID]

type probeTransition = sce.EnabledTransition[probeState, sce.HistoryID]

// probePolicy is the probe's document, as a generated policy would answer it.
type probePolicy struct {
	sessionID string
	// jobPending says `working` was entered and `job` starts when the macrostep
	// ends.
	jobPending bool
	// eventData and eventOrigin are `_event.data` and `_event.origin` of the
	// event being taken.
	eventData   string
	eventOrigin string
	// taken is what the machine took, in order, and origins the `_event.origin`
	// of each. Written by the owner.
	taken   []string
	origins []string
	// duringCall, when set, is the host's callback `call` makes inside its
	// macrostep, before it sends `ping` to its own session.
	duringCall func()
}

var _ sce.StatePolicy[probeState, probeEvent] = (*probePolicy)(nil)

func probeStateTarget(state probeState) probeTarget {
	return sce.StateTarget[probeState, sce.HistoryID](state)
}

func (p *probePolicy) InitialState() probeState {
	return probeWorking
}

func (p *probePolicy) IsFinalState(state probeState) bool {
	return state == probeOver
}

func (p *probePolicy) GetParent(probeState) (probeState, bool) {
	return probeWorking, false
}

func (p *probePolicy) IsCompoundState(probeState) bool {
	return false
}

func (p *probePolicy) IsParallelState(probeState) bool {
	return false
}

func (p *probePolicy) GetChildStates(probeState) []probeState {
	return nil
}

func (p *probePolicy) GetInitialTargets(probeState) []probeTarget {
	return nil
}

func (p *probePolicy) GetDocumentInitialTargets() []probeTarget {
	return []probeTarget{probeStateTarget(probeWorking)}
}

func (p *probePolicy) GetHistoryParent(history sce.HistoryID) probeState {
	panic(fmt.Sprintf("the probe declares no <history>; asked for %d", history))
}

func (p *probePolicy) GetHistoryDefaultTargets(history sce.HistoryID) []probeTarget {
	panic(fmt.Sprintf("the probe declares no <history>; asked for %d", history))
}

func (p *probePolicy) GetDocumentOrder(state probeState) int {
	return int(state)
}

func (p *probePolicy) GetEventName(event probeEvent) string {
	return probeEventNames[event]
}

func (p *probePolicy) GetStateName(state probeState) string {
	return probeStateNames[state]
}

func (p *probePolicy) NullEvent() probeEvent {
	return probeNull
}

func (p *probePolicy) BindCurrentEvent(probeEvent, *probeEngine) {
}

func (p *probePolicy) GetEventFromName(name string) (probeEvent, bool) {
	for event, eventName := range probeEventNames {
		if eventName == name {
			return event, true
		}
	}
	return probeNull, false
}

func (p *probePolicy) GetStateFromName(name string) (probeState, bool) {
	for state, stateName := range probeStateNames {
		if stateName == name {
			return state, true
		}
	}
	return probeWorking, false
}

func (p *probePolicy) HistoryValue(sce.HistoryID) ([]probeState, bool) { return nil, false }

func (p *probePolicy) FirstEnabledTransition(state probeState, event probeEvent, _ *probeEngine) (probeTransition, bool) {
	if state == probeOver {
		return probeTransition{}, false
	}
	to := func(target probeState, index int) (probeTransition, bool) {
		return probeTransition{Source: state, Targets: []probeTarget{probeStateTarget(target)}, TransitionIndex: index}, true
	}
	recorded := func(index int) (probeTransition, bool) {
		return probeTransition{Source: state, TransitionIndex: index, HasActions: true}, true
	}
	switch event {
	case probeGo:
		if state == probeIdle {
			return to(probeWorking, transitionGo)
		}
	case probeLeave:
		if state == probeWorking {
			return to(probeIdle, transitionLeave)
		}
	case probeFinish:
		return to(probeOver, transitionFinish)
	case probePing:
		return recorded(transitionPing)
	case probeJobDone:
		return recorded(transitionDone)
	case probeJobFailed:
		return recorded(transitionFailed)
	case probeCall:
		return recorded(transitionCall)
	}
	return probeTransition{}, false
}

func (p *probePolicy) ExecuteTransitionContent(_ probeState, transitionIndex int, engine *probeEngine) {
	var kind string
	switch transitionIndex {
	case transitionCall:
		if p.duringCall != nil {
			p.duringCall()
		}
		// What the generated `<send event="ping"/>` raises: an external event
		// whose sender is this session.
		own := sce.NewEventWithMetadata(probePing)
		own.Metadata = sce.SentBySession("", p.sessionID)
		own.Metadata.Data = "own"
		engine.RaiseExternalWithMeta(own)
		return
	case transitionPing:
		kind = "ping"
	case transitionDone:
		kind = "done"
	case transitionFailed:
		kind = "failed"
	default:
		return
	}
	p.taken = append(p.taken, kind+":"+p.eventData)
	p.origins = append(p.origins, p.eventOrigin)
}

func (p *probePolicy) ExecuteEntryActions(state probeState, _ *probeEngine, _ bool) {
	if state == probeWorking {
		p.jobPending = true
	}
}

func (p *probePolicy) ExecuteExitActions(state probeState, engine *probeEngine, _ []probeState) {
	if state != probeWorking {
		return
	}
	p.jobPending = false
	engine.CancelHostInvoke(probeType, probeJob)
}

func (p *probePolicy) ExecutePendingInvokes(engine *probeEngine) {
	if !p.jobPending {
		return
	}
	p.jobPending = false
	engine.PerformHostInvoke(sce.HostInvokeRequest{ProcessorType: probeType, InvokeID: probeJob})
}

func (p *probePolicy) ExecuteHistoryDefaultContent(sce.HistoryID, *probeEngine) {}

func (p *probePolicy) HasParallelStates() bool {
	return false
}

func (p *probePolicy) NeedsScriptEngine() bool {
	return false
}

func (p *probePolicy) NeedsDataModelInit() bool {
	return false
}

func (p *probePolicy) HasInvokeSupport() bool {
	return true
}

func (p *probePolicy) HasFinalize() bool {
	return false
}

func (p *probePolicy) HasAutoforward() bool {
	return false
}

func (p *probePolicy) HasActiveStates() bool {
	return false
}

func (p *probePolicy) HasChildTick() bool {
	return false
}

func (p *probePolicy) InitializeDataModel(*probeEngine) *sce.ScriptSessionFailure {
	return nil
}

func (p *probePolicy) ExecuteFinalizeForChildEvent(*sce.EventWithMetadata[probeEvent], *probeEngine) {
}

func (p *probePolicy) GetActiveStates() []probeState {
	return nil
}

func (p *probePolicy) SetActiveStates([]probeState) {
}

func (p *probePolicy) ForwardToAutoforwardChildren(string, sce.EventMetadata, *probeEngine) {
}

func (p *probePolicy) TickChildren(*probeEngine) {
}

func (p *probePolicy) PopulateEventMetadata(meta *sce.EventMetadata) {
	p.eventData = meta.Data
	p.eventOrigin = meta.OriginAddress()
}

func (p *probePolicy) LiftTypedPayload(probeEvent, *sce.EventMetadata) error { return nil }

func (p *probePolicy) ClearEventMetadata() {
	p.eventData = ""
	p.eventOrigin = ""
}

// OwnSessionID is the session id an ending names this machine by.
func (p *probePolicy) OwnSessionID() string { return p.sessionID }

// SetSessionID gives the probe the id its host has for it, as a generated policy
// takes one (sce.SessionIDSetter).
func (p *probePolicy) SetSessionID(id string) error {
	if reason := sce.SessionIDRefusalOf(id); reason != "" {
		return sce.Refuse(reason)
	}
	p.sessionID = id
	return nil
}

// HostInvokeIDs makes `done.invoke.job` the host's alone to raise, as a
// generated policy does for a host-run invocation.
func (p *probePolicy) HostInvokeIDs() []string { return []string{probeJob} }

// probe is one engine of the probe with its host.
type probe struct {
	engine *probeEngine
	policy *probePolicy
	// log is what the host's invoker was told, in order: `start:<token>` and
	// `cancel:<token>`. Written on the owner.
	log []string
	// onCancel, when set, is called by the invoker after it records a cancel.
	onCancel func()
}

// newProbe is a probe whose host serves `job` and runs the work past the call,
// so `job` runs until the host ends it or the machine leaves `working`.
func newProbe() *probe {
	policy := &probePolicy{sessionID: sce.GenerateSessionID()}
	pr := &probe{engine: sce.NewEngine[probeState, probeEvent](policy), policy: policy}
	pr.engine.RegisterInvoker(probeType, func(event sce.HostInvokeEvent) *sce.HostInvokeResponse {
		if event.Start != nil {
			pr.log = append(pr.log, fmt.Sprintf("start:%d", event.Start.Token))
		}
		if event.Cancel != nil {
			pr.log = append(pr.log, fmt.Sprintf("cancel:%d", event.Cancel.Token))
			if pr.onCancel != nil {
				pr.onCancel()
			}
		}
		return nil
	})
	return pr
}

// latestToken is the token of the latest start the host was told of.
func (pr *probe) latestToken(t *testing.T) uint64 {
	t.Helper()
	for i := len(pr.log) - 1; i >= 0; i-- {
		if digits, ok := strings.CutPrefix(pr.log[i], "start:"); ok {
			token, err := strconv.ParseUint(digits, 10, 64)
			if err != nil {
				t.Fatalf("the invoker recorded a start it could not read: %q", pr.log[i])
			}
			return token
		}
	}
	t.Fatalf("`job` never started: %v", pr.log)
	return 0
}

// ending is the end of start token of `job`: a completion when done, a failure
// otherwise.
func ending(token uint64, done bool, data string) sce.HostInvokeEnding {
	return sce.HostInvokeEnding{ProcessorType: probeType, InvokeID: probeJob, Token: token, Done: done, Data: data}
}

// postKind says which input a post is.
type postKind int

const (
	postPing postKind = iota
	postPingByName
	postDone
	postFailed
)

// post is one input a producer posts, kept so the expectation and the replay
// can be made from what the inbox took.
type post struct {
	kind  postKind
	data  string
	token uint64
}

// to posts p to inbox and answers whether the inbox took it.
func (p post) to(inbox sce.Inbox[probeEvent]) bool {
	switch p.kind {
	case postPing:
		return inbox.RaiseExternal(probePing, p.data)
	case postPingByName:
		return inbox.RaiseExternalByName("ping", p.data)
	case postDone:
		return inbox.EndHostInvoke("", ending(p.token, true, p.data))
	default:
		return inbox.EndHostInvoke("", ending(p.token, false, p.data))
	}
}

// postOf is post k of producer: external events, typed and by name, and now and
// then an ending of token, completed or failed. Each data names its producer and
// its place, so the record says which post it came from.
func postOf(producer, k int, token uint64) post {
	data := fmt.Sprintf("p%d-%d", producer, k)
	switch k % 7 {
	case 3:
		return post{kind: postDone, data: data, token: token}
	case 5:
		return post{kind: postFailed, data: data, token: token}
	case 1:
		return post{kind: postPingByName, data: data}
	default:
		return post{kind: postPing, data: data}
	}
}

// expectedFrom is what the machine must take from posted, in the order the posts
// were taken: every event, and of the endings only the first, since it ends
// `job` and the token check refuses the rest.
func expectedFrom(posted []post) []string {
	ended := false
	var expected []string
	for _, p := range posted {
		switch p.kind {
		case postPing, postPingByName:
			expected = append(expected, "ping:"+p.data)
		case postDone, postFailed:
			if ended {
				continue
			}
			ended = true
			if p.kind == postDone {
				expected = append(expected, "done:"+p.data)
			} else {
				expected = append(expected, "failed:"+p.data)
			}
		}
	}
	return expected
}

// replay is posted replayed on a probe of its own by one goroutine: every post,
// then one tick, then the stop. It answers what the machine took and what the
// host was told.
func replay(t *testing.T, posted []post) ([]string, []string) {
	t.Helper()
	pr := newProbe()
	if err := pr.engine.Initialize(); err != nil {
		t.Fatal(err)
	}
	inbox := pr.engine.Inbox()
	for _, p := range posted {
		if !p.to(inbox) {
			t.Fatalf("the replay's inbox refused a post")
		}
	}
	pr.engine.Tick()
	pr.engine.Stop()
	return pr.policy.taken, pr.log
}

// postFromAnotherGoroutine posts with send from a goroutine of its own, as a
// host's worker does, and answers whether the inbox took it.
func postFromAnotherGoroutine[E comparable](t *testing.T, inbox sce.Inbox[E], send func(sce.Inbox[E]) bool) bool {
	t.Helper()
	answer := make(chan bool, 1)
	go func() { answer <- send(inbox) }()
	select {
	case taken := <-answer:
		return taken
	case <-time.After(bound):
		t.Fatalf("a post from another goroutine never returned")
		return false
	}
}

func TestPostsFromManyGoroutinesDuringMacrostepsAreTakenAsASingleGoroutineReplayTakesThem(t *testing.T) {
	const producers, perProducer = 8, 60

	pr := newProbe()
	inbox := pr.engine.Inbox()
	// The waker is what tells this owner there is something to apply; the owner
	// sleeps on it between ticks rather than on a clock.
	wake := make(chan struct{}, 1)
	signal := func() {
		select {
		case wake <- struct{}{}:
		default:
		}
	}
	inbox.WakeWith(signal)
	if err := pr.engine.Initialize(); err != nil {
		t.Fatal(err)
	}
	token := pr.latestToken(t)

	// The order the inbox took the posts in is recorded under one lock with the
	// post, so it is the order of the inbox itself.
	var (
		order    sync.Mutex
		posted   []post
		refused  []string
		running  atomic.Int32
		finished sync.WaitGroup
	)
	running.Store(producers)
	release := make(chan struct{})
	for producer := 0; producer < producers; producer++ {
		finished.Add(1)
		go func(producer int) {
			defer finished.Done()
			<-release
			for k := 0; k < perProducer; k++ {
				input := postOf(producer, k, token)
				order.Lock()
				if input.to(inbox) {
					posted = append(posted, input)
				} else {
					refused = append(refused, fmt.Sprintf("post %d/%d", producer, k))
				}
				order.Unlock()
			}
			// After it stops counting itself, so an owner asleep on the waker
			// is not left waiting by the last producer.
			running.Add(-1)
			signal()
		}(producer)
	}
	close(release)

	timeout := time.NewTimer(bound)
	defer timeout.Stop()
	for running.Load() > 0 {
		select {
		case <-wake:
		case <-timeout.C:
			t.Fatalf("the producers did not finish posting within %v", bound)
		}
		pr.engine.Tick()
	}
	finished.Wait()
	// What the last producer posted after the owner's last tick.
	pr.engine.Tick()
	// The invocation table: the first ending ended `job`, so the stop that exits
	// `working` has nothing to cancel.
	pr.engine.Stop()

	if len(refused) != 0 {
		t.Fatalf("an open inbox refused posts: %v", refused)
	}
	if len(posted) != producers*perProducer {
		t.Fatalf("the inbox took %d posts of %d", len(posted), producers*perProducer)
	}
	expected := expectedFrom(posted)
	if !slices.Equal(pr.policy.taken, expected) {
		t.Fatalf("the machine did not take the posts in the order the inbox took them:\n got %v\nwant %v",
			pr.policy.taken, expected)
	}
	if want := []string{fmt.Sprintf("start:%d", token)}; !slices.Equal(pr.log, want) {
		t.Fatalf("an ending was not applied, or a refused one was: the host was told %v, want %v", pr.log, want)
	}
	replayTaken, replayLog := replay(t, posted)
	if !slices.Equal(pr.policy.taken, replayTaken) {
		t.Fatalf("the run differs from a single-goroutine replay of its posts:\n run %v\nreplay %v",
			pr.policy.taken, replayTaken)
	}
	if !slices.Equal(pr.log, replayLog) {
		t.Fatalf("the invocations differ from a single-goroutine replay of the posts: run %v, replay %v",
			pr.log, replayLog)
	}
}

func TestAPostedEndingOfACancelledOrRestartedStartIsRefusedAsADirectOneIs(t *testing.T) {
	pr := newProbe()
	inbox := pr.engine.Inbox()
	if err := pr.engine.Initialize(); err != nil {
		t.Fatal(err)
	}
	first := pr.latestToken(t)

	// `leave` exits `working`, which cancels the first start.
	pr.engine.RaiseExternal(probeLeave, "")
	pr.engine.Tick()
	if want := []string{fmt.Sprintf("start:%d", first), fmt.Sprintf("cancel:%d", first)}; !slices.Equal(pr.log, want) {
		t.Fatalf("leaving `working` did not cancel `job`: the host was told %v, want %v", pr.log, want)
	}

	if !postFromAnotherGoroutine(t, inbox, func(in sce.Inbox[probeEvent]) bool {
		return in.EndHostInvoke("", ending(first, true, "after-cancel"))
	}) {
		t.Fatalf("an open inbox refused an ending")
	}
	pr.engine.Tick()
	if len(pr.policy.taken) != 0 {
		t.Fatalf("a cancelled start's posted ending was taken: %v", pr.policy.taken)
	}
	if pr.engine.CompleteHostInvoke(probeType, probeJob, first, "direct") {
		t.Fatalf("a cancelled start's direct ending was taken")
	}

	// `go` enters `working` again: the same invocation, a new start.
	pr.engine.RaiseExternal(probeGo, "")
	pr.engine.Tick()
	second := pr.latestToken(t)
	if second == first {
		t.Fatalf("a restart reused the first start's token %d", first)
	}

	if !postFromAnotherGoroutine(t, inbox, func(in sce.Inbox[probeEvent]) bool {
		return in.EndHostInvoke("", ending(first, false, "stale"))
	}) {
		t.Fatalf("an open inbox refused an ending")
	}
	// Addressed by the session id, as a host serving a tree ends one.
	if !postFromAnotherGoroutine(t, inbox, func(in sce.Inbox[probeEvent]) bool {
		return in.EndHostInvoke(pr.policy.sessionID, ending(second, true, "fresh"))
	}) {
		t.Fatalf("an open inbox refused an ending")
	}
	pr.engine.Tick()
	if want := []string{"done:fresh"}; !slices.Equal(pr.policy.taken, want) {
		t.Fatalf("the first start's ending was taken for the second's: took %v, want %v", pr.policy.taken, want)
	}
	if pr.engine.FailHostInvoke(probeType, probeJob, first, "direct", nil) {
		t.Fatalf("the first start's direct ending was taken")
	}
}

func TestAPostedEndingCarriesTheOriginItNames(t *testing.T) {
	pr := newProbe()
	inbox := pr.engine.Inbox()
	if err := pr.engine.Initialize(); err != nil {
		t.Fatal(err)
	}
	first := pr.latestToken(t)

	// To this machine: applied with CompleteHostInvokeFrom.
	answered := ending(first, true, "answered")
	answered.Origin = sce.NewEventOrigin(sce.MeshRPCInvokeType, "mesh://peer")
	if !postFromAnotherGoroutine(t, inbox, func(in sce.Inbox[probeEvent]) bool {
		return in.EndHostInvoke("", answered)
	}) {
		t.Fatalf("an open inbox refused an ending")
	}
	pr.engine.Tick()

	// `leave` and `go` start `job` again.
	pr.engine.RaiseExternal(probeLeave, "")
	pr.engine.RaiseExternal(probeGo, "")
	pr.engine.Tick()
	second := pr.latestToken(t)

	// To the session named by its id: applied with EndHostInvokeIn.
	failed := ending(second, false, "refused")
	failed.Origin = sce.NewEventOrigin(sce.MeshRPCInvokeType, "mesh://other")
	if !postFromAnotherGoroutine(t, inbox, func(in sce.Inbox[probeEvent]) bool {
		return in.EndHostInvoke(pr.policy.sessionID, failed)
	}) {
		t.Fatalf("an open inbox refused an ending")
	}
	pr.engine.Tick()

	if want := []string{"done:answered", "failed:refused"}; !slices.Equal(pr.policy.taken, want) {
		t.Fatalf("the endings were not both taken: took %v, want %v", pr.policy.taken, want)
	}
	if want := []string{"mesh://peer", "mesh://other"}; !slices.Equal(pr.policy.origins, want) {
		t.Fatalf("a posted ending lost the origin it named: _event.origin %v, want %v", pr.policy.origins, want)
	}
}

func TestAMeshDeliveryIsPostedAndAppliedAsApplyMeshCallAppliesIt(t *testing.T) {
	pr := newProbe()
	inbox := pr.engine.Inbox()
	if err := pr.engine.Initialize(); err != nil {
		t.Fatal(err)
	}
	token := pr.latestToken(t)

	raise := sce.MeshCall{Kind: sce.MeshCallRaise, Name: "ping", Metadata: sce.MeshEventMetadata("peer", "remote", "")}
	if !postFromAnotherGoroutine(t, inbox, func(in sce.Inbox[probeEvent]) bool { return in.ApplyMeshCall(raise) }) {
		t.Fatalf("an open inbox refused a Mesh delivery")
	}
	pr.engine.Tick()

	if want := []string{"ping:peer"}; !slices.Equal(pr.policy.taken, want) {
		t.Fatalf("the posted Mesh delivery was not taken: took %v, want %v", pr.policy.taken, want)
	}
	if want := []string{"mesh://remote"}; !slices.Equal(pr.policy.origins, want) {
		t.Fatalf("the posted Mesh delivery lost its origin: _event.origin %v, want %v", pr.policy.origins, want)
	}
	if want := []string{fmt.Sprintf("start:%d", token)}; !slices.Equal(pr.log, want) {
		t.Fatalf("a Mesh delivery reached the host's invocation: the host was told %v, want %v", pr.log, want)
	}
}

func TestAPostBeforeTheRunIsTakenByItAndOneAfterTheStopIsRefused(t *testing.T) {
	pr := newProbe()
	inbox := pr.engine.Inbox()
	woken := 0
	inbox.WakeWith(func() { woken++ })

	if !inbox.RaiseExternal(probePing, "early") {
		t.Fatalf("the inbox of an engine not yet started refused a post")
	}
	if woken != 1 {
		t.Fatalf("a taken post did not wake the owner: woken %d times", woken)
	}
	if len(pr.policy.taken) != 0 {
		t.Fatalf("a post was applied before the run started: %v", pr.policy.taken)
	}
	// Applied at the first macrostep boundary of the run.
	if err := pr.engine.Initialize(); err != nil {
		t.Fatal(err)
	}
	if want := []string{"ping:early"}; !slices.Equal(pr.policy.taken, want) {
		t.Fatalf("a post made before the run was not taken by it: took %v, want %v", pr.policy.taken, want)
	}

	// Held when the run is stopped: applied as a direct call would be then, onto
	// the queue of a machine that will not dequeue it, which the next Step counts.
	if !inbox.RaiseExternal(probePing, "held") {
		t.Fatalf("a running engine's inbox refused a post")
	}
	pr.engine.Stop()
	if inbox.RaiseExternal(probePing, "late") {
		t.Fatalf("a stopped engine's inbox took a post")
	}
	if woken != 2 {
		t.Fatalf("a refused post woke the owner, or a taken one did not: woken %d times, want 2", woken)
	}
	pr.engine.Step()
	if got := pr.engine.UnseenExternalEvents(); got != 1 {
		t.Fatalf("the post held at the stop was not applied as a direct call is: %d unseen events, want 1", got)
	}
	if event, ok := pr.engine.LastUnseenEvent(); !ok || event != probePing {
		t.Fatalf("the unseen event is not the held ping: %v, %v", event, ok)
	}

	// A run entered again has an open inbox.
	if err := pr.engine.Initialize(); err != nil {
		t.Fatal(err)
	}
	if !inbox.RaiseExternalByName("ping", "again") {
		t.Fatalf("the inbox of a run entered again refused a post")
	}
	pr.engine.Tick()
	if want := []string{"ping:early", "ping:again"}; !slices.Equal(pr.policy.taken, want) {
		t.Fatalf("the run entered again did not take its post: took %v, want %v", pr.policy.taken, want)
	}
}

// An ending naming the machine's own session, posted before its run, is kept for
// the run as every post before it is (docs/adr/0084, D2): the machine holds its
// session from the moment it is made. The run starts `job` and then applies the
// post at its first boundary. A fresh engine's first start carries token 0, the
// token the post names.
func TestAnEndingNamingTheRootPostedBeforeTheRunIsTakenByIt(t *testing.T) {
	pr := newProbe()
	inbox := pr.engine.Inbox()
	const firstToken = 0
	if !postFromAnotherGoroutine(t, inbox, func(in sce.Inbox[probeEvent]) bool {
		return in.EndHostInvoke(pr.policy.sessionID, ending(firstToken, true, "early"))
	}) {
		t.Fatalf("an ending naming the machine's own session (%s) was refused before its run", pr.policy.sessionID)
	}
	if len(pr.policy.taken) != 0 {
		t.Fatalf("a post was applied before the run started: %v", pr.policy.taken)
	}

	if err := pr.engine.Initialize(); err != nil {
		t.Fatal(err)
	}
	if token := pr.latestToken(t); token != firstToken {
		t.Fatalf("the run's first start of `job` carries token %d, not the %d the post named", token, firstToken)
	}
	if want := []string{"done:early"}; !slices.Equal(pr.policy.taken, want) {
		t.Fatalf("the ending posted before the run, naming its session, was not taken by it: took %v, want %v",
			pr.policy.taken, want)
	}
}

// An id the host gives once the engine is made (Engine.SetSessionID) moves the
// machine's entry in its tree's directory: an ending naming the new id, posted
// before the run, is taken by it, and one naming the old id is refused. A running
// session keeps its id, and an id that is no Nmtoken is refused.
func TestAnIDGivenAfterTheEngineIsMadeIsTheOneAPostBeforeTheRunNames(t *testing.T) {
	pr := newProbe()
	inbox := pr.engine.Inbox()
	drawn := pr.policy.sessionID
	const given = "given-root"
	if err := pr.engine.SetSessionID(given); err != nil {
		t.Fatalf("an engine not yet running refused the id %q: %v", given, err)
	}
	if pr.policy.sessionID != given {
		t.Fatalf("the policy holds the id %q, not the %q its host gave", pr.policy.sessionID, given)
	}
	if postFromAnotherGoroutine(t, inbox, func(in sce.Inbox[probeEvent]) bool {
		return in.EndHostInvoke(drawn, ending(0, false, "old"))
	}) {
		t.Fatalf("an ending naming the id the machine was made with (%s) was taken after the host gave it %q",
			drawn, given)
	}
	const firstToken = 0
	if !postFromAnotherGoroutine(t, inbox, func(in sce.Inbox[probeEvent]) bool {
		return in.EndHostInvoke(given, ending(firstToken, true, "given"))
	}) {
		t.Fatalf("an ending naming the id the host gave (%s) was refused before the run", given)
	}

	if err := pr.engine.Initialize(); err != nil {
		t.Fatal(err)
	}
	if token := pr.latestToken(t); token != firstToken {
		t.Fatalf("the run's first start of `job` carries token %d, not the %d the post named", token, firstToken)
	}
	if want := []string{"done:given"}; !slices.Equal(pr.policy.taken, want) {
		t.Fatalf("the ending posted before the run, naming the id the host gave, was not taken by it: "+
			"took %v, want %v", pr.policy.taken, want)
	}

	if err := pr.engine.SetSessionID("another"); err == nil {
		t.Fatalf("a running session took a new id")
	}
	if pr.policy.sessionID != given {
		t.Fatalf("a refused id reached the policy: it holds %q", pr.policy.sessionID)
	}
}

// idlessPolicy is the probe's document behind a policy that keeps no session id:
// embedding the interface promotes its methods alone.
type idlessPolicy struct {
	sce.StatePolicy[probeState, probeEvent]
}

func TestAnIDIsRefusedByAPolicyThatTakesNoneOrForAnIDThatIsNoNmtoken(t *testing.T) {
	idless := sce.NewEngine[probeState, probeEvent](idlessPolicy{&probePolicy{}})
	if err := idless.SetSessionID("given-root"); err == nil {
		t.Fatalf("a policy that takes no session id took one")
	}

	pr := newProbe()
	drawn := pr.policy.sessionID
	if err := pr.engine.SetSessionID("no spaces"); err == nil {
		t.Fatalf("an id that is no Nmtoken was taken")
	}
	if pr.policy.sessionID != drawn {
		t.Fatalf("a refused id reached the policy: it holds %q, not %q", pr.policy.sessionID, drawn)
	}
	// The entry stays under the id the machine was made with.
	if !pr.engine.Inbox().EndHostInvoke(drawn, ending(0, true, "kept")) {
		t.Fatalf("a refused id moved the machine's entry off %q", drawn)
	}
}

func TestAPostHeldWhenTheRunEndsIsAppliedAndOneAfterIsRefused(t *testing.T) {
	pr := newProbe()
	inbox := pr.engine.Inbox()
	// The host posts while `job` is cancelled: from inside the microstep that
	// enters `over`, after the last boundary's drain and before the run's end.
	heldTaken := false
	pr.onCancel = func() { heldTaken = inbox.RaiseExternal(probePing, "at-the-end") }
	if err := pr.engine.Initialize(); err != nil {
		t.Fatal(err)
	}

	pr.engine.RaiseExternal(probeFinish, "")
	pr.engine.Step()
	if !pr.engine.IsInFinalState() {
		t.Fatalf("`finish` did not end the run")
	}
	if !heldTaken {
		t.Fatalf("the inbox refused a post made before the run ended")
	}
	if got := pr.engine.UnseenExternalEvents(); got != 1 {
		t.Fatalf("the post held when the run ended was not applied and counted: %d unseen events, want 1", got)
	}
	if event, ok := pr.engine.LastUnseenEvent(); !ok || event != probePing {
		t.Fatalf("the unseen event is not the held ping: %v, %v", event, ok)
	}
	if len(pr.policy.taken) != 0 {
		t.Fatalf("a run that ended took an event: %v", pr.policy.taken)
	}
	if inbox.RaiseExternal(probePing, "after") {
		t.Fatalf("the inbox of a run that ended took a post")
	}
}

// W3C test 201's shape, without the HTTP listener: inside a macrostep, an event
// arrives from another goroutine while the owner waits for it (the host's
// callback posts from a goroutine it waits for, as a BasicHTTP send waits for the
// listener that posts the event), and the macrostep then sends an event to its
// own session. The posted event arrived first, so it is taken first: the owner
// applies what its inbox holds before it puts its own event on the external
// queue. Waiting for the posting goroutine's answer is the only ordering;
// nothing sleeps.
func TestAnEventPostedBeforeTheMachinesOwnSendIsTakenFirst(t *testing.T) {
	pr := newProbe()
	inbox := pr.engine.Inbox()
	postedTaken := false
	pr.policy.duringCall = func() {
		postedTaken = postFromAnotherGoroutine(t, inbox, func(in sce.Inbox[probeEvent]) bool {
			return in.RaiseExternal(probePing, "posted")
		})
	}
	if err := pr.engine.Initialize(); err != nil {
		t.Fatal(err)
	}

	pr.engine.RaiseExternal(probeCall, "")
	pr.engine.Tick()

	if !postedTaken {
		t.Fatalf("the inbox refused the post the callback made")
	}
	if want := []string{"ping:posted", "ping:own"}; !slices.Equal(pr.policy.taken, want) {
		t.Fatalf("the machine's own send overtook an event that arrived before it: took %v, want %v",
			pr.policy.taken, want)
	}
}

const (
	// treeRoot is the session id the tree cases give their root.
	treeRoot = "fixture-root"
	// treeClock is the standard host type `worker` asks, and clockID the id of
	// its invocation.
	treeClock = "sce-host-clock-now"
	clockID   = "clock"
	// clockNow is a completion of the clock the child's result record reads.
	clockNow = `{"error":"","utc_ms":1700000000000,"monotonic_ms":5}`
	// treeDrives is far above the drives a tree case needs, so a stall fails
	// rather than loops.
	treeDrives = 16
)

type orderEvent = static_invoke_ending_order.StaticInvokeEndingOrderEvent

type orderEngine = sce.Engine[static_invoke_ending_order.StaticInvokeEndingOrderState, orderEvent]

// orderHost is the host of static_invoke_ending_order: it serves the clock
// `worker` asks, recording each start and answering none, and performs
// `beforeQuestion`, the act `ask` makes before it sends `question` to `worker`.
type orderHost struct {
	inbox  sce.Inbox[orderEvent]
	starts []sce.HostInvokeRequest
	// duringQuestion, when set, is what `beforeQuestion` does.
	duringQuestion func()
}

// BeforeQuestion is `<sce:action name="beforeQuestion">`.
func (h *orderHost) BeforeQuestion() {
	if h.duringQuestion != nil {
		h.duringQuestion()
	}
}

// clockEnding is the completion of start, the clock's.
func clockEnding(start sce.HostInvokeRequest) sce.HostInvokeEnding {
	return sce.HostInvokeEnding{ProcessorType: treeClock, InvokeID: clockID, Token: start.Token, Done: true, Data: clockNow}
}

// orderStarted is static_invoke_ending_order started under treeRoot, its clock
// served by host, driven until `worker` has started the clock. It answers the
// root, its policy, and the clock's start.
func orderStarted(t *testing.T, host *orderHost) (*orderEngine, *static_invoke_ending_order.StaticInvokeEndingOrderPolicy, sce.HostInvokeRequest) {
	t.Helper()
	policy := static_invoke_ending_order.NewStaticInvokeEndingOrderPolicy(host)
	if err := policy.SetSessionID(treeRoot); err != nil {
		t.Fatal(err)
	}
	engine := sce.NewEngine[static_invoke_ending_order.StaticInvokeEndingOrderState, orderEvent](&policy)
	engine.SetClock(sce.NewManualClock(0))
	host.inbox = engine.Inbox()
	engine.RegisterInvoker(treeClock, func(event sce.HostInvokeEvent) *sce.HostInvokeResponse {
		if event.Start != nil {
			host.starts = append(host.starts, *event.Start)
		}
		return nil
	})
	if err := engine.Initialize(); err != nil {
		t.Fatal(err)
	}
	for drives := 0; len(host.starts) == 0 && drives < treeDrives; drives++ {
		engine.Tick()
	}
	if len(host.starts) != 1 {
		t.Fatalf("`worker` did not start the clock once: the host was handed %+v", host.starts)
	}
	start := host.starts[0]
	if start.ProcessorType != treeClock || start.InvokeID != clockID || start.SessionID == treeRoot {
		t.Fatalf("the clock's start is not `worker`'s: %+v", start)
	}
	return engine, &policy, start
}

// driveUntilBothHeard ticks engine until the parent has heard `stamped` and
// `answer`, or treeDrives ticks have passed.
func driveUntilBothHeard(engine *orderEngine, policy *static_invoke_ending_order.StaticInvokeEndingOrderPolicy) {
	for drives := 0; (policy.StampedAt() == 0 || policy.AnswerAt() == 0) && drives < treeDrives; drives++ {
		engine.Tick()
	}
}

// docs/adr/0084, D2 as amended at step 10: the host posts the clock's ending,
// naming `worker`, from inside the parent's macrostep (`beforeQuestion`, from a
// goroutine it waits for), and the parent then sends `question` to `worker` in
// that macrostep. The ending was posted first, so `worker` takes
// `done.invoke.clock` first and the parent hears `stamped` before `answer`.
func TestAnEndingPostedForAChildBeforeItsParentSendsToItIsTakenFirst(t *testing.T) {
	host := &orderHost{}
	engine, policy, start := orderStarted(t, host)
	posted, asked := false, false
	host.duringQuestion = func() {
		asked = true
		posted = postFromAnotherGoroutine(t, host.inbox, func(in sce.Inbox[orderEvent]) bool {
			return in.EndHostInvoke(start.SessionID, clockEnding(start))
		})
	}

	engine.RaiseExternalByName("ask", "")
	driveUntilBothHeard(engine, policy)

	if !asked {
		t.Fatalf("`ask` never performed `beforeQuestion`")
	}
	if !posted {
		t.Fatalf("the inbox refused the ending the host posted for `worker` (%s)", start.SessionID)
	}
	if policy.StampedAt() != 1 || policy.AnswerAt() != 2 {
		t.Fatalf("`worker` took the question its parent sent after the clock's ending was posted before "+
			"that ending: the parent heard `stamped` as number %d and `answer` as number %d, want 1 and 2",
			policy.StampedAt(), policy.AnswerAt())
	}
}

// A post to a child's inbox wakes the tree's one owner through the waker the host
// installed on the root, and the owner's usual drive (Tick) applies it: the
// parent hears `stamped` after one tick.
func TestAPostToAChildWakesTheTreesOwnerAndItsNextTickAppliesIt(t *testing.T) {
	host := &orderHost{}
	engine, policy, start := orderStarted(t, host)
	var woken atomic.Int32
	host.inbox.WakeWith(func() { woken.Add(1) })

	if !postFromAnotherGoroutine(t, host.inbox, func(in sce.Inbox[orderEvent]) bool {
		return in.EndHostInvoke(start.SessionID, clockEnding(start))
	}) {
		t.Fatalf("the inbox refused the ending the host posted for `worker` (%s)", start.SessionID)
	}
	if got := woken.Load(); got != 1 {
		t.Fatalf("a post to `worker`'s inbox did not wake the tree's owner once: woken %d times", got)
	}
	engine.Tick()
	if policy.StampedAt() != 1 || policy.AnswerAt() != 0 {
		t.Fatalf("the tick after the wake did not apply the ending posted for `worker`: `stamped` heard as "+
			"number %d and `answer` as number %d, want 1 and 0", policy.StampedAt(), policy.AnswerAt())
	}
}

// A post naming a session the tree does not hold is refused, and its answer says
// so; nothing is raised.
func TestAPostNamingASessionTheTreeDoesNotHoldIsRefused(t *testing.T) {
	host := &orderHost{}
	engine, policy, start := orderStarted(t, host)
	for _, nowhere := range []string{treeRoot + ".nowhere", start.SessionID + ".nowhere", "elsewhere"} {
		if postFromAnotherGoroutine(t, host.inbox, func(in sce.Inbox[orderEvent]) bool {
			return in.EndHostInvoke(nowhere, clockEnding(start))
		}) {
			t.Errorf("an ending naming %q, a session the tree does not hold, was taken", nowhere)
		}
	}
	engine.Tick()
	if policy.StampedAt() != 0 {
		t.Fatalf("a refused ending reached `worker`: `stamped` heard as number %d", policy.StampedAt())
	}
}

// A child whose run ended leaves the tree's directory: `rest` leaves `working`,
// which stops `worker`, and a post naming it is refused from then on, where it
// was taken while `worker` ran.
func TestAPostNamingAChildThatHasEndedIsRefused(t *testing.T) {
	policy := static_invoke_tree.NewStaticInvokeTreePolicy()
	if err := policy.SetSessionID(treeRoot); err != nil {
		t.Fatal(err)
	}
	engine := sce.NewEngine[static_invoke_tree.StaticInvokeTreeState, static_invoke_tree.StaticInvokeTreeEvent](&policy)
	engine.SetClock(sce.NewManualClock(0))
	var starts []sce.HostInvokeRequest
	engine.RegisterInvoker(treeClock, func(event sce.HostInvokeEvent) *sce.HostInvokeResponse {
		if event.Start != nil {
			starts = append(starts, *event.Start)
		}
		return nil
	})
	inbox := engine.Inbox()
	if err := engine.Initialize(); err != nil {
		t.Fatal(err)
	}
	// `go` is forwarded to `worker`, which enters `second` and starts the clock.
	engine.RaiseExternalByName("go", "")
	for drives := 0; len(starts) == 0 && drives < treeDrives; drives++ {
		engine.Tick()
	}
	if len(starts) != 1 {
		t.Fatalf("`worker` did not start the clock once: the host was handed %+v", starts)
	}
	start := starts[0]
	// A stale token: taken by the running `worker`'s inbox, refused by its token
	// check when applied, so the clock keeps running.
	stale := clockEnding(start)
	stale.Token = start.Token + 1
	if !postFromAnotherGoroutine(t, inbox, func(in sce.Inbox[static_invoke_tree.StaticInvokeTreeEvent]) bool {
		return in.EndHostInvoke(start.SessionID, stale)
	}) {
		t.Fatalf("the inbox of `worker` (%s) refused a post while it ran", start.SessionID)
	}

	engine.RaiseExternalByName("rest", "")
	engine.Tick()
	if postFromAnotherGoroutine(t, inbox, func(in sce.Inbox[static_invoke_tree.StaticInvokeTreeEvent]) bool {
		return in.EndHostInvoke(start.SessionID, clockEnding(start))
	}) {
		t.Fatalf("an ending naming `worker` (%s) was taken after `rest` stopped it", start.SessionID)
	}
	if got := policy.Stamps(); got != 0 {
		t.Fatalf("an ending reached `worker` after it ended, or a stale one before: stamps is %d", got)
	}
}
