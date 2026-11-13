// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// SCE Accepted Subset §2.15, "Child sessions", and §scxml-6.4 (docs/adr/0098)
// on the Go channel: a `datamodel="sce-static"` child declares its variables
// with the values its invoke handed, each in place of its own initial value, so
// a variable declared after a handed one reads the handed value.
//
// `worker` is handed `start = 7` and declares `copied = start` after it; it
// ends the moment `copied` is 7. A child that wrote the handed value over
// `start` after declaring both holds `copied = 0`, never ends, and leaves the
// parent in `working`.
//
// Fixture: tests/integration/a_static_child_declares_its_data_with_a_handed_value.scxml.
// The Rust half is `a_static_child_declares_its_data_with_a_handed_value.rs`.

package static_datamodel

import (
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"

	handed "github.com/newmassrael/sce-go-tests/integration/static_datamodel/a_static_child_declares_its_data_with_a_handed_value"
)

func TestAVariableDeclaredAfterAHandedOneReadsTheHandedValue(t *testing.T) {
	policy := handed.NewAStaticChildDeclaresItsDataWithAHandedValuePolicy()
	policy.SessionID = sce.GenerateSessionID()
	engine := sce.NewEngine[handed.AStaticChildDeclaresItsDataWithAHandedValueState, handed.AStaticChildDeclaresItsDataWithAHandedValueEvent](&policy)
	if err := engine.Initialize(); err != nil {
		t.Fatal(err)
	}
	// Let the child start, run and end, and its parent take `done.invoke`.
	for i := 0; i < 5; i++ {
		engine.Tick()
	}
	if terminal, ok := engine.TerminalState(); !ok || terminal != handed.AStaticChildDeclaresItsDataWithAHandedValueStatePassed {
		t.Fatalf("`worker` declares `copied = start` after `start`, which its invoke handed 7, so it ends: the parent stands in %v", engine.GetActiveStates())
	}
}
