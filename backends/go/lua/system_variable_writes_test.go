// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// §scxml-5.10 and §scxml-B-2-8: which writes to a system variable the binding
// refuses.
//
// §scxml-5.10: "The Processor MUST cause any attempt to change the value of a
// system variable to fail and MUST place the error 'error.execution' on the
// internal event queue." A refused write is a script that fails, which is
// what ExecuteScript reports and what the Processor turns into
// `error.execution`.
//
// The expectations are not this file's. They live in
// `tests/ecmascript/system_variable_writes.json`, one write per case with
// whether it is refused and a condition read back afterwards, and every
// engine reads the same file, beside its reader of `event_blank_fields.json`.
// The table's `session` is bound as the system variables and its `event` as
// the current event before each write.
//
// This binding's answer before the repair, read from the code on 2026-11-08:
// `_event` was a plain table and `_sessionid`, `_name` and `_ioprocessors`
// plain globals, so every write in the table landed and every refused case
// failed.

package scelua

import (
	"encoding/json"
	"os"
	"path/filepath"
	"reflect"
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"
)

// systemWritesSession is the session a case runs in: its id, its name and the
// location of each I/O processor, keyed by the processor's name.
type systemWritesSession struct {
	SessionID    string            `json:"sessionid"`
	Name         string            `json:"name"`
	IoProcessors map[string]string `json:"ioprocessors"`
}

// systemWritesEvent is the current event of every case; `data` is the
// payload text handed to the binding.
type systemWritesEvent struct {
	Name       string `json:"name"`
	Type       string `json:"type"`
	SendID     string `json:"sendid"`
	Origin     string `json:"origin"`
	OriginType string `json:"origintype"`
	InvokeID   string `json:"invokeid"`
	Data       string `json:"data"`
}

// systemWritesCase is one write. The members this reader acts on are
// pointers, so an absent one is told from an empty one.
type systemWritesCase struct {
	Group    string  `json:"group"`
	WriteLua *string `json:"write_lua"`
	Refused  *bool   `json:"refused"`
	AfterLua *string `json:"after_lua"`
	Clause   string  `json:"clause"`
}

type systemWritesTable struct {
	Session systemWritesSession `json:"session"`
	Event   systemWritesEvent   `json:"event"`
	Cases   []systemWritesCase  `json:"cases"`
}

// readSystemWritesTable finds the shared table from this file's own location,
// the same way the reader of event_blank_fields.json names its own.
func readSystemWritesTable(t *testing.T) systemWritesTable {
	t.Helper()
	// backends/go/lua → repository root
	path := filepath.Join("..", "..", "..", "tests", "ecmascript", "system_variable_writes.json")
	raw, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("cannot read the shared table at %s: %v", path, err)
	}
	var table systemWritesTable
	if err := json.Unmarshal(raw, &table); err != nil {
		t.Fatalf("cannot parse %s: %v", path, err)
	}
	// A floor, not an equality: adding a case must not have to touch this
	// number, but a table that stopped being read must not pass either.
	if len(table.Cases) < 18 {
		t.Fatalf("the shared system-variable table holds only %d case(s), so this is not measuring the surface it claims to",
			len(table.Cases))
	}
	if table.Session.SessionID == "" || table.Session.Name == "" || len(table.Session.IoProcessors) == 0 {
		t.Fatalf("the shared table's `session` names no id, name or I/O processor: %+v", table.Session)
	}
	if table.Event.Name == "" || table.Event.Type == "" {
		t.Fatalf("the shared table's `event` names no `name` or `type`: %+v", table.Event)
	}
	refused := 0
	for index, testCase := range table.Cases {
		// A case this reader cannot read is refused, not skipped: a skipped
		// case is a case that passes.
		if testCase.WriteLua == nil || testCase.Refused == nil || testCase.AfterLua == nil {
			t.Fatalf("case %d (%s) lacks one of `write_lua`, `refused`, `after_lua`", index, testCase.Group)
		}
		if *testCase.Refused {
			refused++
		}
	}
	// Both kinds must be present: a table of refusals alone passes a binding
	// that refuses every write, and one of controls alone a binding that
	// refuses none.
	if refused == 0 || refused == len(table.Cases) {
		t.Fatalf("the shared table holds %d refused case(s) of %d; it needs both kinds", refused, len(table.Cases))
	}
	return table
}

// ioProcessorsOf turns the table's processor map into the descriptors
// SetupSystemVariables takes.
func ioProcessorsOf(session systemWritesSession) []sce.IoProcessorDescriptor {
	descriptors := make([]sce.IoProcessorDescriptor, 0, len(session.IoProcessors))
	for name, location := range session.IoProcessors {
		descriptors = append(descriptors, sce.IoProcessorDescriptor{Name: name, Location: location})
	}
	return descriptors
}

// openSystemWritesSession creates the session the table describes, with its
// system variables and its current event bound. The engine is returned only
// if every step succeeded.
func openSystemWritesSession(t *testing.T, table systemWritesTable) (*LuaEngine, bool) {
	t.Helper()
	engine := NewLuaEngine()
	id := table.Session.SessionID
	if err := engine.CreateSession(id); err != nil {
		t.Fatalf("CreateSession: %v", err)
	}
	if err := engine.SetupSystemVariables(id, table.Session.Name, ioProcessorsOf(table.Session)); err != nil {
		t.Errorf("SetupSystemVariables: %v", err)
		return nil, false
	}
	if _, err := engine.SetCurrentEvent(id, sce.SetCurrentEventArgs{
		Name:       table.Event.Name,
		Data:       table.Event.Data,
		Type:       table.Event.Type,
		SendID:     table.Event.SendID,
		Origin:     table.Event.Origin,
		OriginType: table.Event.OriginType,
		InvokeID:   table.Event.InvokeID,
	}); err != nil {
		t.Errorf("SetCurrentEvent: %v", err)
		return nil, false
	}
	return engine, true
}

func TestAWriteToASystemVariableFails(t *testing.T) {
	table := readSystemWritesTable(t)
	id := table.Session.SessionID
	// Every case is reported, not just the first: a binding that guards the
	// globals and not `_event`'s fields fails different cases from one that
	// guards neither.
	read := 0
	for index, testCase := range table.Cases {
		engine, ok := openSystemWritesSession(t, table)
		if !ok {
			continue
		}
		sess, err := engine.getSession(id)
		if err != nil {
			t.Fatalf("case %d (%s): %v", index, testCase.Group, err)
		}
		top := sess.l.Top()
		err = engine.ExecuteScript(id, *testCase.WriteLua)
		switch {
		case *testCase.Refused && err == nil:
			t.Errorf("case %d (%s): %s was accepted; %s refuses it",
				index, testCase.Group, *testCase.WriteLua, testCase.Clause)
		case !*testCase.Refused && err != nil:
			t.Errorf("case %d (%s): %s was refused (%v); %s allows it",
				index, testCase.Group, *testCase.WriteLua, err, testCase.Clause)
		}
		// A refusal is reported, not left behind: the stack the next script
		// runs on is the one this one started on.
		if after := sess.l.Top(); after != top {
			t.Errorf("case %d (%s): %s left the Lua stack at %d, it started at %d",
				index, testCase.Group, *testCase.WriteLua, after, top)
		}
		// The value the write would have changed, read back: an engine must
		// not pass by reporting a failure while the write landed.
		if got, err := engine.EvaluateExpression(id, *testCase.AfterLua); err != nil {
			t.Errorf("case %d (%s): %s did not evaluate after %s: %v",
				index, testCase.Group, *testCase.AfterLua, *testCase.WriteLua, err)
		} else if got != true {
			t.Errorf("case %d (%s): after %s, %s answered %#v (%T); %s",
				index, testCase.Group, *testCase.WriteLua, *testCase.AfterLua, got, got, testCase.Clause)
		}
		engine.DestroySession(id)
		read++
	}
	if read != len(table.Cases) {
		t.Fatalf("read %d of the %d case(s) the table holds", read, len(table.Cases))
	}
}

// A system variable is bound as a read-only view whose own table is empty,
// so the host must read it as a script does. A raw walk of the view handed
// `_event` and `_ioprocessors` over as empty maps.
func TestASystemVariableReadsWholeFromTheHost(t *testing.T) {
	table := readSystemWritesTable(t)
	engine, ok := openSystemWritesSession(t, table)
	if !ok {
		return
	}
	id := table.Session.SessionID
	defer engine.DestroySession(id)

	event, err := engine.EvaluateExpression(id, "_event")
	if err != nil {
		t.Fatalf("_event did not evaluate: %v", err)
	}
	wantEvent := map[string]interface{}{
		"name":       table.Event.Name,
		"type":       table.Event.Type,
		"sendid":     table.Event.SendID,
		"origin":     table.Event.Origin,
		"origintype": table.Event.OriginType,
		"invokeid":   table.Event.InvokeID,
		// The payload text is `{"n": 1}`, decoded; an integral number is
		// handed over as an int64.
		"data": map[string]interface{}{"n": int64(1)},
	}
	if !reflect.DeepEqual(event, wantEvent) {
		t.Errorf("_event read as %#v, want %#v", event, wantEvent)
	}

	processors, err := engine.GetVariable(id, "_ioprocessors")
	if err != nil {
		t.Fatalf("_ioprocessors did not read: %v", err)
	}
	wantProcessors := make(map[string]interface{}, len(table.Session.IoProcessors))
	for name, location := range table.Session.IoProcessors {
		wantProcessors[name] = map[string]interface{}{"location": location}
	}
	if !reflect.DeepEqual(processors, wantProcessors) {
		t.Errorf("_ioprocessors read as %#v, want %#v", processors, wantProcessors)
	}
}

// A global the document names goes through the same refusal as a script's
// own write, and the refusal is an error from the call rather than a panic
// that takes the host down.
func TestADocumentGlobalNamedAfterASystemVariableIsAnError(t *testing.T) {
	table := readSystemWritesTable(t)
	engine, ok := openSystemWritesSession(t, table)
	if !ok {
		return
	}
	id := table.Session.SessionID
	defer engine.DestroySession(id)

	if err := engine.SetVariable(id, "_sessionid", "x"); err == nil {
		t.Errorf("SetVariable(_sessionid) was accepted")
	}
	if err := engine.SetVariableAsDOM(id, "_name", "<a/>"); err == nil {
		t.Errorf("SetVariableAsDOM(_name) was accepted")
	}
	if got, err := engine.EvaluateExpression(id, "_sessionid"); err != nil || got != table.Session.SessionID {
		t.Errorf("_sessionid reads %#v (%v) after a refused SetVariable, want %q", got, err, table.Session.SessionID)
	}
	if got, err := engine.EvaluateExpression(id, "_name"); err != nil || got != table.Session.Name {
		t.Errorf("_name reads %#v (%v) after a refused SetVariableAsDOM, want %q", got, err, table.Session.Name)
	}
	// The control: a name of the document's own is still written.
	if err := engine.SetVariable(id, "held", "x"); err != nil {
		t.Errorf("SetVariable(held): %v", err)
	} else if got, err := engine.GetVariable(id, "held"); err != nil || got != "x" {
		t.Errorf("held reads %#v (%v), want \"x\"", got, err)
	}
}

// A system variable that cannot be bound is the engine's error, not a session
// that silently runs without it. Removing the shared binder is the one way a
// session can lose it.
func TestASystemVariableThatCannotBeBoundIsAnError(t *testing.T) {
	table := readSystemWritesTable(t)
	engine := NewLuaEngine()
	id := table.Session.SessionID
	if err := engine.CreateSession(id); err != nil {
		t.Fatalf("CreateSession: %v", err)
	}
	defer engine.DestroySession(id)
	if err := engine.ExecuteScript(id, "_scxml_bind_system = nil"); err != nil {
		t.Fatalf("removing the binder failed: %v", err)
	}
	if err := engine.SetupSystemVariables(id, table.Session.Name, ioProcessorsOf(table.Session)); err == nil {
		t.Errorf("SetupSystemVariables succeeded with no binder")
	}
	if _, err := engine.SetCurrentEvent(id, sce.SetCurrentEventArgs{
		Name: table.Event.Name,
		Type: table.Event.Type,
	}); err == nil {
		t.Errorf("SetCurrentEvent succeeded with no binder")
	}
}
