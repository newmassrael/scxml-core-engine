// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.10.1 (docs/adr/0090, test 3): a host names the origin of an event
// it delivers in its own terms — Go AOT path. The host runs `job` and completes
// it naming {`x-sce-host`, `motor`}; the document reads `motor` and
// `x-sce-host`, not an SCXML location and not the SCXML processor's type.
//
// Fixture: integration_resources/a_host_names_its_origin_in_its_own_terms/a_host_names_its_origin_in_its_own_terms.scxml
// (canonical, shared with every channel).
//
// Regeneration (after fixture or template edit):
//
//	scripts/regen_a_host_names_its_origin_in_its_own_terms_go.sh
package a_host_names_its_origin_in_its_own_terms

import (
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
)

// hostType is the `<invoke type>` the build is told the host runs; the
// regeneration script passes the same string as `--host-invoker`.
const hostType = "x-sce-host"

func TestAHostNamesItsOriginInItsOwnTerms(t *testing.T) {
	policy := NewAHostNamesItsOriginInItsOwnTermsPolicy()
	policy.SessionID = sce.GenerateSessionID()
	policy.ScriptEngine = scegotest.NewLuaEngine()
	engine := sce.NewEngine[AHostNamesItsOriginInItsOwnTermsState, AHostNamesItsOriginInItsOwnTermsEvent](&policy)
	// The invocation outlives the call that starts it: the host completes it
	// afterwards, under the token the start carried.
	var started *sce.HostInvokeRequest
	engine.RegisterInvoker(hostType, func(ev sce.HostInvokeEvent) *sce.HostInvokeResponse {
		if ev.Start != nil {
			started = ev.Start
		}
		return nil
	})
	if err := engine.Initialize(); err != nil {
		t.Fatal(err)
	}
	engine.Step()

	if started == nil {
		t.Fatalf("`job` was not started when `running` was entered")
	}
	if started.InvokeID != "job" {
		t.Fatalf("the host was asked to run %q, want job", started.InvokeID)
	}
	if !engine.CompleteHostInvokeFrom(hostType, started.InvokeID, started.Token, "",
		sce.NewEventOrigin(hostType, "motor")) {
		t.Fatalf("the running invocation's completion was refused")
	}
	engine.Step()

	if ended, ok := engine.TerminalState(); !ok || ended != AHostNamesItsOriginInItsOwnTermsStateDone {
		t.Errorf("the run must end in `done`")
	}
	if got, ok := policy.HostNamed(); !ok || got != 1 {
		t.Errorf("hostNamed = %d (readable %v), want 1: `_event.origin` and `_event.origintype` "+
			"were not the pair the host named", got, ok)
	}
}
