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
	if len(*s.sends) != 4 {
		t.Fatalf("the pairs' send and the three that carry one value: %v", *s.sends)
	}
	if got := (*s.sends)[0].EventName; got != "notify" {
		t.Fatalf("the first request is the pairs' send: %q", got)
	}
	if got, want := textParams((*s.sends)[0].Params), wantedTextParams("4", "true", "busy", "8"); !reflect.DeepEqual(got, want) {
		t.Errorf("the text each <param> crosses as: got %v, want %v", got, want)
	}
	assertTypedEventData(t, (*s.sends)[0].EventData, "send")
}

// The request the send of `event` made, which a run of the fixture holds once.
func (s started) requestOf(t *testing.T, event string) sce.HostSendRequest {
	t.Helper()
	var found []sce.HostSendRequest
	for _, request := range *s.sends {
		if request.EventName == event {
			found = append(found, request)
		}
	}
	if len(found) != 1 {
		t.Fatalf("one request for `%s`, got %d: %v", event, len(found), *s.sends)
	}
	return found[0]
}

// SCE Accepted Subset §2.15: a `<content expr>` that names one value is the
// event's whole data, as the JSON the value is, and the request's `content`, as
// the text it is — read from the fields when the send runs (`count` 4 and
// `label` "busy" after `bump`, not the 3 and "idle" a copy at start-up holds).
func TestASendContentThatNamesAValueCarriesItWhole(t *testing.T) {
	s := newStarted()
	s.drive("bump", "go")

	value := s.requestOf(t, "value")
	if value.EventData != "8" || value.Content != "8" {
		t.Errorf("a number is its digits, as data and as content: data %q, content %q", value.EventData, value.Content)
	}
	if len(value.Params) != 0 {
		t.Errorf("a value is not a pair: %v", value.Params)
	}
	text := s.requestOf(t, "text")
	if text.EventData != `"busy"` || text.Content != "busy" {
		t.Errorf("a string is quoted as data and is itself as content: data %q, content %q", text.EventData, text.Content)
	}
}

// The control for the case above: nothing has written a variable, so the fields
// still hold what `<data expr>` gave them.
func TestASendContentReadBeforeAnyBumpCarriesTheDeclaredValue(t *testing.T) {
	s := newStarted()
	s.drive("go")

	value := s.requestOf(t, "value")
	if value.EventData != "6" || value.Content != "6" {
		t.Errorf("data %q, content %q", value.EventData, value.Content)
	}
	text := s.requestOf(t, "text")
	if text.EventData != `"idle"` || text.Content != "idle" {
		t.Errorf("data %q, content %q", text.EventData, text.Content)
	}
}

// W3C SCXML 5.6.2: a `<content expr>` that cannot be evaluated — here a
// multiplication a 32-bit field cannot hold — is reported with `error.execution`
// and has the empty string as its value, while the message still goes.
func TestASendContentWhoseValueCannotBeComputedIsTheEmptyString(t *testing.T) {
	s := newStarted()
	s.drive("bump", "go")

	lost := s.requestOf(t, "lost")
	if lost.EventData != `""` || lost.Content != "" {
		t.Errorf("the empty string, as data and as content: data %q, content %q", lost.EventData, lost.Content)
	}
	for _, request := range *s.sends {
		if request.EventName == "after" {
			t.Errorf("the error ends the block, so the send after it never runs: %v", *s.sends)
		}
	}
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
	if got := (*s.starts)[0].Src; got != "job://params" {
		t.Errorf("the src is the string the machine computed when the invocation started: got %q", got)
	}
	if got := (*s.starts)[0].Content; got != "busy" {
		t.Errorf("the content is the body the machine computed when the invocation started, not the \"idle\" a copy at start-up holds: got %q", got)
	}
}

// W3C SCXML 6.4.1: an attribute that cannot be evaluated starts nothing. `big`
// makes `count` too large for the multiplication the `srcexpr` is chosen by, so
// the source cannot be computed and the host is never asked.
func TestAnInvokeWhoseSourceCannotBeComputedStartsNothing(t *testing.T) {
	s := newStarted()
	s.drive("big", "go")
	if len(*s.starts) != 0 {
		t.Errorf("a source nobody could compute starts nothing: %v", *s.starts)
	}
}

// The same for the body: `bloat` makes `huge` too large for the multiplication the
// `<content expr>` is chosen by while `count` is as it was, so the source can be
// computed and the body cannot, and the host is never asked.
func TestAnInvokeWhoseBodyCannotBeComputedStartsNothing(t *testing.T) {
	s := newStarted()
	s.drive("bloat", "go")
	if len(*s.starts) != 0 {
		t.Errorf("a body nobody could compute starts nothing: %v", *s.starts)
	}
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
	if got := (*s.starts)[0].Content; got != "idle" {
		t.Errorf("invoke content: got %q, want \"idle\"", got)
	}
}

// W3C SCXML 5.7.1: a `<param>` whose value cannot be computed — here a
// multiplication a 32-bit field cannot hold — is reported with `error.execution`
// and its pair left out, while the message still goes and the invocation still
// starts. `errors` counts the reports the document took, one for the send's pair,
// one for the content that could not be computed and one for the invoke, so a
// pair dropped in silence is told from one reported.
func TestAParamWhoseValueCannotBeComputedIsReportedAndLeftOut(t *testing.T) {
	s := newStarted()
	s.drive("bump", "go")
	if len(*s.sends) != 4 {
		t.Errorf("every send still went, the one whose value failed among them: %v", *s.sends)
	}
	if len(*s.starts) != 1 {
		t.Errorf("the invoke still started: %v", *s.starts)
	}
	if got := s.policy.Errors(); got != 3 {
		t.Errorf("errors is %d, not 3: one error.execution for the send's `boom`, one for the content and one for the invoke's", got)
	}
	for _, params := range []map[string][]string{(*s.sends)[0].Params, (*s.starts)[0].Params} {
		if _, present := params["boom"]; present {
			t.Errorf("the failed pair is left out: %v", params)
		}
	}
}
