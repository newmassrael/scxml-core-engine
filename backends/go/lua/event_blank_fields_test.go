// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// §scxml-5.10.1 and §scxml-B-2-8: what an `_event` field the Processor left
// blank reads as.
//
// The expectations are not this file's. They live in
// `tests/ecmascript/event_blank_fields.json`, one event per case with the
// sentence that decides it, and every engine reads the same file, beside its
// reader of `event_data_readings.json`. An absent member of a case's `event`
// is a field the Processor left blank, handed to the binding as the empty
// string, which is how SetCurrentEventArgs carries it.
//
// This binding's answer before the repair, read from the code on 2026-11-07:
// it pushed `sendid`, `origin`, `origintype` and `invokeid` as strings for
// every event, so a blank one read "" and every `== nil` case answered false.

package scelua

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"
)

// blankFieldsEvent is the event a case hands the binding. `name` and `type`
// always have a value, so they are pointers that tell an absent member from
// an empty one; the other four read an absent member as the empty string.
type blankFieldsEvent struct {
	Name       *string `json:"name"`
	Type       *string `json:"type"`
	SendID     string  `json:"sendid"`
	InvokeID   string  `json:"invokeid"`
	Origin     string  `json:"origin"`
	OriginType string  `json:"origintype"`
}

type blankFieldsCase struct {
	Group  string                `json:"group"`
	Event  blankFieldsEvent      `json:"event"`
	Source string                `json:"source"`
	Lua    string                `json:"lua"`
	Clause string                `json:"clause"`
	Expect domSurfaceExpectation `json:"expect"`
}

type blankFieldsTable struct {
	Cases []blankFieldsCase `json:"cases"`
}

// readBlankFieldsTable finds the shared table from this file's own location,
// the same way the reader of event_data_readings.json names its own.
func readBlankFieldsTable(t *testing.T) blankFieldsTable {
	t.Helper()
	// backends/go/lua → repository root
	path := filepath.Join("..", "..", "..", "tests", "ecmascript", "event_blank_fields.json")
	raw, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("cannot read the shared table at %s: %v", path, err)
	}
	var table blankFieldsTable
	if err := json.Unmarshal(raw, &table); err != nil {
		t.Fatalf("cannot parse %s: %v", path, err)
	}
	// A floor, not an equality: adding a case must not have to touch this
	// number, but a table that stopped being read must not pass either.
	if len(table.Cases) < 11 {
		t.Fatalf("the shared blank-field table holds only %d case(s), so this is not measuring the surface it claims to",
			len(table.Cases))
	}
	for index, testCase := range table.Cases {
		// An expectation this reader cannot read is refused, not skipped: a
		// skipped case is a case that passes.
		if testCase.Expect.Bool == nil {
			t.Fatalf("case %d (%s) has no readable `expect.bool`", index, testCase.Group)
		}
		if testCase.Event.Name == nil || testCase.Event.Type == nil {
			t.Fatalf("case %d (%s) names no event `name` or `type`", index, testCase.Group)
		}
	}
	return table
}

func TestABlankEventFieldReadsAsNil(t *testing.T) {
	table := readBlankFieldsTable(t)
	// Every case is reported, not just the first: a binding that drops
	// `origin` always and one that binds it as "" fail different cases.
	read := 0
	for index, testCase := range table.Cases {
		engine := NewLuaEngine()
		if err := engine.CreateSession("blank"); err != nil {
			t.Fatalf("CreateSession: %v", err)
		}
		if _, err := engine.SetCurrentEvent("blank", sce.SetCurrentEventArgs{
			Name:       *testCase.Event.Name,
			Type:       *testCase.Event.Type,
			SendID:     testCase.Event.SendID,
			Origin:     testCase.Event.Origin,
			OriginType: testCase.Event.OriginType,
			InvokeID:   testCase.Event.InvokeID,
		}); err != nil {
			t.Errorf("case %d (%s): SetCurrentEvent: %v", index, testCase.Group, err)
		} else if got, err := engine.EvaluateExpression("blank", testCase.Lua); err != nil {
			t.Errorf("case %d (%s): %s did not evaluate: %v (%s)",
				index, testCase.Group, testCase.Lua, err, testCase.Clause)
		} else if !domSurfaceMatches(got, testCase.Expect) {
			t.Errorf("case %d (%s): %s answered %#v (%T), %s says %s",
				index, testCase.Group, testCase.Lua, got, got, testCase.Clause,
				describeExpectation(testCase.Expect))
		}
		engine.DestroySession("blank")
		read++
	}
	if read != len(table.Cases) {
		t.Fatalf("read %d of the %d case(s) the table holds", read, len(table.Cases))
	}
}
