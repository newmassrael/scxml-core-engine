// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// docs/adr/0038, decision 3: a host states in its own type whether it performs
// the duties of a machine, and a machine that states a duty refuses a host that
// only simulates them unless it is built for one on purpose.
//
// `static_host_duty.scxml` states the duty `start_keepalive`, released by
// `stop_keepalive` on the `<onexit>` of `engaged`, and the notice
// `note_engaged`. Its interface carries `Realization()`; `NewStaticHostDutyPolicy`
// panics for a host that does not perform, `NewSimulatedStaticHostDutyPolicy`
// is the caller saying that it means to. `static_host_call.scxml` states no
// duty, so its host declares no realization and is taken as it always was. The
// Kotlin and Python halves are in `StaticDatamodelTest.kt` and
// `test_a_static_host_duty.py`.

package static_datamodel

import (
	"fmt"
	"reflect"
	"strings"
	"testing"

	"github.com/newmassrael/sce-forge-runtime/optional"
	sce "github.com/newmassrael/sce-go-runtime"

	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_host_call"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_host_duty"
)

// refusal is what building a machine panicked with, or nil if it did not panic.
func refusal(build func()) (message any) {
	defer func() { message = recover() }()
	build()
	return nil
}

// runDuty engages and releases the machine once, and returns the names of the
// calls its host heard, oldest first.
func runDuty(policy *static_host_duty.StaticHostDutyPolicy, host *dutyHost) []string {
	policy.SessionID = sce.GenerateSessionID()
	engine := sce.NewEngine[static_host_duty.StaticHostDutyState, static_host_duty.StaticHostDutyEvent](policy)
	engine.Initialize()
	engine.RaiseExternalByName("engage", "")
	engine.Step()
	engine.RaiseExternalByName("release", "")
	engine.Step()
	names := []string{}
	for _, call := range host.take() {
		names = append(names, call.(map[string]any)["action"].(string))
	}
	return names
}

func TestAHostThatPerformsTheDutiesIsAcceptedAndCalled(t *testing.T) {
	host := &dutyHost{realization: sce.ActionPerforms}
	var policy static_host_duty.StaticHostDutyPolicy
	if message := refusal(func() { policy = static_host_duty.NewStaticHostDutyPolicy(host) }); message != nil {
		t.Fatalf("a host that performs was refused: %v", message)
	}
	want := []string{"start_keepalive", "note_engaged", "stop_keepalive"}
	if got := runDuty(&policy, host); !reflect.DeepEqual(got, want) {
		t.Errorf("the host heard %v, want %v", got, want)
	}
}

func TestAHostThatSimulatesTheDutiesIsRefusedAtConstruction(t *testing.T) {
	host := &dutyHost{realization: sce.ActionSimulates}
	message := refusal(func() { static_host_duty.NewStaticHostDutyPolicy(host) })
	if message == nil {
		t.Fatalf("a host that declares Simulates built the machine")
	}
	text := fmt.Sprint(message)
	for _, part := range []string{"StaticHostDuty", "only simulates", "NewSimulatedStaticHostDutyPolicy"} {
		if !strings.Contains(text, part) {
			t.Errorf("the refusal %q does not say %q", text, part)
		}
	}
}

func TestAHostThatDeclaresNoRealizationConstantIsRefused(t *testing.T) {
	// A value no constant names is not Performs, so a host cannot pass the
	// refusal by returning the zero value or a number of its own.
	host := &dutyHost{realization: sce.ActionRealization(99)}
	if message := refusal(func() { static_host_duty.NewStaticHostDutyPolicy(host) }); message == nil {
		t.Errorf("a host that declares a realization no constant names built the machine")
	}
}

func TestTheOptInAcceptsAHostThatSimulatesTheDuties(t *testing.T) {
	host := &dutyHost{realization: sce.ActionSimulates}
	var policy static_host_duty.StaticHostDutyPolicy
	if message := refusal(func() { policy = static_host_duty.NewSimulatedStaticHostDutyPolicy(host) }); message != nil {
		t.Fatalf("the opt-in refused a simulating host: %v", message)
	}
	want := []string{"start_keepalive", "note_engaged", "stop_keepalive"}
	if got := runDuty(&policy, host); !reflect.DeepEqual(got, want) {
		t.Errorf("a host accepted on purpose heard %v, want %v", got, want)
	}
}

// noRealizationHost implements the interface of a machine with no duty, which
// has no `Realization` method, so it compiles only if the interface has none.
type noRealizationHost struct{ heard []string }

func (h *noRealizationHost) ShowAttempts(count uint32, exhausted bool) {
	h.heard = append(h.heard, fmt.Sprintf("%d,%t", count, exhausted))
}

// ShowDue takes arguments that may be absent (docs/adr/0055, step 10); this host
// hears none of them, since `static_host_call` calls it only from events the test
// does not send.
func (h *noRealizationHost) ShowDue(day optional.Of[int32], urgent optional.Of[bool]) {}

func TestAMachineWithNoDutyTakesAHostThatDeclaresNoRealization(t *testing.T) {
	host := &noRealizationHost{}
	policy := static_host_call.NewStaticHostCallPolicy(host)
	policy.SessionID = sce.GenerateSessionID()
	engine := sce.NewEngine[static_host_call.StaticHostCallState, static_host_call.StaticHostCallEvent](&policy)
	engine.Initialize()
	if want := []string{"0,false"}; !reflect.DeepEqual(host.heard, want) {
		t.Errorf("entering `idle` made the host hear %v, want %v", host.heard, want)
	}
}
