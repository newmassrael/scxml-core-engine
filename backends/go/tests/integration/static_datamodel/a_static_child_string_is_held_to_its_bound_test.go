// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// SCE Accepted Subset §2.15, "Child sessions": a string an `<invoke
// type="scxml">` hands its child is held to the bound the child declared for
// that variable, in UTF-8 bytes, as an `<assign>` to it would be. A value past
// it is the evaluation that failed (W3C SCXML 5.7.1): `error.execution` is
// raised, that one value is left out, and the child still starts, holding the
// one its `<data>` gave it.
//
// `static_invoke_string.scxml` invokes three children whose `title` holds four
// bytes: `fits` is handed 'wxyz' and ends on it; `over` is handed eight bytes and
// `wide` two characters of five bytes, both past the bound, so each starts with
// the 'ab' its `<data>` gave it, which it ends on. The Rust and Kotlin halves are
// `a_static_child_string_is_held_to_its_bound.rs` and
// `AStaticChildStringIsHeldToItsBoundTest.kt`.

package static_datamodel

import (
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"

	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_invoke_string"
)

func TestAStringPastTheChildsBoundIsLeftOutAndReported(t *testing.T) {
	policy := static_invoke_string.NewStaticInvokeStringPolicy()
	policy.SessionID = sce.GenerateSessionID()
	engine := sce.NewEngine[static_invoke_string.StaticInvokeStringState, static_invoke_string.StaticInvokeStringEvent](&policy)
	engine.Initialize()
	// Let the children run and report to their parent.
	for i := 0; i < 5; i++ {
		engine.Tick()
	}
	// 1 (`fits`) + 10 (`over`) + 100 (`wide`): all three ended, so none held a
	// value past its bound but the one that fits.
	if got := policy.Completed(); got != 111 {
		t.Errorf("completed is %d, not 111", got)
	}
	// The two values past the bound were reported, once each.
	if got := policy.Errors(); got != 2 {
		t.Errorf("errors is %d, not 2", got)
	}
}
