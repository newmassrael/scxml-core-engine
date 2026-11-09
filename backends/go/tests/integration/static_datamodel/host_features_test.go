// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The standard host features (docs/SCE_HOST_FEATURES.md) as a generated Go
// machine sees them through the reference host: every scenario of
// `tests/host_features/scenarios` replayed against the machine it names, with
// `hostfeatures.ReferenceHost` registered as the invoker of every standard type.
//
// A scenario's `host` directives set what the host is like before a step; its
// `expect.host_invokes` and `expect.host_state` are what the host was asked and
// what it holds afterwards. The expected values are derived from the
// specification, not observed from the host.

package static_datamodel

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"reflect"
	"sort"
	"strconv"
	"strings"
	"testing"

	hf "github.com/newmassrael/sce-go-host-features"
	sce "github.com/newmassrael/sce-go-runtime"

	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_host_clock"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_host_http"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_host_kv"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_host_model"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_host_notify"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_model_harness"
)

// scriptTransport answers what a scenario says and keeps what it was given.
type scriptTransport struct {
	set      bool
	hold     bool
	fail     bool
	response *hf.HTTPResponse
	log      []hf.HTTPRequest
}

func (s *scriptTransport) Exchange(request hf.HTTPRequest) (*hf.HTTPResponse, error) {
	s.log = append(s.log, request)
	if s.hold {
		return nil, nil
	}
	if s.fail || !s.set {
		return nil, fmt.Errorf("the transport has no response")
	}
	return s.response, nil
}

func headerPairs(value any) []hf.HTTPHeader {
	var out []hf.HTTPHeader
	for _, pair := range value.([]any) {
		p := pair.([]any)
		out = append(out, hf.HTTPHeader{Name: p[0].(string), Value: p[1].(string)})
	}
	return out
}

// directive sets what a step's `http` says the transport answers.
func (s *scriptTransport) directive(d map[string]any) {
	s.set, s.hold, s.fail, s.response = true, false, false, nil
	if hold, _ := d["hold"].(bool); hold {
		s.hold = true
		return
	}
	if _, failed := d["fail"]; failed {
		s.fail = true
		return
	}
	var body []byte
	switch {
	case d["body"] != nil:
		body = []byte(d["body"].(string))
	case d["body_hex"] != nil:
		text := d["body_hex"].(string)
		for i := 0; i+1 < len(text); i += 2 {
			n, err := strconv.ParseUint(text[i:i+2], 16, 8)
			if err != nil {
				panic(err)
			}
			body = append(body, byte(n))
		}
	case d["body_repeat"] != nil:
		repeat := d["body_repeat"].([]any)
		for i := 0; i < int(repeat[1].(float64)); i++ {
			body = append(body, repeat[0].(string)...)
		}
	}
	s.response = &hf.HTTPResponse{Status: int(d["status"].(float64)), Headers: headerPairs(d["headers"]), Body: body}
}

type hostRig struct {
	host      *hf.ReferenceHost
	clock     *hf.ScriptedClock
	transport *scriptTransport
	model     *hf.ScriptedProvider
}

// answerText is the text a `model` answer gives: `text`, or `text_repeat` as a
// text and a count.
func answerText(answer map[string]any) string {
	if text, ok := answer["text"].(string); ok {
		return text
	}
	repeat := answer["text_repeat"].([]any)
	return strings.Repeat(repeat[0].(string), int(repeat[1].(float64)))
}

// scriptModel applies a `model` directive to the scripted provider: each member
// it names.
func scriptModel(provider *hf.ScriptedProvider, d map[string]any) {
	if answers, ok := d["answers"].([]any); ok {
		provider.Answers = nil
		for _, raw := range answers {
			answer := raw.(map[string]any)
			finish, ok := answer["finish"].(string)
			if !ok {
				finish = "stop"
			}
			provider.Answers = append(provider.Answers, hf.ScriptedAnswer{Text: answerText(answer), Finish: finish})
		}
	}
	if constrained, ok := d["constrained"].(bool); ok {
		provider.Constrained = constrained
	}
	if name, ok := d["name"].(string); ok {
		provider.Name = name
	}
	if context, ok := d["context"].(float64); ok {
		provider.Context = uint64(context)
	}
	if hold, ok := d["hold"].(bool); ok {
		provider.Hold = hold
	}
}

// integerFields are the request fields a scenario writes as numbers; every
// other is text.
var integerFields = map[string]bool{"at_ms": true, "max_tokens": true, "temperature_milli": true}

// directives applies the `host` object of a step.
func (r *hostRig) directives(t *testing.T, directives map[string]any) {
	for name, value := range directives {
		switch name {
		case "clock":
			c := value.(map[string]any)
			reading, zone := *r.clock.Reading, *r.clock.ZoneNow
			if n, ok := c["utc_ms"].(float64); ok {
				reading.UTCMs = int64(n)
			}
			if n, ok := c["monotonic_ms"].(float64); ok {
				reading.MonotonicMs = int64(n)
			}
			if s, ok := c["zone_id"].(string); ok {
				zone.ID = s
			}
			if n, ok := c["utc_offset_s"].(float64); ok {
				zone.UTCOffsetS = int32(n)
			}
			r.clock.Reading, r.clock.ZoneNow = &reading, &zone
		case "permission":
			r.host.SetPermission(value.(string))
		case "user_decision":
			r.host.SetUserDecision(value.(string))
		case "quota":
			r.host.SetQuota(int(value.(float64)))
		case "fail":
			faults := map[string]string{}
			for operation, code := range value.(map[string]any) {
				faults[operation] = code.(string)
			}
			r.host.SetFaults(faults)
		case "http":
			r.transport.directive(value.(map[string]any))
		case "model":
			scriptModel(r.model, value.(map[string]any))
		default:
			t.Fatalf("`%s` is not a host directive", name)
		}
	}
}

// starts are what the host was started with since the last step, as a scenario
// writes it: the request as its record, the integer fields as numbers.
func (r *hostRig) starts() []any {
	out := []any{}
	for _, start := range r.host.TakeStarts() {
		request := map[string]any{}
		for name, text := range start.Params {
			if integerFields[name] {
				n, err := strconv.ParseInt(text, 10, 64)
				if err != nil {
					panic(err)
				}
				request[name] = float64(n)
			} else {
				request[name] = text
			}
		}
		out = append(out, map[string]any{"type": start.Operation, "id": start.ID, "request": request})
	}
	return out
}

func stringList(items []string) []any {
	out := []any{}
	for _, item := range items {
		out = append(out, item)
	}
	return out
}

// hostState is the part of the host's state a scenario names.
func (r *hostRig) hostState(t *testing.T, wanted map[string]any) map[string]any {
	state := map[string]any{}
	for name := range wanted {
		switch name {
		case "store":
			store := map[string]any{}
			for _, entry := range r.host.StoreEntries() {
				store[entry[0]] = entry[1]
			}
			state[name] = store
		case "waiting":
			state[name] = stringList(r.host.Waiting())
		case "shown":
			state[name] = stringList(r.host.Shown())
		case "cancelled":
			state[name] = stringList(r.host.Cancelled())
		case "http":
			log := []any{}
			for _, request := range r.transport.log {
				headers := []any{}
				for _, h := range request.Headers {
					headers = append(headers, map[string]any{"name": h.Name, "value": h.Value})
				}
				log = append(log, map[string]any{"method": request.Method, "url": request.URL, "headers": headers, "body": request.Body})
			}
			state[name] = log
		default:
			t.Fatalf("`%s` is not host state", name)
		}
	}
	return state
}

// same is whether a variable's value is the scenario's.
func same(got any, want any) bool {
	switch w := want.(type) {
	case float64:
		switch g := got.(type) {
		case int64:
			return float64(g) == w
		case int32:
			return float64(g) == w
		case uint16:
			return float64(g) == w
		case uint32:
			return float64(g) == w
		case int:
			return float64(g) == w
		case float64:
			return g == w
		}
		return false
	default:
		return reflect.DeepEqual(got, want)
	}
}

const hostSettleTicks = 5

// replayHost replays the scenario of `name` against a machine the host serves.
func replayHost[S interface {
	comparable
	String() string
}, E comparable](t *testing.T, name string, policy sce.StatePolicy[S, E], schemas []sce.AnswerSchema, variables map[string]func() any) {
	replayHostScenario[S, E](t, name, name, policy, schemas, variables)
}

// replayHostScenario replays the scenario `stem` against `machine`, the machine the
// scenario names, which the host serves: a machine whose run ends has one scenario
// a run (the reference harness's cases).
func replayHostScenario[S interface {
	comparable
	String() string
}, E comparable](t *testing.T, stem string, machine string, policy sce.StatePolicy[S, E], schemas []sce.AnswerSchema, variables map[string]func() any) {
	path := filepath.Join("..", "..", "..", "..", "..", "tests", "host_features", "scenarios", stem+".json")
	text, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("the scenario is not readable: %v", err)
	}
	var scenario map[string]any
	if err := json.Unmarshal(text, &scenario); err != nil {
		t.Fatalf("a scenario is JSON: %v", err)
	}
	if scenario["machine"] != machine {
		t.Fatalf("the scenario %s names %v, not %s", stem, scenario["machine"], machine)
	}
	clock := hf.NewScriptedClock()
	transport := &scriptTransport{}
	rig := &hostRig{host: hf.NewReferenceHost(clock, transport), clock: clock, transport: transport, model: hf.NewScriptedProvider()}
	rig.host.SetModelProvider(rig.model)
	rig.host.SetAnswerSchemas(schemas)

	engine := sce.NewEngine[S, E](policy)
	engine.SetClock(sce.NewManualClock(0))
	hf.Serve(engine, rig.host)
	if err := engine.Initialize(); err != nil {
		t.Fatal(err)
	}
	scheduled, needsScheduler := any(policy).(interface{ NeedsEventScheduler() bool })
	needsScheduler = needsScheduler && scheduled.NeedsEventScheduler()
	settle := func() {
		if !needsScheduler {
			engine.Step()
			return
		}
		for i := 0; i < hostSettleTicks; i++ {
			engine.Tick()
		}
	}
	settle()

	steps := scenario["steps"].([]any)
	if len(steps) == 0 {
		t.Fatal("a scenario with no steps judges nothing")
	}
	for n, raw := range steps {
		step := raw.(map[string]any)
		note, _ := step["note"].(string)
		note = stem + ": " + note
		if directives, ok := step["host"].(map[string]any); ok {
			rig.directives(t, directives)
		}
		if ms, ok := step["advance_ms"].(float64); ok {
			engine.AdvanceTimeMs(int64(ms))
			settle()
		}
		if event, ok := step["event"].(string); ok {
			if _, matches := engine.ResolveEventByName(event); !matches {
				t.Fatalf("step %d: the machine's events do not match `%s`", n, event)
			}
			data := ""
			if payload, ok := step["data"]; ok {
				encoded, err := json.Marshal(payload)
				if err != nil {
					t.Fatal(err)
				}
				data = string(encoded)
			}
			engine.RaiseExternalByName(event, data)
			settle()
		}
		expect := step["expect"].(map[string]any)
		started := rig.starts()
		if want, ok := expect["host_invokes"]; ok && !reflect.DeepEqual(started, want) {
			t.Errorf("step %d (%s): what the host was asked: %v, want %v", n, note, started, want)
		}
		if want, ok := expect["host_state"].(map[string]any); ok {
			if got := rig.hostState(t, want); !reflect.DeepEqual(got, want) {
				t.Errorf("step %d (%s): what the host holds: %v, want %v", n, note, got, want)
			}
		}
		if want, ok := expect["state"].(string); ok {
			var atomic []S
			for _, state := range engine.GetActiveStates() {
				if !policy.IsCompoundState(state) {
					atomic = append(atomic, state)
				}
			}
			if len(atomic) != 1 || atomic[0].String() != want {
				t.Errorf("step %d (%s): the current state is %v, want %s", n, note, atomic, want)
			}
		}
		if vars, ok := expect["variables"].(map[string]any); ok {
			for variable, want := range vars {
				read, known := variables[variable]
				if !known {
					t.Fatalf("step %d: the driver reads no variable `%s`", n, variable)
				}
				if got := read(); !same(got, want) {
					t.Errorf("step %d (%s): variable `%s` is %v, not %v", n, note, variable, got, want)
				}
			}
		}
	}
}

func TestAMachineAsksTheKeyValueStoreOfItsHost(t *testing.T) {
	policy := static_host_kv.NewStaticHostKvPolicy()
	policy.SessionID = sce.GenerateSessionID()
	replayHost[static_host_kv.StaticHostKvState, static_host_kv.StaticHostKvEvent](t, "static_host_kv", &policy, nil, map[string]func() any{
		"outcome":    func() any { return policy.Outcome() },
		"found":      func() any { return policy.Found() },
		"got":        func() any { return policy.Got() },
		"replaced":   func() any { return policy.Replaced() },
		"existed":    func() any { return policy.Existed() },
		"next_found": func() any { return policy.NextFound() },
		"next":       func() any { return policy.Next() },
	})
}

func TestAMachineReadsTheClockOfItsHost(t *testing.T) {
	policy := static_host_clock.NewStaticHostClockPolicy()
	policy.SessionID = sce.GenerateSessionID()
	replayHost[static_host_clock.StaticHostClockState, static_host_clock.StaticHostClockEvent](t, "static_host_clock", &policy, nil, map[string]func() any{
		"outcome":      func() any { return policy.Outcome() },
		"utc_ms":       func() any { return policy.UtcMs() },
		"monotonic_ms": func() any { return policy.MonotonicMs() },
		"zone_id":      func() any { return policy.ZoneId() },
		"utc_offset_s": func() any { return policy.UtcOffsetS() },
	})
}

func TestAMachinePostsNotificationsThroughItsHost(t *testing.T) {
	policy := static_host_notify.NewStaticHostNotifyPolicy()
	policy.SessionID = sce.GenerateSessionID()
	replayHost[static_host_notify.StaticHostNotifyState, static_host_notify.StaticHostNotifyEvent](t, "static_host_notify", &policy, nil, map[string]func() any{
		"outcome":    func() any { return policy.Outcome() },
		"permission": func() any { return policy.Permission() },
		"status":     func() any { return policy.Status() },
		"existed":    func() any { return policy.Existed() },
	})
}

func TestAMachineMakesAnHTTPExchangeThroughItsHost(t *testing.T) {
	policy := static_host_http.NewStaticHostHttpPolicy()
	policy.SessionID = sce.GenerateSessionID()
	replayHost[static_host_http.StaticHostHttpState, static_host_http.StaticHostHttpEvent](t, "static_host_http", &policy, nil, map[string]func() any{
		"outcome":          func() any { return policy.Outcome() },
		"status":           func() any { return policy.Status() },
		"response_headers": func() any { return policy.ResponseHeaders() },
		"response_body":    func() any { return policy.ResponseBody() },
	})
}

// modelVariables are the readers of what static_host_model publishes, each as a
// scenario states it: a record by its fields, a 64-bit integer as its digits.
func modelVariables(t *testing.T, policy *static_host_model.StaticHostModelPolicy) map[string]func() any {
	return map[string]func() any{
		"calls":         func() any { return policy.Calls() },
		"outcome":       func() any { return policy.Outcome() },
		"finish":        func() any { return policy.Finish() },
		"constrained":   func() any { return policy.Constrained() },
		"answered_by":   func() any { return policy.AnsweredBy() },
		"prompt_tokens": func() any { return policy.PromptTokens() },
		"answer_tokens": func() any { return policy.AnswerTokens() },
		"answer": func() any {
			if a, ok := policy.Answer().Get(); ok {
				return asJSON(t, map[string]any{
					"id": a.Id(), "amount": signedDigits(a.Amount()), "title": a.Title(), "kind": a.Kind().String(),
					"note": a.Note(), "tag": sce.BytesAsPayloadText(a.Tag()), "priority": a.Priority(),
				})
			}
			return nil
		},
		"why": func() any {
			if why, ok := policy.Why().Get(); ok {
				return asJSON(t, map[string]any{"code": why.Code().String(), "path": why.Path(), "bound": why.Bound()})
			}
			return nil
		},
	}
}

func TestAMachineAsksItsHostForATextUnderAnAnswerSchema(t *testing.T) {
	policy := static_host_model.NewStaticHostModelPolicy()
	policy.SessionID = sce.GenerateSessionID()
	replayHost[static_host_model.StaticHostModelState, static_host_model.StaticHostModelEvent](t, "static_host_model", &policy, static_host_model.AnswerSchemas, modelVariables(t, &policy))
}

// outcomeClass is what the exhaustive provider answers for a class of the table.
func outcomeClass(class map[string]any) hf.OutcomeClass {
	switch class["end"] {
	case "answer":
		return hf.OutcomeClass{Kind: hf.Answered, Text: class["text"].(string), Finish: class["finish"].(string)}
	case "failure":
		return hf.OutcomeClass{Kind: hf.Failed, Code: class["error"].(string)}
	case "refusal":
		return hf.OutcomeClass{Kind: hf.Refused}
	default:
		return hf.OutcomeClass{Kind: hf.Held}
	}
}

// runSequence runs static_host_model against the exhaustive provider answering
// the classes sequence names, one call a class, and holds what the machine
// publishes after each call to what its class says.
func runSequence(t *testing.T, table map[string]any, sequence []int) {
	classes := table["classes"].([]any)
	var provided []hf.OutcomeClass
	var named []string
	for _, i := range sequence {
		class := classes[i].(map[string]any)
		provided = append(provided, outcomeClass(class))
		named = append(named, class["class"].(string))
	}
	policy := static_host_model.NewStaticHostModelPolicy()
	policy.SessionID = sce.GenerateSessionID()
	host := hf.NewReferenceHost(hf.NewScriptedClock(), &scriptTransport{})
	host.SetAnswerSchemas(static_host_model.AnswerSchemas)
	host.SetModelProvider(hf.NewExhaustiveProvider(provided))
	engine := sce.NewEngine[static_host_model.StaticHostModelState, static_host_model.StaticHostModelEvent](&policy)
	engine.SetClock(sce.NewManualClock(0))
	hf.Serve(engine, host)
	if err := engine.Initialize(); err != nil {
		t.Fatal(err)
	}
	settle := func() {
		for i := 0; i < hostSettleTicks; i++ {
			engine.Tick()
		}
	}
	settle()
	ask := table["ask"].(map[string]any)
	data, err := json.Marshal(ask["data"])
	if err != nil {
		t.Fatal(err)
	}
	variables := modelVariables(t, &policy)
	var answer any
	for n, i := range sequence {
		class := classes[i].(map[string]any)
		engine.RaiseExternalByName(ask["event"].(string), string(data))
		settle()
		switch class["then"] {
		case "advance":
			engine.AdvanceTimeMs(int64(table["deadline_ms"].(float64)))
			settle()
		case "leave":
			engine.RaiseExternalByName(table["leave"].(string), "")
			settle()
		}
		expect := class["expect"].(map[string]any)
		if read, ok := expect["answer"]; ok {
			answer = read
		}
		at := fmt.Sprintf("%v, call %d", named, n+1)
		var atomic []static_host_model.StaticHostModelState
		for _, state := range engine.GetActiveStates() {
			if !policy.IsCompoundState(state) {
				atomic = append(atomic, state)
			}
		}
		if len(atomic) != 1 || atomic[0].String() != "idle" {
			t.Fatalf("%s: the machine is in %v, not idle", at, atomic)
		}
		for _, name := range []string{"outcome", "why", "finish"} {
			if got := variables[name](); !same(got, expect[name]) {
				t.Fatalf("%s: `%s` is %v, not %v", at, name, got, expect[name])
			}
		}
		if got := variables["answer"](); !same(got, answer) {
			t.Fatalf("%s: `answer` is %v, not %v", at, got, answer)
		}
		if got := variables["calls"](); !same(got, float64(n+1)) {
			t.Fatalf("%s: `calls` is %v", at, got)
		}
	}
}

// The machine around a model is tested exhaustively (docs/adr/0087, section 9.4):
// every sequence of outcome classes up to the table's bound, each call read as its
// class says, and every refusal code of the library either reached by a class or
// named as deferred with its reason.
func TestEverySequenceOfOutcomeClassesIsReadAsItsClassesSay(t *testing.T) {
	text, err := os.ReadFile(filepath.Join("..", "..", "..", "..", "..", "tests", "host_features", "model_outcome_classes.json"))
	if err != nil {
		t.Fatalf("the class table is not readable: %v", err)
	}
	var table map[string]any
	if err := json.Unmarshal(text, &table); err != nil {
		t.Fatalf("the class table is JSON: %v", err)
	}
	if table["machine"] != "static_host_model" || len(static_host_model.AnswerSchemas) == 0 ||
		table["answer_schema"] != static_host_model.AnswerSchemas[0].ID {
		t.Fatalf("the table names another machine or answer schema: %v", table["answer_schema"])
	}
	classes := len(table["classes"].([]any))
	bound := int(table["bound"].(float64))
	if classes == 0 || bound == 0 {
		t.Fatal("a table that names nothing judges nothing")
	}
	runs, expected := 0, 0
	for count, power := 1, classes; count <= bound; count, power = count+1, power*classes {
		expected += power
		sequence := make([]int, count)
		for {
			runSequence(t, table, sequence)
			runs++
			position := count - 1
			for position >= 0 && sequence[position] == classes-1 {
				sequence[position] = 0
				position--
			}
			if position < 0 {
				break
			}
			sequence[position]++
		}
	}
	if runs != expected {
		t.Fatalf("%d sequences were run, not the %d up to the bound", runs, expected)
	}
	codes := map[string]bool{}
	for _, raw := range table["classes"].([]any) {
		if why, ok := raw.(map[string]any)["expect"].(map[string]any)["why"].(map[string]any); ok {
			codes[why["code"].(string)] = true
		}
	}
	for _, raw := range table["deferred"].([]any) {
		codes[raw.(map[string]any)["code"].(string)] = true
	}
	// The fourteen codes, tooFew the last (docs/adr/0086, section 15.11).
	if len(codes) != 14 {
		t.Fatalf("every code of sce:std/lift/refusal_code.scxml is reached or deferred: %v", codes)
	}
}

// harnessScenarios are the cases of the reference harness's scenario (docs/adr/0087,
// section 9.1 item 8), one scenario each: a run of the harness ends at one of its
// finals, so each case is a run of its own.
var harnessScenarios = []string{
	"static_model_harness_accepted",
	"static_model_harness_exhausted",
	"static_model_harness_not_offered",
	"static_model_harness_repaired",
	"static_model_harness_timeout",
}

// harnessVariables are the readers of what static_model_harness publishes, each as a
// scenario states it: a record by its fields, a 64-bit integer as its digits, an enum
// by its name.
func harnessVariables(t *testing.T, policy *static_model_harness.StaticModelHarnessPolicy) map[string]func() any {
	return map[string]func() any{
		"outcome":  func() any { return policy.Outcome() },
		"attempts": func() any { return policy.Attempts() },
		"offered":  func() any { return asJSON(t, policy.Offered()) },
		"refusals": func() any {
			refusals := []any{}
			for _, refusal := range policy.Refusals() {
				refusals = append(refusals, map[string]any{"code": refusal.Code().String(), "path": refusal.Path()})
			}
			return asJSON(t, refusals)
		},
		"answer": func() any {
			if a, ok := policy.Answer().Get(); ok {
				return asJSON(t, map[string]any{
					"id": a.Id(), "amount": signedDigits(a.Amount()), "title": a.Title(), "kind": a.Kind().String(),
					"note": a.Note(), "tag": sce.BytesAsPayloadText(a.Tag()), "priority": a.Priority(),
				})
			}
			return nil
		},
		"why": func() any {
			if why, ok := policy.Why().Get(); ok {
				return asJSON(t, map[string]any{"code": why.Code().String(), "path": why.Path(), "bound": why.Bound()})
			}
			return nil
		},
	}
}

func TestTheReferenceHarnessRunsEachCaseOfItsScenario(t *testing.T) {
	for _, stem := range harnessScenarios {
		policy := static_model_harness.NewStaticModelHarnessPolicy()
		policy.SessionID = sce.GenerateSessionID()
		replayHostScenario[static_model_harness.StaticModelHarnessState, static_model_harness.StaticModelHarnessEvent](
			t, stem, "static_model_harness", &policy, static_model_harness.AnswerSchemas, harnessVariables(t, &policy))
	}
}

// harnessRow is a row of the reference harness's transition table, as the table and
// the transition observer (docs/adr/0089) name it.
type harnessRow = sce.TransitionTaken[static_model_harness.StaticModelHarnessState]

// harnessApplicationCodes are the codes of the application's own checks, after the
// fourteen of the lift its enum repeats (enum_harness_code.scxml), each named for
// the class that reaches it.
var harnessApplicationCodes = []string{"notOffered"}

// harnessSettleTicks are the ticks a harness run is given after each event: a run may
// make its calls and take each completion within one event, a call a round.
const harnessSettleTicks = 8

// harnessOutcome is the end of the host operation a class gives a call (docs/adr/0087,
// section 9.4): an answer by its `finish`, a failure by its code, the refusal, and a
// held call, whichever way it is left.
func harnessOutcome(class map[string]any) string {
	switch class["end"] {
	case "answer":
		return "answer:" + class["finish"].(string)
	case "failure":
		return class["error"].(string)
	case "refusal":
		return "invalid_argument"
	default:
		return "hold"
	}
}

// harnessRun is what one run of the harness against the exhaustive provider showed.
type harnessRun struct {
	rows  []harnessRow
	calls int
	ended string
	codes []string
}

func harnessRowName(row harnessRow) string {
	return fmt.Sprintf("%s[%d]", row.Source.String(), row.Index)
}

// runHarness runs the reference harness against the exhaustive provider answering the
// classes sequence names, one call a class: it offers the table's ids, starts it, and
// leaves each call it holds as the table's harness section says, until it ends.
func runHarness(t *testing.T, table map[string]any, sequence []int) harnessRun {
	classes := table["classes"].([]any)
	harness := table["harness"].(map[string]any)
	if harness["then"] != "advance" {
		t.Fatalf("a held call of the harness is left by its own deadline, not %v", harness["then"])
	}
	var provided []hf.OutcomeClass
	for _, i := range sequence {
		provided = append(provided, outcomeClass(classes[i].(map[string]any)))
	}
	policy := static_model_harness.NewStaticModelHarnessPolicy()
	policy.SessionID = sce.GenerateSessionID()
	host := hf.NewReferenceHost(hf.NewScriptedClock(), &scriptTransport{})
	host.SetAnswerSchemas(static_model_harness.AnswerSchemas)
	host.SetModelProvider(hf.NewExhaustiveProvider(provided))
	engine := sce.NewEngine[static_model_harness.StaticModelHarnessState, static_model_harness.StaticModelHarnessEvent](&policy)
	var rows []harnessRow
	engine.SetTransitionObserver(func(taken harnessRow) {
		rows = append(rows, taken)
	})
	engine.SetClock(sce.NewManualClock(0))
	hf.Serve(engine, host)
	if err := engine.Initialize(); err != nil {
		t.Fatal(err)
	}
	settle := func() {
		for i := 0; i < harnessSettleTicks; i++ {
			engine.Tick()
		}
	}
	settle()
	for _, id := range harness["offered"].([]any) {
		data, err := json.Marshal(map[string]any{"id": id})
		if err != nil {
			t.Fatal(err)
		}
		engine.RaiseExternalByName(harness["offer"].(string), string(data))
		settle()
	}
	start, err := json.Marshal(harness["data"])
	if err != nil {
		t.Fatal(err)
	}
	engine.RaiseExternalByName(harness["start"].(string), string(start))
	settle()
	leaf := func() string {
		name := ""
		for _, state := range engine.GetActiveStates() {
			if !policy.IsCompoundState(state) {
				name = state.String()
			}
		}
		return name
	}
	bound := int(harness["bound"].(float64))
	deadline := int64(harness["deadline_ms"].(float64))
	for waits := 0; leaf() != "accepted" && leaf() != "gave_up"; waits++ {
		if waits > bound {
			t.Fatalf("%v: the run neither ended nor held a call it could be moved past", sequence)
		}
		engine.AdvanceTimeMs(deadline)
		settle()
	}
	codes := []string{}
	for _, refusal := range policy.Refusals() {
		codes = append(codes, refusal.Code().String())
	}
	return harnessRun{rows: rows, calls: len(host.TakeStarts()), ended: leaf(), codes: codes}
}

// The reference harness is tested exhaustively, and its coverage is measured
// (docs/adr/0087, section 9.4 item 1, Landing order item 7): every sequence of `bound`
// outcome classes is a run, the transition observer of docs/adr/0089 tells each row of
// the emitted transition table the run takes, and the runs together take every row of
// the table, reach every refusal code, and reach every host outcome at every call site
// (a row that enters the state whose invoke is the call).
//
// The harness makes at most `bound` calls and the provider answers a call by its
// position alone, so every sequence of fewer classes the harness can consume is a
// prefix of a sequence run here, and none is asked past.
func TestEverySequenceOfOutcomeClassesTakesEveryRowOfTheReferenceHarness(t *testing.T) {
	text, err := os.ReadFile(filepath.Join("..", "..", "..", "..", "..", "tests", "host_features", "model_outcome_classes.json"))
	if err != nil {
		t.Fatalf("the class table is not readable: %v", err)
	}
	var table map[string]any
	if err := json.Unmarshal(text, &table); err != nil {
		t.Fatalf("the class table is JSON: %v", err)
	}
	harness := table["harness"].(map[string]any)
	if harness["machine"] != "static_model_harness" || len(static_model_harness.AnswerSchemas) == 0 ||
		table["answer_schema"] != static_model_harness.AnswerSchemas[0].ID {
		t.Fatalf("the table names another harness or answer schema: %v", table["answer_schema"])
	}
	classes := table["classes"].([]any)
	bound := int(harness["bound"].(float64))
	call := harness["call"].(string)
	if len(classes) == 0 || bound == 0 {
		t.Fatal("a table that names nothing judges nothing")
	}
	rows := map[harnessRow]bool{}
	sites := map[harnessRow]bool{}
	for _, row := range static_model_harness.StaticModelHarnessTransitions {
		pair := harnessRow{Source: row.From, Index: row.Index}
		rows[pair] = true
		if row.Targets == call {
			sites[pair] = true
		}
	}
	if len(sites) == 0 {
		t.Fatalf("no row of the table enters `%s`", call)
	}
	outcomes := map[string]bool{}
	for _, raw := range classes {
		outcomes[harnessOutcome(raw.(map[string]any))] = true
	}

	taken := map[harnessRow]bool{}
	codes := map[string]bool{}
	reached := map[string]bool{}
	runs, expected := 0, 1
	for i := 0; i < bound; i++ {
		expected *= len(classes)
	}
	sequence := make([]int, bound)
	for {
		var named []string
		for _, i := range sequence {
			named = append(named, classes[i].(map[string]any)["class"].(string))
		}
		run := runHarness(t, table, sequence)
		if run.ended != "accepted" && run.ended != "gave_up" {
			t.Fatalf("%v: the run ended in `%s`, not a final", named, run.ended)
		}
		var called []harnessRow
		for _, row := range run.rows {
			if sites[row] {
				called = append(called, row)
			}
			taken[row] = true
		}
		if len(called) != run.calls || run.calls > bound {
			t.Fatalf("%v: %d calls, %d of them made by a row that enters `%s`", named, run.calls, len(called), call)
		}
		for n, site := range called {
			reached[harnessRowName(site)+" "+harnessOutcome(classes[sequence[n]].(map[string]any))] = true
		}
		for _, code := range run.codes {
			codes[code] = true
		}
		runs++
		position := bound - 1
		for position >= 0 && sequence[position] == len(classes)-1 {
			sequence[position] = 0
			position--
		}
		if position < 0 {
			break
		}
		sequence[position]++
	}
	if runs != expected {
		t.Fatalf("%d sequences were run, not the %d of the bound", runs, expected)
	}

	var toldNoRow, neverTaken []string
	for row := range taken {
		if !rows[row] {
			toldNoRow = append(toldNoRow, harnessRowName(row))
		}
	}
	for row := range rows {
		if !taken[row] {
			neverTaken = append(neverTaken, harnessRowName(row))
		}
	}
	sort.Strings(toldNoRow)
	sort.Strings(neverTaken)
	if len(toldNoRow) != 0 {
		t.Errorf("the observer was told pairs that are no row of the table: %v", toldNoRow)
	}
	if len(neverTaken) != 0 {
		t.Errorf("every row of the harness's transition table is taken; never taken: %v", neverTaken)
	}

	deferred := map[string]bool{}
	for _, raw := range table["deferred"].([]any) {
		deferred[raw.(map[string]any)["code"].(string)] = true
	}
	expectedCodes := map[string]bool{}
	for _, raw := range classes {
		if why, ok := raw.(map[string]any)["expect"].(map[string]any)["why"].(map[string]any); ok {
			expectedCodes[why["code"].(string)] = true
		}
	}
	for _, code := range harnessApplicationCodes {
		expectedCodes[code] = true
	}
	for code := range deferred {
		expectedCodes[code] = true
	}
	if len(expectedCodes) != 15 {
		t.Fatalf("the fourteen lift codes and the application's own: %v", expectedCodes)
	}
	var neverReached, unknown []string
	for code := range expectedCodes {
		if !codes[code] && !deferred[code] {
			neverReached = append(neverReached, code)
		}
	}
	for code := range codes {
		if !expectedCodes[code] {
			unknown = append(unknown, code)
		}
	}
	sort.Strings(neverReached)
	sort.Strings(unknown)
	if len(neverReached) != 0 {
		t.Errorf("every refusal code is reached; never reached: %v", neverReached)
	}
	if len(unknown) != 0 {
		t.Errorf("the harness kept codes no class reaches: %v", unknown)
	}

	var notAtEverySite []string
	for site := range sites {
		for outcome := range outcomes {
			if key := harnessRowName(site) + " " + outcome; !reached[key] {
				notAtEverySite = append(notAtEverySite, key)
			}
		}
	}
	sort.Strings(notAtEverySite)
	if len(notAtEverySite) != 0 {
		t.Errorf("every host outcome is reached at every call site; never reached: %v", notAtEverySite)
	}
}
