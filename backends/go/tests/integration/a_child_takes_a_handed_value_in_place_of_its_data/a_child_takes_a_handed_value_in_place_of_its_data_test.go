// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// §scxml-6.4 on the Go channel (docs/adr/0098): a value an invoke hands its
// child is declared as the initial value of the child's top-level `<data>` it
// names, in place of that `<data>`'s own, which is never evaluated; a state's
// `<data>`, early- or late-bound, never takes one; a name no top-level
// `<data>` carries is not added; an object arrives whole and as the child's
// own copy; and the child's top-level `<script>`, initial `<onentry>` and first
// macrostep read what was handed.
//
// Each child reports what it saw to its parent, which judges it and ends in a
// `<final>` named for the first check that failed. The script engine is a
// double over the Lua engine, shared by the parent and its children as a
// host's would be, that records every expression and script it is asked to run
// and every value it is asked to write, so "the `<data>`'s own value is not
// evaluated" is read from the engine as well as from the children's reports.
//
// Fixture: integration_resources/a_child_takes_a_handed_value_in_place_of_its_data/a_child_takes_a_handed_value_in_place_of_its_data.scxml
//
// Regeneration (after fixture or template edit):
//   scripts/materialize.sh go-integration

package a_child_takes_a_handed_value_in_place_of_its_data

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

// ownValues are pieces of the `<data>` expressions of the children whose
// evaluation is observable — `undefined.invalidProperty` raises, and
// `effects.push('own effectful ran')` has an effect — as they reach the engine.
// Neither appears in any other expression of the document, so an evaluation
// that carries one is the evaluation of a handed `<data>`'s own value.
var ownValues = []string{"invalidProperty", "own effectful ran"}

// assertNoOwnValueRan fails the test for every evaluation of a handed
// `<data>`'s own value the double recorded.
func assertNoOwnValueRan(t *testing.T, double *recordingEngine) {
	t.Helper()
	for _, ran := range double.ran {
		for _, own := range ownValues {
			if strings.Contains(ran, own) {
				t.Errorf("a child evaluated the own value of a <data> it was handed a value for: %q", ran)
			}
		}
	}
}

type handingEngine = sce.Engine[AChildTakesAHandedValueInPlaceOfItsDataState, AChildTakesAHandedValueInPlaceOfItsDataEvent]

// runOver starts the fixture over double and runs it until it ends, failing
// the test if it does not.
func runOver(t *testing.T, double *recordingEngine) *handingEngine {
	t.Helper()
	policy := NewAChildTakesAHandedValueInPlaceOfItsDataPolicy()
	policy.SessionID = sce.GenerateSessionID()
	policy.ScriptEngine = double
	engine := sce.NewEngine[AChildTakesAHandedValueInPlaceOfItsDataState, AChildTakesAHandedValueInPlaceOfItsDataEvent](&policy)
	if err := engine.Initialize(); err != nil {
		t.Fatalf("the parent starts: %v", err)
	}
	if !engine.RunUntilCompletion(3*time.Second, 10*time.Millisecond) {
		t.Fatalf("the parent did not end; it is in %v. byParam / byNamelist / lateBound: that "+
			"phase's child never reported. firstMacrostep: the child's first macrostep read its own "+
			"value of `handed` and stands in `missed`", engine.GetCurrentState())
	}
	return engine
}

func TestAChildTakesEachHandedValueInPlaceOfItsTopLevelData(t *testing.T) {
	double := newRecordingEngine("")
	engine := runOver(t, double)

	got, ended := engine.TerminalState()
	if !ended || got != AChildTakesAHandedValueInPlaceOfItsDataStatePass {
		t.Fatalf(
			"the parent ended in %v, want pass. "+
				"failOwnValueRan: `effectful`'s own `effects.push(..)` ran though a value was "+
				"handed for it. "+
				"failChildRaised: the child raised error.execution, so `raising`'s own "+
				"`undefined.invalidProperty` was evaluated. "+
				"failHandedValueLost: a top-level <data> kept its own value; nothing was handed. "+
				"failStateDataTookTheValue / failLateStateDataTookTheValue: `kept`, a <data> of a "+
				"state, took the handed value; only the top-level datamodel is seeded. "+
				"failUnmatchedNameAdded / failNamelistNameAdded: a name no <data> of the child "+
				"declares became a variable there. "+
				"failObjectNotWhole: an object arrived without its structure. "+
				"failChildSharedTheParentsObject: the child was handed the parent's object, not a "+
				"copy. "+
				"failScriptReadItsOwnValue / failEntryReadItsOwnValue: the top-level <script> or the "+
				"initial <onentry> ran before the values were declared. "+
				"failNamelistOwnValueRaised / failNamelistValueLost: the same, for `namelist`. "+
				"failParentRaised: the parent could not evaluate what it hands.",
			got,
		)
	}
	assertNoOwnValueRan(t, double)
	if raising := double.written["raising"]; len(raising) != 1 || raising[0] != "handed raising" {
		t.Errorf("`raising` is written once, with the value its parent handed, and was written %v", raising)
	}
	if stranger := double.written["stranger"]; len(stranger) != 0 {
		t.Errorf("`stranger` names no <data> of the child and is never written, and was written %v", stranger)
	}
}

func TestAHandedValueTheSessionCannotHoldIsThatDataInitializationError(t *testing.T) {
	// The double refuses every write of `refusable`, so the value handed for it
	// cannot be held: §scxml-5.3 makes that the error of `refusable`'s
	// initialization, error.execution in the child. The parent judges every
	// other value before the child's error count, so it ends in
	// failChildRaised only with every other value in its place.
	double := newRecordingEngine("refusable")
	engine := runOver(t, double)

	got, ended := engine.TerminalState()
	if !ended || got != AChildTakesAHandedValueInPlaceOfItsDataStateFailChildRaised {
		t.Fatalf("a handed value the child's session cannot hold raises error.execution in the "+
			"child, which ends its parent in failChildRaised, and the parent ended in %v (%v)", got, ended)
	}
	// The value that could not be held is the handed one, and the `<data>`'s
	// own was not evaluated in its place.
	if refusable := double.written["refusable"]; len(refusable) == 0 || refusable[0] != "handed refusable" {
		t.Errorf("the first write of `refusable` is the value its parent handed, and the writes were %v", refusable)
	}
	assertNoOwnValueRan(t, double)
}
