// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.2.5 — Go compile+run gate for a `<send type>` the HOST serves.
//
// The clause makes the Event I/O Processor identifier extensible, so the set is
// open by design. SCE implemented two of them and refused everything else with
// `error.execution`; nothing let a platform widen the set. Rust, C++ and C11
// grew a registry first, and this backend refused the declaration by name until
// it grew one of its own — the refusal being honest is exactly what made the
// gap a coverage debt rather than a silent drop.
//
// The committed machine beside this file is generated from
// `sce-build/tests/fixtures/host_processor/statechart_host_processor.scxml`
// WITH the declaration (regen: `scripts/regen_host_processor.sh`), the same
// document the Rust, C++ and C11 channels drive. Because the tree is part of
// the module it is really compiled; these tests drive that one machine several
// times, so what they measure is the registration and not the build.
//
// The pair at the top is the whole contract:
//
//   * a registered handler receives the send and its reply arrives as an
//     event — the feature working;
//   * the same machine with nothing registered raises `error.execution` —
//     a wiring mistake staying visible instead of reading as success.
//
// Both are needed. A gate holding only the first would pass on an engine that
// dispatched to nothing and called it delivered, which is the silence being
// repaid.

package statechart_host_processor

import (
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
)

// The type the fixture was compiled for. `scripts/regen_host_processor.sh`
// passes this same string to `--host-processor`; a test registering a different
// one would measure nothing and pass, so the `refused` counter is asserted
// rather than the registration trusted.
const declaredType = "x-sce-host"

type started struct {
	engine *sce.Engine[StatechartHostProcessorState, StatechartHostProcessorEvent]
	policy *StatechartHostProcessorPolicy
}

func newStarted() started {
	policy := NewStatechartHostProcessorPolicy()
	policy.SessionID = sce.GenerateSessionID()
	policy.ScriptEngine = scegotest.NewLuaEngine()
	engine := sce.NewEngine[StatechartHostProcessorState, StatechartHostProcessorEvent](&policy)
	return started{engine: engine, policy: &policy}
}

// The fixture's `<assign>`s are the only witness: every outcome here leaves the
// machine in the same single state, so the configuration cannot tell them
// apart.
func (s started) counter(t *testing.T, name string) int64 {
	t.Helper()
	value, ok := sce.ReadDatamodelInt(s.policy.ScriptEngine, s.policy.SessionID, name)
	if !ok {
		t.Fatalf("the fixture declares `%s` in its datamodel and the machine could not read it", name)
	}
	return value
}

func TestARegisteredHandlerReceivesTheSendAndItsReplyArrives(t *testing.T) {
	s := newStarted()
	var seen []sce.HostSendRequest
	s.engine.RegisterEventProcessor(declaredType, func(req sce.HostSendRequest) []sce.HostSendResponse {
		seen = append(seen, req)
		// The request/reply shape: the reply becomes an event the document was
		// already waiting for, which is what lets a state DECLARE an act
		// instead of a host-side table performing it.
		return []sce.HostSendResponse{{EventName: "turn.done"}}
	})
	s.engine.Initialize()
	s.engine.Step()

	if got := s.counter(t, "served"); got != 1 {
		t.Fatalf("the handler's reply never reached the document: served = %d", got)
	}
	if got := s.counter(t, "refused"); got != 0 {
		t.Fatalf("a served send also raised error.execution: refused = %d", got)
	}
	// The false-positive guard: an ordinary `<send>` in the same block must
	// still deliver. Without it a change that broke every send while leaving
	// the host branch intact would read as a pass.
	if got := s.counter(t, "plain"); got != 1 {
		t.Fatalf("an ordinary <send> in the same block stopped delivering: plain = %d", got)
	}

	if len(seen) != 1 {
		t.Fatalf("the handler ran %d times", len(seen))
	}
	req := seen[0]
	if req.ProcessorType != declaredType {
		t.Fatalf("handler saw type %q, expected %q", req.ProcessorType, declaredType)
	}
	if req.EventName != "watch.turn" {
		t.Fatalf("handler saw event %q, expected \"watch.turn\"", req.EventName)
	}
	// The payload the author wrote has to survive the crossing, or the document
	// can name an act but not parameterise it — which is most of the reason to
	// move an act into the document at all.
	if values := req.Params["within"]; len(values) != 1 || values[0] != "2500" {
		t.Fatalf("the <param> did not reach the handler: %v", req.Params)
	}
	// §scxml-6.2.4: correlating a reply, or honouring a `<cancel>`, needs the
	// send id — auto-generated here because the fixture declares none.
	if req.SendID == "" {
		t.Fatal("the request carried no send id")
	}
}

// The other half, and the one that keeps the repair honest: the build declared
// the type so codegen emitted a dispatch, but nothing was registered, so nobody
// performed the act.
func TestADeclaredTypeWithNoHandlerStillRaisesErrorExecution(t *testing.T) {
	s := newStarted()
	s.engine.Initialize()
	s.engine.Step()

	if got := s.counter(t, "refused"); got != 1 {
		t.Fatalf("an unregistered processor was silently treated as served: refused = %d", got)
	}
	if got := s.counter(t, "served"); got != 0 {
		t.Fatalf("served = %d with nothing registered", got)
	}
}

// A handler may perform work and have nothing to say. That is not an error, and
// reporting it as one would cost every fire-and-forget act a spurious
// `error.execution`.
func TestAHandlerThatAnswersNothingIsNotAnError(t *testing.T) {
	for _, reply := range []struct {
		name   string
		answer []sce.HostSendResponse
	}{
		{"empty slice", []sce.HostSendResponse{}},
		// Go's zero value for a slice, which a handler that simply declares its
		// return and never appends will produce. Distinguished from the empty
		// slice because a `len()` check treats them alike and a `== nil` check
		// does not — and the engine must not be reading the difference.
		{"nil slice", nil},
	} {
		t.Run(reply.name, func(t *testing.T) {
			s := newStarted()
			ran := false
			s.engine.RegisterEventProcessor(declaredType, func(sce.HostSendRequest) []sce.HostSendResponse {
				ran = true
				return reply.answer
			})
			s.engine.Initialize()
			s.engine.Step()

			if !ran {
				t.Fatal("the handler never ran")
			}
			if got := s.counter(t, "refused"); got != 0 {
				t.Fatalf("a silent handler was reported as an unsupported processor: refused = %d", got)
			}
			if got := s.counter(t, "served"); got != 0 {
				t.Fatalf("no reply was sent, so no reply event should have arrived: served = %d", got)
			}
		})
	}
}

// Registering some other type does not serve this one. The registry is keyed,
// and a lookup that fell back to "any handler" would deliver a document's acts
// to a processor it never named.
func TestAHandlerRegisteredForAnotherTypeDoesNotServeThisOne(t *testing.T) {
	s := newStarted()
	s.engine.RegisterEventProcessor("x-some-other-host", func(sce.HostSendRequest) []sce.HostSendResponse {
		return []sce.HostSendResponse{{EventName: "turn.done"}}
	})
	s.engine.Initialize()
	s.engine.Step()

	if got := s.counter(t, "served"); got != 0 {
		t.Fatalf("a handler for a different type answered this send: served = %d", got)
	}
	if got := s.counter(t, "refused"); got != 1 {
		t.Fatalf("refused = %d", got)
	}
}

// A reply may name an event this machine does not declare — a host serving
// several documents, or one that has moved on since. That is dropped, exactly
// as any undeclared event reaching the queue is, and it is not an error.
func TestAReplyNamingAnUndeclaredEventIsDropped(t *testing.T) {
	s := newStarted()
	s.engine.RegisterEventProcessor(declaredType, func(sce.HostSendRequest) []sce.HostSendResponse {
		return []sce.HostSendResponse{{EventName: "turn.never.declared"}}
	})
	s.engine.Initialize()
	s.engine.Step()

	if got := s.counter(t, "served"); got != 0 {
		t.Fatalf("an undeclared reply name reached a transition: served = %d", got)
	}
	if got := s.counter(t, "refused"); got != 0 {
		t.Fatalf("a dropped reply was reported as a refusal: refused = %d", got)
	}
	if got := s.counter(t, "plain"); got != 1 {
		t.Fatalf("the machine stopped running after an unknown reply name: plain = %d", got)
	}
}

// The query the generated send site uses to tell "ran and said nothing" from
// "was never wired up". Both give the same answer from the dispatch, and only
// the second is an error, so the distinction cannot come from the return value
// alone.
func TestTheRegistryReportsWhatItHolds(t *testing.T) {
	s := newStarted()
	if s.engine.HasEventProcessor(declaredType) {
		t.Fatal("an unregistered type reads as present")
	}
	s.engine.RegisterEventProcessor(declaredType, func(sce.HostSendRequest) []sce.HostSendResponse {
		return nil
	})
	if !s.engine.HasEventProcessor(declaredType) {
		t.Fatal("the registered type reads as absent")
	}
	if s.engine.HasEventProcessor("x-never-registered") {
		t.Fatal("an unregistered type reads as present")
	}
}

// Registering a type twice replaces. Appending would leave dispatch depending
// on registration order, and a host re-registering means to change what serves
// the act — not to add a second server whose turn may never come.
func TestRegisteringATypeTwiceReplaces(t *testing.T) {
	s := newStarted()
	supersededRan := false
	currentRan := false
	s.engine.RegisterEventProcessor(declaredType, func(sce.HostSendRequest) []sce.HostSendResponse {
		supersededRan = true
		return nil
	})
	s.engine.RegisterEventProcessor(declaredType, func(sce.HostSendRequest) []sce.HostSendResponse {
		currentRan = true
		return []sce.HostSendResponse{{EventName: "turn.done"}}
	})
	s.engine.Initialize()
	s.engine.Step()

	if supersededRan {
		t.Fatal("the superseded handler still served the act")
	}
	if !currentRan {
		t.Fatal("the current handler never ran")
	}
	if got := s.counter(t, "served"); got != 1 {
		t.Fatalf("served = %d", got)
	}
}
