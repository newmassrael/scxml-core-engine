// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// §scxml-B-2-8-1: which reading an arriving payload gets.
//
// The expectations are not this file's. They live in
// `tests/ecmascript/event_data_readings.json`, one payload per case with the
// sentence of the clause that decides it, and the two C++ engines, the two
// Kotlin engines, the Rust binding and the Python binding read the same file.
// A per-backend copy drifts toward the backend that reads it, which is the
// blindness that let eight engines give four different answers to one clause.
//
// This binding's answer, measured 2026-08-19 before the repair: a payload that
// opens with `<` and is not a well-formed document arrived as nil. The leading
// `<` sent it down the DOM rung and the failed parse ended there, so the
// clause's closing sentence — "Otherwise, the Processor MUST treat the content
// as a space-normalized string literal" — was never reached.

package scelua

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"
)

type eventDataCase struct {
	Payload string                `json:"payload"`
	Source  string                `json:"source"`
	Lua     string                `json:"lua"`
	Clause  string                `json:"clause"`
	Expect  domSurfaceExpectation `json:"expect"`
}

type eventDataTable struct {
	Cases []eventDataCase `json:"cases"`
}

// readEventDataTable finds the shared table from this file's own location,
// the same way every other reader names it.
func readEventDataTable(t *testing.T) eventDataTable {
	t.Helper()
	// backends/go/lua → repository root
	path := filepath.Join("..", "..", "..", "tests", "ecmascript", "event_data_readings.json")
	raw, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("cannot read the shared table at %s: %v", path, err)
	}
	var table eventDataTable
	if err := json.Unmarshal(raw, &table); err != nil {
		t.Fatalf("cannot parse %s: %v", path, err)
	}
	// A floor, not an equality: adding a case must not have to touch this
	// number, but a table that stopped being read must not pass either.
	if len(table.Cases) < 8 {
		t.Fatalf("the shared reading table produced only %d case(s), so this is not measuring the surface it claims to",
			len(table.Cases))
	}
	return table
}

// deliverAndRead hands one payload to the binding the way an arriving event
// does, then evaluates one lowered expression against the `_event` it built.
func deliverAndRead(t *testing.T, payload, expr string) interface{} {
	t.Helper()
	engine := NewLuaEngine()
	if err := engine.CreateSession("reading"); err != nil {
		t.Fatalf("CreateSession: %v", err)
	}
	defer engine.DestroySession("reading")
	if err := engine.SetCurrentEvent("reading", sce.SetCurrentEventArgs{
		Name: "brief",
		Data: payload,
		Type: "external",
	}); err != nil {
		t.Fatalf("SetCurrentEvent: %v", err)
	}
	value, err := engine.EvaluateExpression("reading", expr)
	if err != nil {
		t.Fatalf("evaluating %q for payload %q: %v", expr, payload, err)
	}
	return value
}

func TestEventDataReadsEveryPayloadTheClauseNames(t *testing.T) {
	table := readEventDataTable(t)
	// Every case is reported, not just the first: an engine that drops the
	// fall-through is a different defect from one that runs the payload, and
	// one failure cannot separate them.
	for _, testCase := range table.Cases {
		got := deliverAndRead(t, testCase.Payload, testCase.Lua)
		if !domSurfaceMatches(got, testCase.Expect) {
			t.Errorf("payload %q: %s answered %#v (%T), %s says %s",
				testCase.Payload, testCase.Lua, got, got, testCase.Clause,
				describeExpectation(testCase.Expect))
		}
	}
}

// TestAPayloadThatIsACallLeavesTheSessionAlone is the sharper half of the
// expression case, which the shared table cannot ask because the side effect
// is spelled in the receiver's own language.
func TestAPayloadThatIsACallLeavesTheSessionAlone(t *testing.T) {
	engine := NewLuaEngine()
	if err := engine.CreateSession("s"); err != nil {
		t.Fatalf("CreateSession: %v", err)
	}
	defer engine.DestroySession("s")
	if err := engine.ExecuteScript("s", "breached = false"); err != nil {
		t.Fatalf("ExecuteScript: %v", err)
	}
	if err := engine.SetCurrentEvent("s", sce.SetCurrentEventArgs{
		Name: "brief",
		Data: "(function() breached = true return 'x' end)()",
		Type: "external",
	}); err != nil {
		t.Fatalf("SetCurrentEvent: %v", err)
	}
	breached, err := engine.EvaluateExpression("s", "breached")
	if err != nil {
		t.Fatalf("EvaluateExpression: %v", err)
	}
	if breached != false {
		t.Errorf("the payload ran: a host, a peer session or an HTTP sender could write this session's globals by naming them in event data")
	}
}
