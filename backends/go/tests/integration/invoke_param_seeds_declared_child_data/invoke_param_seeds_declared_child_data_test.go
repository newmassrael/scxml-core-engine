// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// §scxml-6.4.3: an `<invoke>` `<param>` seeds a declared `<data>` of the
// invoked session with the INVOKING session's value — Go AOT channel.
//
// The clause has two halves and the fixture gives each one a `<final>`:
// a matching name takes the param's value (and the child's own `<data>`
// expression is ignored), and a name matching no top-level `<data>` is
// not added to the child's data model at all.
//
// The W3C IRP param surface (226, 240, 241, 243, 244, 245, 276) passes
// literals only, so it cannot separate "the parent evaluated this" from
// "the child evaluated this text" — `1` means `1` in either data model.
// This fixture makes the two answers differ.
//
// Fixture: integration_resources/invoke_param_seeds_declared_child_data/invoke_param_seeds_declared_child_data.scxml
//
// Regeneration (after fixture or template edit):
//   scripts/regen_invoke_param_seeds_declared_child_data_go.sh

package invoke_param_seeds_declared_child_data

import (
	"testing"
	"time"

	sce "github.com/newmassrael/sce-go-runtime"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
)

func TestAnInvokeParamCarriesTheInvokingSessionsValue(t *testing.T) {
	// Engine DI Parity RFC (Path B+): per-test LuaEngine.
	policy := NewInvokeParamSeedsDeclaredChildDataPolicy()
	policy.SessionID = sce.GenerateSessionID()
	policy.ScriptEngine = scegotest.NewLuaEngine()
	engine := sce.NewEngine[InvokeParamSeedsDeclaredChildDataState, InvokeParamSeedsDeclaredChildDataEvent](&policy)
	engine.Initialize()

	// Three sequential invokes, each answering in its own macrostep.
	completed := engine.RunUntilCompletion(3*time.Second, 10*time.Millisecond)
	if !completed {
		t.Fatalf("invoke_param_seeds_declared_child_data timed out before reaching a final state")
	}

	got := engine.GetCurrentState()
	if got != InvokeParamSeedsDeclaredChildDataStatePass {
		t.Fatalf(
			"reached %v, want Pass. "+
				"FailChildEvaluatedTheExpression: the child evaluated the author's "+
				"`<param expr>` text in its own data model and found its own `token` "+
				"— §scxml-6.4.3 says the value of the param element, which only the "+
				"invoking session can produce. "+
				"FailParentOnlyExprLost: the expression named a variable only the "+
				"parent has and nothing arrived, which is the same defect where the "+
				"child has no shadow to find. "+
				"FailUnmatchedParamEnteredTheChild: a `<param>` naming no top-level "+
				"`<data>` of the child became a variable there anyway — the clause "+
				"forbids adding it; on this channel that filter is the "+
				"`HasVariable` guard in the child's `InitializeDataModel`. "+
				"FailShadowSeedLost / FailDeclaredParamLost / FailNamelistValueLost: "+
				"the child saw neither the parent's value nor a shadow, so its own "+
				"`<data>` default stood — nothing was seeded at all.",
			got,
		)
	}
}
