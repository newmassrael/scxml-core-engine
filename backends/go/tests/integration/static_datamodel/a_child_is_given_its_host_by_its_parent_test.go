// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// SCE Accepted Subset §2.15, "Child sessions" (docs/adr/0005, decision 6): a
// child that declares `<sce:action>`s takes the host that performs them when it
// is built, because its first `<onentry>` can already perform an act — a host
// installed afterwards would arrive one act too late. So the host has to exist
// when the invocation starts, and the parent obtains it from its own host, which
// answers one for the child each time the invocation starts.
//
// `static_child_host.scxml` invokes `worker`, which announces itself on entry
// (`started`) and reports its steps when it ends (`finished`). The parent
// declares no act: its host is there for the child alone.
// `static_child_host_hybrid.scxml` invokes whichever of two candidates its
// `srcexpr` names, each with an act of its own, and the host answered is the one
// for THAT candidate. Go has no saved state, so no restore is read here; the
// Kotlin and Python halves are `AChildIsGivenItsHostByItsParentTest.kt` and
// `test_a_child_is_given_its_host_by_its_parent.py`.

package static_datamodel

import (
	"fmt"
	"reflect"
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"

	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_child_host"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_child_host_hybrid"
)

// workerHost is the host a `worker` child performs its acts through; it keeps
// what it was asked.
type workerHost struct{ calls []string }

func (h *workerHost) Started() { h.calls = append(h.calls, "started") }
func (h *workerHost) Finished(steps uint32) {
	h.calls = append(h.calls, fmt.Sprintf("finished(%d)", steps))
}

// parentHost is the parent's host: it answers a fresh child host for each
// question and keeps every one, in the order asked.
type parentHost struct {
	asked   int
	workers []*workerHost
}

func (h *parentHost) ActionsForWorker() static_child_host.StaticChildHostSceSynthInvokeWorkerActions {
	h.asked++
	worker := &workerHost{}
	h.workers = append(h.workers, worker)
	return worker
}

func startChildHost(host *parentHost) (*static_child_host.StaticChildHostPolicy, *sce.Engine[static_child_host.StaticChildHostState, static_child_host.StaticChildHostEvent]) {
	policy := static_child_host.NewStaticChildHostPolicy(host)
	policy.SessionID = sce.GenerateSessionID()
	engine := sce.NewEngine[static_child_host.StaticChildHostState, static_child_host.StaticChildHostEvent](&policy)
	engine.Initialize()
	for i := 0; i < 40; i++ {
		engine.Tick()
	}
	return &policy, engine
}

// send hands the parent an event, which it forwards to its child, and lets the
// child run.
func sendToChildHost(engine *sce.Engine[static_child_host.StaticChildHostState, static_child_host.StaticChildHostEvent], name string) {
	engine.RaiseExternalByName(name, "")
	for i := 0; i < 40; i++ {
		engine.Tick()
	}
}

func TestTheChildPerformsItsFirstActThroughTheHostItsParentAnswered(t *testing.T) {
	host := &parentHost{}
	startChildHost(host)
	if host.asked != 1 {
		t.Fatalf("the parent's host was asked %d times, not once", host.asked)
	}
	// The act of its first `<onentry>` is already performed: the host was there
	// when the child was built, not installed after.
	if want := []string{"started"}; !reflect.DeepEqual(host.workers[0].calls, want) {
		t.Errorf("the child asked its host %v, want %v", host.workers[0].calls, want)
	}
}

func TestTheChildReportsWhatItDidThroughTheSameHost(t *testing.T) {
	host := &parentHost{}
	policy, engine := startChildHost(host)
	sendToChildHost(engine, "a")
	sendToChildHost(engine, "b")
	if got := policy.Completed(); got != 1 {
		t.Errorf("completed is %d, not 1: the child ended and the parent counted it", got)
	}
	if want := []string{"started", "finished(2)"}; !reflect.DeepEqual(host.workers[0].calls, want) {
		t.Errorf("the child asked its host %v, want %v", host.workers[0].calls, want)
	}
	if host.asked != 1 {
		t.Errorf("nothing should ask the parent's host again, it was asked %d times", host.asked)
	}
}

func TestAStateInvokedAgainIsGivenAHostOfItsOwn(t *testing.T) {
	host := &parentHost{}
	_, engine := startChildHost(host)
	for _, name := range []string{"a", "b", "again", "back"} {
		sendToChildHost(engine, name)
	}
	if host.asked != 2 {
		t.Fatalf("the parent's host was asked %d times, not once per start", host.asked)
	}
	if host.workers[0] == host.workers[1] {
		t.Fatal("the second start was given the first child's host")
	}
	// The first child's run is its own, and the second starts from nothing.
	if want := []string{"started", "finished(2)"}; !reflect.DeepEqual(host.workers[0].calls, want) {
		t.Errorf("the first child asked its host %v, want %v", host.workers[0].calls, want)
	}
	if want := []string{"started"}; !reflect.DeepEqual(host.workers[1].calls, want) {
		t.Errorf("the second child asked its host %v, want %v", host.workers[1].calls, want)
	}
}

// firstHost and secondHost are the hosts of the two candidates of
// `static_child_host_hybrid`, each with an act of its own.
type firstHost struct{ calls []string }

func (h *firstHost) FirstRan() { h.calls = append(h.calls, "first_ran") }

type secondHost struct{ calls []string }

func (h *secondHost) SecondRan() { h.calls = append(h.calls, "second_ran") }

// hybridParentHost answers the host for the candidate it is asked about, and
// records which candidate that was.
type hybridParentHost struct {
	asked   []string
	firsts  []*firstHost
	seconds []*secondHost
}

func (h *hybridParentHost) ActionsForWorkStaticHostedFirst() static_child_host_hybrid.StaticHostedFirstActions {
	h.asked = append(h.asked, "first")
	host := &firstHost{}
	h.firsts = append(h.firsts, host)
	return host
}

func (h *hybridParentHost) ActionsForWorkStaticHostedSecond() static_child_host_hybrid.StaticHostedSecondActions {
	h.asked = append(h.asked, "second")
	host := &secondHost{}
	h.seconds = append(h.seconds, host)
	return host
}

func TestACandidateIsGivenTheHostAnsweredForItAndNoOther(t *testing.T) {
	host := &hybridParentHost{}
	policy := static_child_host_hybrid.NewStaticChildHostHybridPolicy(host)
	policy.SessionID = sce.GenerateSessionID()
	engine := sce.NewEngine[static_child_host_hybrid.StaticChildHostHybridState, static_child_host_hybrid.StaticChildHostHybridEvent](&policy)
	engine.Initialize()
	tick := func() {
		for i := 0; i < 40; i++ {
			engine.Tick()
		}
	}
	tick()

	if want := []string{"first"}; !reflect.DeepEqual(host.asked, want) {
		t.Fatalf("the parent's host was asked %v, want %v", host.asked, want)
	}
	if want := []string{"first_ran"}; !reflect.DeepEqual(host.firsts[0].calls, want) {
		t.Errorf("the first candidate asked its host %v, want %v", host.firsts[0].calls, want)
	}
	if got := policy.Completed(); got != 1 {
		t.Errorf("completed is %d, not 1: it ran and ended", got)
	}

	for _, name := range []string{"again", "back"} {
		engine.RaiseExternalByName(name, "")
		tick()
	}
	if want := []string{"first", "second"}; !reflect.DeepEqual(host.asked, want) {
		t.Fatalf("the parent's host was asked %v, want %v", host.asked, want)
	}
	if want := []string{"second_ran"}; !reflect.DeepEqual(host.seconds[0].calls, want) {
		t.Errorf("the second candidate asked its host %v, want %v", host.seconds[0].calls, want)
	}
	// The first candidate's run is its own and was not repeated.
	if want := []string{"first_ran"}; !reflect.DeepEqual(host.firsts[0].calls, want) {
		t.Errorf("the first candidate asked its host %v, want %v", host.firsts[0].calls, want)
	}
	if got := policy.Completed(); got != 2 {
		t.Errorf("completed is %d, not 2", got)
	}
}
