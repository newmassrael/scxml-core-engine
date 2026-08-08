// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4.1: an <invoke> naming an unsupported `type` raises
// error.execution — Go AOT path.
//
// The spec defines the case ("the processor MUST place error.execution in the
// internal event queue"), so the document is valid SCXML with one observable:
// that raise. No child session starts and done.invoke.<id> never fires.
//
// Both engines were silent here in different ways before this landed — the
// Interpreter substituted an SCXML handler for the unknown type, and AOT
// dropped the <invoke> from the model entirely. A backend that renders this
// fixture without the raise reproduces the AOT form, and the machine then
// rests in `probe` instead of reaching `pass`.
//
// Fixture: integration_resources/invoke_unsupported_type/invoke_unsupported_type.scxml
//
// Regeneration (after fixture or template edit):
//   scripts/regen_invoke_unsupported_type_go.sh

package invoke_unsupported_type

import (
	"testing"
	"time"

	sce "github.com/newmassrael/sce-go-runtime"
)

func TestAnUnsupportedInvokeTypeRaisesErrorExecution(t *testing.T) {
	policy := NewInvokeUnsupportedTypePolicy()
	policy.SessionID = sce.GenerateSessionID()
	engine := sce.NewEngine[InvokeUnsupportedTypeState, InvokeUnsupportedTypeEvent](&policy)
	engine.Initialize()

	completed := engine.RunUntilCompletion(2*time.Second, 10*time.Millisecond)

	if !completed {
		t.Fatalf("the machine never completed (parked in %v). W3C SCXML 6.4.1 requires "+
			"an <invoke> whose `type` names no supported processor to place "+
			"error.execution on the internal queue; parking in `probe` means the "+
			"<invoke> was dropped rather than lowered", engine.GetCurrentState())
	}
	if got := engine.GetCurrentState(); got != InvokeUnsupportedTypeStatePass {
		t.Fatalf("machine reached %v, want Pass: it completed somewhere other than the "+
			"error.execution target", got)
	}
}
