// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4 autoforward happens at the external dequeue — Go AOT path.
//
// Appendix D's `mainEventLoop` forwards one statement after
// `externalQueue.dequeue()` and before `selectTransitions`, and §6.4.2 says
// the same in prose: the parent forwards "at the point at which it removes it
// from the external event queue". Forwarding where the event is queued
// instead breaks run-to-completion — the child sees event N before the parent
// has processed 1..N-1.
//
// Siblings `autoforward_done_invoke` and `autoforward_internal_queue` pin
// which events are forwarded and are deliberately blind to when; this one
// pins the position and nothing else.
//
// Fixture: integration_resources/autoforward_dequeue_point/autoforward_dequeue_point.scxml
//
// Regeneration (after fixture or template edit):
//   scripts/regen_autoforward_dequeue_point_go.sh

package autoforward_dequeue_point

import (
	"testing"
	"time"

	sce "github.com/newmassrael/sce-go-runtime"
)

func TestAnExternalEventIsForwardedAtTheDequeueNotTheEnqueue(t *testing.T) {
	policy := NewAutoforwardDequeuePointPolicy()
	policy.SessionID = sce.GenerateSessionID()
	engine := sce.NewEngine[AutoforwardDequeuePointState, AutoforwardDequeuePointEvent](&policy)
	engine.Initialize()

	completed := engine.RunUntilCompletion(2*time.Second, 10*time.Millisecond)
	if !completed {
		t.Fatalf("autoforward_dequeue_point timed out before reaching a final state — " +
			"the probe child reported neither verdict, so `second` never reached it")
	}

	got := engine.GetCurrentState()
	if got != AutoforwardDequeuePointStatePass {
		t.Fatalf(
			"parent reached %v, want Pass: the probe child saw `second` before `mark`, "+
				"so both events were handed over while the parent was still executing "+
				"the transition that queued them. W3C Appendix D `mainEventLoop` "+
				"forwards one statement after `externalQueue.dequeue()`, and §6.4.2 puts "+
				"it \"at the point at which it removes it from the external event "+
				"queue\" — forwarding at the enqueue lets the child run ahead of the "+
				"parent by a whole event.",
			got,
		)
	}
}
