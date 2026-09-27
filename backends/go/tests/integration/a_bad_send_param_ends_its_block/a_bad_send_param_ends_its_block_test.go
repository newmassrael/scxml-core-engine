// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// W3C SCXML 5.7.1 + 4.9: a <send> whose <param> cannot be read still sends
// its message without that pair, and the error ends its block; a valid
// location param is sent — Go AOT path.
//
// Measured 2026-09-27, this channel let the rest of the block run and
// silently omitted a `location=""` pair without an error.
//
// Fixture: integration_resources/a_bad_send_param_ends_its_block/a_bad_send_param_ends_its_block.scxml
// (canonical, shared with the C++ / C11 / Rust / Kotlin / Python channels).
//
// Regeneration: scripts/regen_a_bad_send_param_ends_its_block_go.sh

package a_bad_send_param_ends_its_block

import (
	"fmt"
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
)

func record(v int64, ok bool) string {
	if !ok {
		return "<unreadable>"
	}
	return fmt.Sprint(v)
}

func TestTheMessageGoesAndTheBlockStops(t *testing.T) {
	policy := NewABadSendParamEndsItsBlockPolicy()
	policy.SessionID = sce.GenerateSessionID()
	// The handlers record with <assign>, so this is an ECMAScript-datamodel
	// machine.
	policy.ScriptEngine = scegotest.NewLuaEngine()
	engine := sce.NewEngine[ABadSendParamEndsItsBlockState, ABadSendParamEndsItsBlockEvent](&policy)
	engine.Initialize()
	engine.RaiseExternal(ABadSendParamEndsItsBlockEventFinish, "", "")
	engine.Step()

	seen := fmt.Sprintf("errors=%s partials=%s bares=%s after=%s carried=%s (wanted 3 / 2 / 1 / 0 / 1)",
		record(policy.Errors()), record(policy.Partials()), record(policy.Bares()), record(policy.After()),
		record(policy.Carried()))
	if ended, ok := engine.TerminalState(); !ok || ended != ABadSendParamEndsItsBlockStateDone {
		t.Errorf("`finish` must carry the run to `done` (%s)", seen)
	}
	if v, ok := policy.Errors(); !ok || v != 3 {
		t.Errorf("each unreadable <param> raises one error.execution (%s)", seen)
	}
	if v, ok := policy.Partials(); !ok || v != 2 {
		t.Errorf("both internal sends go, carrying the good pair without the bad one (%s)", seen)
	}
	if v, ok := policy.Bares(); !ok || v != 1 {
		t.Errorf("the external send goes with its empty pair left out (%s)", seen)
	}
	if v, ok := policy.After(); !ok || v != 0 {
		t.Errorf("the <param> error ends the block, so nothing after the <send> runs (%s)", seen)
	}
	if v, ok := policy.Carried(); !ok || v != 1 {
		t.Errorf("a valid location param is sent with its value (%s)", seen)
	}
}
