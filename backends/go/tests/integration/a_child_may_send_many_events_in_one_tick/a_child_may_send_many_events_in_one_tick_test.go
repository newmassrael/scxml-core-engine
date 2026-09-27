// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// W3C SCXML 6.4: every event an invoked child sends to `#_parent` reaches
// the parent, however many it sends in one tick — Go AOT path.
//
// Measured 2026-09-27, this channel wrote a child's parent-bound events into
// a channel buffered at 100 with a blocking send, on the parent's own
// goroutine, so the 101st event of one tick blocked forever. The test is
// bounded by a deadline so that shape fails rather than hangs.
//
// Fixture: integration_resources/a_child_may_send_many_events_in_one_tick/a_child_may_send_many_events_in_one_tick.scxml
// (canonical, shared with the C++ / C11 / Rust / Kotlin / Python channels).
//
// Regeneration: scripts/regen_a_child_may_send_many_events_in_one_tick_go.sh

package a_child_may_send_many_events_in_one_tick

import (
	"fmt"
	"testing"
	"time"

	sce "github.com/newmassrael/sce-go-runtime"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
)

func record(v int64, ok bool) string {
	if !ok {
		return "<unreadable>"
	}
	return fmt.Sprint(v)
}

func TestEveryEventTheChildSentArrives(t *testing.T) {
	policy := NewAChildMaySendManyEventsInOneTickPolicy()
	policy.SessionID = sce.GenerateSessionID()
	// The handler records with <assign>, so this is an ECMAScript-datamodel
	// machine.
	policy.ScriptEngine = scegotest.NewLuaEngine()
	engine := sce.NewEngine[AChildMaySendManyEventsInOneTickState, AChildMaySendManyEventsInOneTickEvent](&policy)

	finished := make(chan struct{})
	go func() {
		engine.Initialize()
		engine.RaiseExternal(AChildMaySendManyEventsInOneTickEventFinish, "", "")
		engine.Step()
		close(finished)
	}()
	select {
	case <-finished:
	case <-time.After(10 * time.Second):
		t.Fatal("the engine did not return: a child that sends many events in one tick must not block it")
	}

	seen := fmt.Sprintf("ticks=%s (wanted 110)", record(policy.Ticks()))
	if ended, ok := engine.TerminalState(); !ok || ended != AChildMaySendManyEventsInOneTickStateDone {
		t.Errorf("`finish` must carry the run to `done` (%s)", seen)
	}
	if v, ok := policy.Ticks(); !ok || v != 110 {
		t.Errorf("every event the child sent arrives before `finish` (%s)", seen)
	}
}
