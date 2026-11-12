// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// §scxml-6.4 on the Go channel (docs/adr/0098): a value an invoke hands its
// child is declared as the initial value of the child's top-level `<data>` it
// names, in place of that `<data>`'s own, which is never evaluated; a state's
// `<data>` never takes one; a name no top-level `<data>` carries is not added;
// an object arrives whole; and the child's top-level `<script>` and initial
// `<onentry>` read what was handed.
//
// The child reports what it saw to its parent, which ends in a `<final>` named
// for the first check that failed. The script engine is a double over the Lua
// engine, shared by the parent and its child as a host's would be, that
// records every expression and script it is asked to run and every value it
// is asked to write, so "the `<data>`'s own value is not evaluated" is read
// from the engine as well as from the child's report.
//
// Fixture: backends/go/tests/integration/a_go_child_takes_a_handed_value_in_place_of_its_data/a_go_child_takes_a_handed_value_in_place_of_its_data.scxml,
// a Go-only document beside this test; tools/codegen/trees/go-integration.toml
// generates the machines beside it.
//
// Regeneration (after fixture or template edit):
//   scripts/materialize.sh go-integration

package a_go_child_takes_a_handed_value_in_place_of_its_data

import (
	"errors"
	"strings"
	"testing"
	"time"

	sce "github.com/newmassrael/sce-go-runtime"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
)

// recordingEngine is the Lua engine, recording what the document asks of it,
// with every write of one name refused on request. The methods it does not
// name are the Lua engine's, unchanged.
type recordingEngine struct {
	sce.IScriptEngine
	// refuse is the name whose every write the double refuses; empty refuses
	// none.
	refuse string
	// ran is every expression evaluated and every script executed, as the
	// engine was handed it, in order.
	ran []string
	// written is every value a write was asked to give each name, in order.
	written map[string][]interface{}
}

func newRecordingEngine(refuse string) *recordingEngine {
	return &recordingEngine{
		IScriptEngine: scegotest.NewLuaEngine(),
		refuse:        refuse,
		written:       map[string][]interface{}{},
	}
}

func (d *recordingEngine) EvaluateExpression(sessionID, expr string) (interface{}, error) {
	d.ran = append(d.ran, expr)
	return d.IScriptEngine.EvaluateExpression(sessionID, expr)
}

func (d *recordingEngine) ExecuteScript(sessionID, script string) error {
	d.ran = append(d.ran, script)
	return d.IScriptEngine.ExecuteScript(sessionID, script)
}

func (d *recordingEngine) SetVariable(sessionID, name string, value interface{}) error {
	d.written[name] = append(d.written[name], value)
	if name == d.refuse {
		return errors.New("the double holds no value named " + name)
	}
	return d.IScriptEngine.SetVariable(sessionID, name, value)
}

// ownValues are pieces of the two `<data>` expressions of the child whose
// evaluation is observable — `undefined.invalidProperty` raises, and
// `trail.push('pushed')` has an effect — as they reach the engine. Neither
// appears in any other expression of the document, so an evaluation that
// carries one is the evaluation of that `<data>`'s own value.
var ownValues = []string{"invalidProperty", "pushed"}

// assertNoOwnValueRan fails the test for every evaluation of a seeded
// `<data>`'s own value the double recorded.
func assertNoOwnValueRan(t *testing.T, double *recordingEngine) {
	t.Helper()
	for _, ran := range double.ran {
		for _, own := range ownValues {
			if strings.Contains(ran, own) {
				t.Errorf("the child evaluated the own value of a <data> it was handed a value for: %q", ran)
			}
		}
	}
}

type handingEngine = sce.Engine[AGoChildTakesAHandedValueInPlaceOfItsDataState, AGoChildTakesAHandedValueInPlaceOfItsDataEvent]

// runOver starts the fixture over double and runs it until it ends, failing
// the test if it does not.
func runOver(t *testing.T, double *recordingEngine) *handingEngine {
	t.Helper()
	policy := NewAGoChildTakesAHandedValueInPlaceOfItsDataPolicy()
	policy.SessionID = sce.GenerateSessionID()
	policy.ScriptEngine = double
	engine := sce.NewEngine[AGoChildTakesAHandedValueInPlaceOfItsDataState, AGoChildTakesAHandedValueInPlaceOfItsDataEvent](&policy)
	if err := engine.Initialize(); err != nil {
		t.Fatalf("the parent starts: %v", err)
	}
	if !engine.RunUntilCompletion(3*time.Second, 10*time.Millisecond) {
		t.Fatalf("the parent did not end; it is in %v", engine.GetCurrentState())
	}
	return engine
}

func TestAChildTakesEachHandedValueInPlaceOfItsTopLevelData(t *testing.T) {
	double := newRecordingEngine("")
	engine := runOver(t, double)

	got, ended := engine.TerminalState()
	if !ended || got != AGoChildTakesAHandedValueInPlaceOfItsDataStatePass {
		t.Fatalf(
			"the parent ended in %v, want pass. "+
				"sawErrorExecution: the child raised error.execution, so `risky`'s own "+
				"`undefined.invalidProperty` was evaluated though a value was handed for it. "+
				"sawStrayAdded: `stray`, which names no <data> of the child, became a variable "+
				"there. "+
				"sawStateDataSeeded: `stateOwned`, a <data> of the state `report`, took the "+
				"handed value; only the top-level datamodel is seeded. "+
				"sawOwnValueEvaluated: `noted`'s own `trail.push('pushed')` ran. "+
				"sawOwnValueKept: `risky` or `noted` kept its own value; nothing was handed. "+
				"sawScriptReadOwnValue: the top-level <script> ran before `risky` was "+
				"declared with the handed value. "+
				"sawEntryReadOwnValue: the initial <onentry> did not read the handed `noted`. "+
				"sawParcelBroken: the object arrived without its structure. "+
				"sawParentError: the parent could not evaluate what it hands.",
			got,
		)
	}
	assertNoOwnValueRan(t, double)
	if risky := double.written["risky"]; len(risky) != 1 || risky[0] != "handed" {
		t.Errorf("`risky` is written once, with the value its parent handed, and was written %v", risky)
	}
	if stray := double.written["stray"]; len(stray) != 0 {
		t.Errorf("`stray` names no <data> of the child and is never written, and was written %v", stray)
	}
}

func TestAHandedValueTheSessionCannotHoldIsThatDataInitializationError(t *testing.T) {
	// The double refuses every write of `risky`, so the value handed for it
	// cannot be held: §scxml-5.3 makes that the error of `risky`'s
	// initialization, error.execution in the child, which the child reports
	// before anything else.
	double := newRecordingEngine("risky")
	engine := runOver(t, double)

	got, ended := engine.TerminalState()
	if !ended || got != AGoChildTakesAHandedValueInPlaceOfItsDataStateSawErrorExecution {
		t.Fatalf("a handed value the child's session cannot hold raises error.execution in the "+
			"child, which ends its parent in sawErrorExecution, and the parent ended in %v (%v)", got, ended)
	}
	// The value that could not be held is the handed one, and the `<data>`'s
	// own was not evaluated in its place.
	if risky := double.written["risky"]; len(risky) == 0 || risky[0] != "handed" {
		t.Errorf("the first write of `risky` is the value its parent handed, and the writes were %v", risky)
	}
	assertNoOwnValueRan(t, double)
}
