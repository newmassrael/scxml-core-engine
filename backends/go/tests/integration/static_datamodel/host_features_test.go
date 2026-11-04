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
	path := filepath.Join("..", "..", "..", "..", "..", "tests", "host_features", "scenarios", name+".json")
	text, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("the scenario is not readable: %v", err)
	}
	var scenario map[string]any
	if err := json.Unmarshal(text, &scenario); err != nil {
		t.Fatalf("a scenario is JSON: %v", err)
	}
	clock := hf.NewScriptedClock()
	transport := &scriptTransport{}
	rig := &hostRig{host: hf.NewReferenceHost(clock, transport), clock: clock, transport: transport, model: hf.NewScriptedProvider()}
	rig.host.SetModelProvider(rig.model)
	rig.host.SetAnswerSchemas(schemas)

	engine := sce.NewEngine[S, E](policy)
	engine.SetClock(sce.NewManualClock(0))
	hf.Serve(engine, rig.host)
	engine.Initialize()
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
	engine.Initialize()
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
	if len(codes) != 13 {
		t.Fatalf("every code of sce:std/lift/refusal_code.scxml is reached or deferred: %v", codes)
	}
}
