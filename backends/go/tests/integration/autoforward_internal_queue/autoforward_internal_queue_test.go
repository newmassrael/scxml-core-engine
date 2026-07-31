// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4 autoforward skips internal-queue events — Go AOT path.
//
// Appendix D's `mainEventLoop` forwards only what it dequeues from the
// external queue; the internal drain above it has no forwarding step at
// all. §6.2 raises `error.execution` onto the internal queue when `<send>`
// names an unsupported type, so it must never reach an `autoforward`
// child — and it must be excluded by where it was raised, not by a filter
// that recognises its name.
//
// Sibling of `autoforward_done_invoke`, which pins the positive half.
//
// Fixture: integration_resources/autoforward_internal_queue/autoforward_internal_queue.scxml
//
// Regeneration (after fixture or template edit):
//   scripts/regen_autoforward_internal_queue_go.sh

package autoforward_internal_queue

import (
	"testing"
	"time"

	sce "github.com/newmassrael/sce-go-runtime"
)

func TestAnInternalQueueEventIsNeverAutoforwarded(t *testing.T) {
	policy := NewAutoforwardInternalQueuePolicy()
	policy.SessionID = sce.GenerateSessionID()
	engine := sce.NewEngine[AutoforwardInternalQueueState, AutoforwardInternalQueueEvent](&policy)
	engine.Initialize()

	completed := engine.RunUntilCompletion(2*time.Second, 10*time.Millisecond)
	if !completed {
		t.Fatalf("autoforward_internal_queue timed out before reaching a final state — the " +
			"watcher child reported neither verdict, so neither `error.execution` nor " +
			"`probe` reached it")
	}

	got := engine.GetCurrentState()
	if got != AutoforwardInternalQueueStatePass {
		t.Fatalf(
			"parent reached %v, want Pass: the watcher saw `error.execution`, so an "+
				"internal-queue event was autoforwarded. W3C Appendix D `mainEventLoop` "+
				"forwards only what it dequeues from the external queue, and §6.2 raises "+
				"`error.execution` onto the internal one — check that the event was not "+
				"routed onto the external queue for some unrelated reason, which would "+
				"leak it past any name-blind forward.",
			got,
		)
	}
}
