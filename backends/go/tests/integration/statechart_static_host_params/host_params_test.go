// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.15 — a `<param expr>` of a `<send>` or an `<invoke>`
// the HOST serves, in a `datamodel="sce-static"` machine, carries the value of a
// typed expression read from the machine's own fields when the send or the
// invoke happens. Go compile+run gate; the Rust and Kotlin twins are
// `a_static_machines_typed_params_reach_the_host.rs` and `StaticHostParamsTest`.
//
// The committed machine in this directory is generated from
// `sce-build/tests/fixtures/host_processor/statechart_static_host_params.scxml`
// (regen: `scripts/regen_host_processor.sh`) and is built with NO script engine:
// its variables are fields, and a `<param>` that needed an engine to be read
// would not have one to ask.
//
// What this holds is the value on the wire. The run `bump`, `go` changes every
// variable before the send and the invoke read it, so a copy taken at start-up
// (3, false, "idle") is told from what the fields hold now (4, true, "busy").

package statechart_static_host_params

import (
	"encoding/json"
	"reflect"
	"sort"
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"
)

// The type the fixture was compiled for; `scripts/regen_host_processor.sh`
// passes the same string to both declarations.
const declaredType = "x-sce-host"

type Engine = sce.Engine[StatechartStaticHostParamsState, StatechartStaticHostParamsEvent]

type started struct {
	engine *Engine
	policy *StatechartStaticHostParamsPolicy
	sends  *[]sce.HostSendRequest
	starts *[]sce.HostInvokeRequest
}

// A machine with the host's side registered, standing at `idle`.
func newStarted() started {
	policy := NewStatechartStaticHostParamsPolicy()
	policy.SessionID = sce.GenerateSessionID()
	engine := sce.NewEngine[StatechartStaticHostParamsState, StatechartStaticHostParamsEvent](&policy)
	sends := []sce.HostSendRequest{}
	starts := []sce.HostInvokeRequest{}
	engine.RegisterEventProcessor(declaredType, func(req sce.HostSendRequest) []sce.HostSendResponse {
		sends = append(sends, req)
		return nil
	})
	engine.RegisterInvoker(declaredType, func(ev sce.HostInvokeEvent) *sce.HostInvokeResponse {
		if ev.Start != nil {
			starts = append(starts, *ev.Start)
		}
		return nil
	})
	engine.Initialize()
	return started{engine: engine, policy: &policy, sends: &sends, starts: &starts}
}

func (s started) drive(events ...string) {
	for _, event := range events {
		s.engine.RaiseExternalByName(event, "")
		s.engine.Step()
	}
}

// The text each param crosses as, sorted by name.
func textParams(params map[string][]string) [][2]string {
	names := make([]string, 0, len(params))
	for name := range params {
		names = append(names, name)
	}
	sort.Strings(names)
	read := make([][2]string, 0, len(names))
	for _, name := range names {
		for _, value := range params[name] {
			read = append(read, [2]string{name, value})
		}
	}
	return read
}

// The text each param crosses as, given what `count`, `ready`, `label` and
// `twice` hold. `delta` and `ratio` never change; `boom` is left out, because the
// multiplication that makes it overflows a 32-bit field.
func wantedTextParams(count, ready, label, twice string) [][2]string {
	wanted := [][2]string{
		{"count", count}, {"ready", ready}, {"label", label}, {"twice", twice},
		{"delta", "-5"}, {"ratio", "1.5"},
	}
	sort.Slice(wanted, func(i, j int) bool { return wanted[i][0] < wanted[j][0] })
	return wanted
}

// `eventData` is the pairs as JSON, typed as the data model holds them: a number
// stays a number, a bool a bool, a string a string. Compared as values, not text,
// because the order of an object's members is not the document's.
func assertTypedEventData(t *testing.T, eventData, what string) {
	t.Helper()
	var value map[string]any
	if err := json.Unmarshal([]byte(eventData), &value); err != nil {
		t.Fatalf("%s: event_data is not JSON (%v): %s", what, err, eventData)
	}
	wanted := map[string]any{
		"count": float64(4), "ready": true, "label": "busy", "twice": float64(8),
		"delta": float64(-5), "ratio": 1.5,
	}
	for name, expected := range wanted {
		if got, ok := value[name]; !ok || !reflect.DeepEqual(got, expected) {
			t.Errorf("%s: `%s` is %v in %s, want %v", what, name, got, eventData, expected)
		}
	}
	if _, present := value["boom"]; present {
		t.Errorf("%s: a pair whose value failed is left out: %s", what, eventData)
	}
}

func TestASendParamCarriesTheValueTheFieldsHoldWhenItIsSent(t *testing.T) {
	s := newStarted()
	s.drive("bump", "go")
	if len(*s.sends) != 1 {
		t.Fatalf("one <send>, one request: %v", *s.sends)
	}
	if got, want := textParams((*s.sends)[0].Params), wantedTextParams("4", "true", "busy", "8"); !reflect.DeepEqual(got, want) {
		t.Errorf("the text each <param> crosses as: got %v, want %v", got, want)
	}
	assertTypedEventData(t, (*s.sends)[0].EventData, "send")
}

func TestAnInvokeParamCarriesTheValueTheFieldsHoldWhenItStarts(t *testing.T) {
	s := newStarted()
	s.drive("bump", "go")
	if len(*s.starts) != 1 {
		t.Fatalf("one <invoke>, one start: %v", *s.starts)
	}
	// A copy taken at start-up would say count 3, ready false, label idle.
	if got, want := textParams((*s.starts)[0].Params), wantedTextParams("4", "true", "busy", "8"); !reflect.DeepEqual(got, want) {
		t.Errorf("the text each <param> crosses as: got %v, want %v", got, want)
	}
	assertTypedEventData(t, (*s.starts)[0].EventData, "invoke")
}

// The same machine on the shorter run: nothing has written a variable, so the
// fields still hold what `<data expr>` gave them. The control that keeps the two
// cases above from passing on a value that is simply always the new one.
func TestAParamReadBeforeAnyBumpCarriesTheDeclaredValues(t *testing.T) {
	s := newStarted()
	s.drive("go")
	want := wantedTextParams("3", "false", "idle", "6")
	if got := textParams((*s.sends)[0].Params); !reflect.DeepEqual(got, want) {
		t.Errorf("send: got %v, want %v", got, want)
	}
	if got := textParams((*s.starts)[0].Params); !reflect.DeepEqual(got, want) {
		t.Errorf("invoke: got %v, want %v", got, want)
	}
}

// W3C SCXML 5.7.1: a `<param>` whose value cannot be computed — here a
// multiplication a 32-bit field cannot hold — is reported with `error.execution`
// and its pair left out, while the message still goes and the invocation still
// starts. `errors` counts the reports the document took, one for the send and one
// for the invoke, so a pair dropped in silence is told from one reported.
func TestAParamWhoseValueCannotBeComputedIsReportedAndLeftOut(t *testing.T) {
	s := newStarted()
	s.drive("bump", "go")
	if len(*s.sends) != 1 {
		t.Errorf("the send still went: %v", *s.sends)
	}
	if len(*s.starts) != 1 {
		t.Errorf("the invoke still started: %v", *s.starts)
	}
	if got := s.policy.Errors(); got != 2 {
		t.Errorf("errors is %d, not 2: one error.execution for the send's `boom` and one for the invoke's", got)
	}
	for _, params := range []map[string][]string{(*s.sends)[0].Params, (*s.starts)[0].Params} {
		if _, present := params["boom"]; present {
			t.Errorf("the failed pair is left out: %v", params)
		}
	}
}
