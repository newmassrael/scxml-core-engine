// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.5 + 6.3.1 donedata surfacing — Go AOT local-invoke path.
//
// Closes the W3C IRP coverage gap: no public IRP test exercises
// `<donedata>` on the invoked child's top-level `<final>` combined
// with `done.invoke.<id>._event.data` readback on the parent. Mirrors
// `tests/integration/DonedataLocalInvokeTest.cpp` (C++ Interpreter,
// commits fb8e3c79 + 00f347cb),
// `sce-kotlin-tests/src/test/kotlin/com/sce/integration/DonedataLocalInvokeTest.kt`
// (Kotlin AOT, 4d284cb4..b070a7ad), and
// `sce-rust-tests/tests/donedata_local_invoke.rs` (Rust AOT,
// b44f02fe..f9e236f6) for the Go AOT code path through
// `sce.RaiseDoneInvoke`.
//
// Fixture: integration_resources/donedata_local_invoke/donedata_local_invoke.scxml
//
// Regeneration (after fixture or template edit):
//   scripts/regen_donedata_local_invoke_go.sh

package donedata_local_invoke

import (
	"testing"
	"time"

	sce "github.com/newmassrael/sce-go-runtime"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
)

func TestParentObservesDonedataOnDoneInvoke(t *testing.T) {
	// Engine DI Parity RFC (Path B+): per-test LuaEngine, replacing the
	// pre-cleanup `RegisterLuaEngine` + `sce.GetScriptEngine()` singleton pair.
	policy := NewDonedataLocalInvokePolicy()
	policy.SessionID = sce.GenerateSessionID()
	policy.ScriptEngine = scegotest.NewLuaEngine()
	engine := sce.NewEngine[DonedataLocalInvokeState, DonedataLocalInvokeEvent](&policy)
	engine.Initialize()

	// Children are synchronous (single top-level `<final>`), so the parent
	// reaches `pass`/`fail` within a few microsteps. A brief poll loop
	// mirrors the Rust/Kotlin harnesses and guards against any future
	// async microstep scheduling.
	completed := engine.RunUntilCompletion(2*time.Second, 10*time.Millisecond)
	if !completed {
		t.Fatalf("donedata_local_invoke timed out before reaching a final state")
	}

	got := engine.GetCurrentState()
	if got != DonedataLocalInvokeStatePass {
		t.Fatalf(
			"parent reached %v, want Pass: `_event.data.result == 42` (param branch) "+
				"or `_event.data == 'hello_content'` (content branch) failed. An empty "+
				"`_event.data` means `sce.RaiseDoneInvoke` dropped the child's donedata "+
				"— mirror the C++ / Kotlin / Rust AOT `stashDonedataAtFinal` contract: "+
				"add `Engine.donedataAtFinal` + a top-level `<final>` stash branch in "+
				"`tools/codegen/templates/go/entry_exit_actions.go.jinja2` and thread "+
				"the stashed payload into `EventMetadata.Data` on the emitted "+
				"`done.invoke.<id>`.",
			got,
		)
	}
}
