// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4 autoforward carries `done.invoke.<id>` — Go AOT path.
//
// Appendix D's `mainEventLoop` forwards every event it dequeues from the
// external queue to each `autoforward` child without testing the event's
// name; the sole exclusion is the cancel event, expressed as control flow.
// §6.4.2 places `done.invoke.<id>` on the external queue of the invoking
// session, so a sibling child that is still running must receive it.
//
// Fixture: integration_resources/autoforward_done_invoke/autoforward_done_invoke.scxml
// (canonical, shared with the C++ / C11 / Rust / Kotlin / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_autoforward_done_invoke_go.sh

package autoforward_done_invoke

import (
	"testing"
	"time"

	sce "github.com/newmassrael/sce-go-runtime"
)

func TestDoneInvokeFromASiblingReachesTheAutoforwardChild(t *testing.T) {
	policy := NewAutoforwardDoneInvokePolicy()
	policy.SessionID = sce.GenerateSessionID()
	engine := sce.NewEngine[AutoforwardDoneInvokeState, AutoforwardDoneInvokeEvent](&policy)
	engine.Initialize()

	completed := engine.RunUntilCompletion(2*time.Second, 10*time.Millisecond)
	if !completed {
		t.Fatalf("autoforward_done_invoke timed out before reaching a final state — the " +
			"watcher child reported neither verdict, so `done.invoke.inv_short` never " +
			"reached the parent's external queue at all")
	}

	got := engine.GetCurrentState()
	if got != AutoforwardDoneInvokeStatePass {
		t.Fatalf(
			"parent reached %v, want Pass: the watcher saw only `probe`, so "+
				"`done.invoke.inv_short` was withheld from a live `autoforward` child. "+
				"W3C Appendix D `mainEventLoop` forwards every event dequeued from the "+
				"external queue and excludes only the cancel event, and §6.4.2 places "+
				"`done.invoke.<id>` on that queue — so no name-based platform-event "+
				"filter belongs on the forwarding path.",
			got,
		)
	}
}
