// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//go:build sce_trace_port

// Each scenario of the properties judge's turnstile, replayed by the Go lane
// (docs/adr/0115, D15 and the converse of D8). A scenario is the run TLC gave of
// a violation or a witness, with the events its environment put on the external
// queue; this test gives the machine those events, on a manual clock, with its
// trace port on, and holds the run the port is told to the recorded run
// `<name>.go.json` beside the scenario, byte for byte. `sce_build::formal::replay`
// judges each recorded run to be the scenario's, step for step
// (sce-build/tests/a_scenario_is_the_run_of_every_lane.rs), as it judges the Rust
// and the C11 lanes'. The twin of the Rust lane's
// a_scenario_of_the_turnstile_is_its_run and of the C11 lane's
// c11_a_scenario_of_the_turnstile_is_its_run.
//
// The harness gives an input once the machine has taken every step it can,
// which is after the step the input names unless the model's environment put it
// on the queue in the middle of a macrostep. A macrostep that also sends to its
// own external queue after that point would take the two in another order here;
// the judge then finds the runs part, which is a defect of this harness to
// settle, and not a verdict about the document. A delayed send comes due by the
// clock moving to the next deadline, which is the earliest: a scenario in which
// the model's time makes a later one due first is one no clock replays, and the
// judge finds that too.
//
// The machine is the formal fixture's (the go-integration tree's directory
// integration/formal_turnstile); the scenarios are written by
// `sce-codegen formal properties --scenarios
// sce-build/tests/fixtures/formal_properties/scenarios`. Recording the lane's
// runs again, after a change that moves a run on purpose, from backends/go/tests:
//
//	UPDATE_EXPECT=1 go test -tags sce_trace_port ./run_trace/ -run TestEachScenarioOfTheTurnstileIsItsRun -count=1
//
// Fixture: sce-build/tests/fixtures/formal_properties/turnstile.scxml
package run_trace

import (
	"bytes"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"sort"
	"strings"
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
	turnstile "github.com/newmassrael/sce-go-tests/integration/formal_turnstile"
)

// scenarioSuffix is what a scenario's file is named by after its name
// (sce_build::formal::replay::SCENARIO_SUFFIX).
const scenarioSuffix = ".scenario.json"

// scenarioInput is one event the environment put on the external queue during
// a scenario: `arrive`, from outside, or `due`, a delayed send come due, with
// the count of steps of the run that were over when it came, from 1.
type scenarioInput struct {
	Kind  string `json:"kind"`
	After *int   `json:"after"`
	Event string `json:"event"`
}

// scenarioDocument is a scenario as the run trace form writes it, which
// sce_build::run_trace reads of reference: the steps and the cycle are read
// there and stepped over here, and a member the form does not have is refused.
type scenarioDocument struct {
	Version int               `json:"version"`
	Lane    string            `json:"lane"`
	Steps   []json.RawMessage `json:"steps"`
	Inputs  []scenarioInput   `json:"inputs"`
	Cycle   *int              `json:"cycle"`
}

// scenariosDir is the directory of the turnstile's scenarios and of each
// lane's run of them, from this package's directory.
func scenariosDir() string {
	return filepath.Join("..", "..", "..", "..", "sce-build", "tests", "fixtures", "formal_properties", "scenarios")
}

// scenarioNames is each scenario in dir, by its name, in the order of the names.
func scenarioNames(t *testing.T, dir string) []string {
	t.Helper()
	entries, err := os.ReadDir(dir)
	if err != nil {
		t.Fatalf("%s cannot be read (%v): write the scenarios with `sce-codegen formal properties --scenarios`", dir, err)
	}
	var names []string
	for _, entry := range entries {
		if name, ok := strings.CutSuffix(entry.Name(), scenarioSuffix); ok && name != "" {
			names = append(names, name)
		}
	}
	sort.Strings(names)
	return names
}

// readInputs is the inputs of the scenario at path. An input with a member the
// form does not have, or without one it has, is refused rather than replayed in
// part.
func readInputs(path string) ([]scenarioInput, error) {
	text, err := os.ReadFile(path)
	if err != nil {
		return nil, err
	}
	decoder := json.NewDecoder(bytes.NewReader(text))
	decoder.DisallowUnknownFields()
	var document scenarioDocument
	if err := decoder.Decode(&document); err != nil {
		return nil, err
	}
	for at, input := range document.Inputs {
		if input.Kind != "arrive" && input.Kind != "due" {
			return nil, fmt.Errorf("inputs[%d] is of the kind %q, which the form does not have", at, input.Kind)
		}
		if input.After == nil || *input.After < 1 || input.Event == "" {
			return nil, fmt.Errorf("inputs[%d] has no `after` from 1 or no `event`", at)
		}
	}
	return document.Inputs, nil
}

// replay is the run the trace port is told when the turnstile is given inputs.
func replay(t *testing.T, name string, inputs []scenarioInput) *sce.RunTrace {
	t.Helper()
	trace := &sce.RunTrace{}
	policy := turnstile.NewTurnstilePolicy()
	policy.SessionID = sce.GenerateSessionID()
	policy.ScriptEngine = scegotest.NewLuaEngine()
	engine := sce.NewEngine[turnstile.TurnstileState, turnstile.TurnstileEvent](&policy)
	// Before the start: the entry configuration's <onentry> may arm a delayed send.
	engine.SetClock(sce.NewManualClock(0))
	engine.SetTracePort(trace.Push)
	if err := engine.Initialize(); err != nil {
		t.Fatalf("%s: the machine did not start: %v", name, err)
	}
	for _, input := range inputs {
		if told := len(trace.Steps()); told < *input.After {
			t.Fatalf("%s: the machine took %d steps, and the model %d before `%s` came",
				name, told, *input.After, input.Event)
		}
		if input.Kind == "arrive" {
			engine.RaiseExternalByName(input.Event, "")
			engine.Tick()
			continue
		}
		wait, ok := engine.TimeUntilNextScheduled()
		if !ok {
			t.Fatalf("%s: `%s` comes due, and the machine holds no delayed send", name, input.Event)
		}
		engine.AdvanceTimeMs(wait.Milliseconds())
	}
	return trace
}

// holdsToTheRecordedScenarioRun holds trace to the recorded run at path, or
// records it again when UPDATE_EXPECT is set.
func holdsToTheRecordedScenarioRun(t *testing.T, path string, trace *sce.RunTrace) {
	t.Helper()
	text := trace.JSON()
	if _, set := os.LookupEnv("UPDATE_EXPECT"); set {
		if err := os.WriteFile(path, []byte(text), 0o644); err != nil {
			t.Fatalf("%s is not writable: %v", path, err)
		}
		return
	}
	recorded, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("%s cannot be read (%v): record it with `UPDATE_EXPECT=1 go test -tags sce_trace_port "+
			"./run_trace/ -run TestEachScenarioOfTheTurnstileIsItsRun` in backends/go/tests", path, err)
	}
	if string(recorded) != text {
		t.Fatalf("the run of %s is not the recorded one the replay judge reads.\nThe run:\n%s", path, text)
	}
}

// Each scenario of the turnstile is its run on the Go lane: the machine, given
// the scenario's inputs, tells its trace port the run recorded beside it.
func TestEachScenarioOfTheTurnstileIsItsRun(t *testing.T) {
	dir := scenariosDir()
	names := scenarioNames(t, dir)
	if len(names) == 0 {
		t.Fatalf("%s holds no scenario: write them with `sce-codegen formal properties --scenarios`", dir)
	}
	for _, name := range names {
		inputs, err := readInputs(filepath.Join(dir, name+scenarioSuffix))
		if err != nil {
			t.Fatalf("the inputs of the scenario %s are not read: %v", name, err)
		}
		trace := replay(t, name, inputs)
		holdsToTheRecordedScenarioRun(t, filepath.Join(dir, name+".go.json"), trace)
	}
}
