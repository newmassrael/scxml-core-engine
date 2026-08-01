// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML Appendix D: pending invokes start before the external dequeue — Go AOT path.
//
// `mainEventLoop` completes the macrostep on eventless and internal
// transitions alone, then runs `invoke(inv)` for every state entered on the
// last iteration, and only then reaches `externalQueue.dequeue()`. The
// external queue is named exactly once in that loop and it is after the
// invokes.
//
// An engine that folds the external drain into its macrostep completion loop
// consumes whatever `<onentry>` queued for the parent itself while the invoked
// children do not yet exist, so an autoforward child misses every event the
// parent queued on the way in. That is a lost event, not a reordered one.
//
// The sibling `autoforward_dequeue_point` pins where in the loop the forward
// sits and is deliberately blind to this axis: there the child opens the
// exchange, so it is alive before anything is queued. Here the parent queues
// first and the child starts second.
//
// Fixture: integration_resources/invoke_precedes_external_dequeue/invoke_precedes_external_dequeue.scxml
//
// Regeneration (after fixture or template edit):
//   scripts/regen_invoke_precedes_external_dequeue_go.sh

package invoke_precedes_external_dequeue

import (
	"testing"
	"time"

	sce "github.com/newmassrael/sce-go-runtime"
)

func TestPendingInvokesStartBeforeTheExternalDequeue(t *testing.T) {
	policy := NewInvokePrecedesExternalDequeuePolicy()
	policy.SessionID = sce.GenerateSessionID()
	engine := sce.NewEngine[InvokePrecedesExternalDequeueState, InvokePrecedesExternalDequeueEvent](&policy)
	engine.Initialize()

	completed := engine.RunUntilCompletion(2*time.Second, 10*time.Millisecond)
	if !completed {
		t.Fatalf("invoke_precedes_external_dequeue timed out before reaching a final state — " +
			"the watching child answered neither verdict, so `probe` never reached it")
	}

	got := engine.GetCurrentState()
	if got != InvokePrecedesExternalDequeueStatePass {
		t.Fatalf(
			"parent reached %v, want Pass: the watching child answered `probe` from "+
				"`waiting`, so it never saw `kick`. The parent drained its external "+
				"queue before starting the invoke, and the event `<onentry>` had "+
				"queued for itself was consumed while no child existed. W3C Appendix D "+
				"`mainEventLoop` runs `invoke(inv)` for every state entered on the last "+
				"iteration before it reaches `externalQueue.dequeue()`, so an "+
				"autoforward child is live for the whole external queue.",
			got,
		)
	}
}
