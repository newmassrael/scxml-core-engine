// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4: autoforward is owed to the external event, not to the door it
// came through — Go AOT path.
//
// The four sibling `autoforward_*` stems all let the machine forward events it
// queued for itself. This one hands it one from outside, through the engine's
// own "here is an event" entry point, and asks whether the `autoforward` child
// sees it. Appendix D's `mainEventLoop` binds the preliminary step
// (`applyFinalize` + the autoforward `send`) to the external event it is about
// to select transitions for, so an engine with a second door has to run the
// step at both or the child goes blind to everything the host delivers.
//
// Measured 2026-08-21: the C++ AOT engine had the step written inline in its
// queue drain, so `processEvent()` skipped it. This engine's ProcessEvent
// raises onto the external queue and steps, so the drain is its only door and
// the fixture pins that — a later ProcessEvent that hands the event straight
// to the transition selector would go red here.
//
// Fixture: integration_resources/host_event_reaches_the_child/host_event_reaches_the_child.scxml
//
// Regeneration (after fixture or template edit):
//   scripts/regen_host_event_reaches_the_child_go.sh

package host_event_reaches_the_child

import (
	"testing"
	"time"

	sce "github.com/newmassrael/sce-go-runtime"
)

func TestAnEventTheHostHandsOverReachesTheAutoforwardChild(t *testing.T) {
	policy := NewHostEventReachesTheChildPolicy()
	policy.SessionID = sce.GenerateSessionID()
	engine := sce.NewEngine[HostEventReachesTheChildState, HostEventReachesTheChildEvent](&policy)
	engine.Initialize()

	// The child opens the exchange, so drive until its `ready` has moved the
	// parent into `armed` — the one state that can be handed an event from
	// outside. Bounded rather than timed: every tick here is the machine's own
	// work, so a machine that has not arrived after this many is not slow, it
	// is not going to.
	for i := 0; i < 50 && engine.GetCurrentState() != HostEventReachesTheChildStateArmed; i++ {
		engine.Tick()
	}
	if got := engine.GetCurrentState(); got != HostEventReachesTheChildStateArmed {
		t.Fatalf("parent parked in %v, want Armed: the probe child never sent `ready`, so the "+
			"fixture never reached the state where a host event can be handed over — this is "+
			"a broken handshake, not a forwarding verdict", got)
	}

	// The axis: the host's own entry point, not RaiseExternal + Tick.
	engine.ProcessEvent(HostEventReachesTheChildEventHostPing)

	completed := engine.RunUntilCompletion(2*time.Second, 10*time.Millisecond)
	if !completed {
		t.Fatalf("host_event_reaches_the_child timed out before reaching a final state — " +
			"the probe child answered neither verdict, so neither `hostPing` nor `marker` " +
			"reached it")
	}

	got := engine.GetCurrentState()
	if got != HostEventReachesTheChildStatePass {
		t.Fatalf(
			"parent reached %v, want Pass: the probe child answered `sawMarkerOnly`, so the "+
				"event the host handed to ProcessEvent was never forwarded to it — the child "+
				"only ever saw the `marker` the parent's own transition body sent. W3C "+
				"Appendix D `mainEventLoop` runs the autoforward `send` against the external "+
				"event before it selects transitions for it, whichever door the event arrived "+
				"through, so an engine that runs that step only in its queue drain leaves an "+
				"`autoforward` child blind to everything its host delivers.",
			got,
		)
	}
}
