// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The members of a JSON object written into `_event.data` come in one order on
// every engine (ARCHITECTURE.md, "JSON Object Key Order"). The cases live in
// tests/json_text/object_key_order.json, read here with encoding/json, so this
// writer is measured against the same table as every other engine's.

package sce

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

func TestTheMembersOfAnObjectAreWrittenInTheOneOrderEveryEngineWrites(t *testing.T) {
	raw, err := os.ReadFile(filepath.Join("..", "..", "..", "tests", "json_text", "object_key_order.json"))
	if err != nil {
		t.Fatalf("the shared key-order table: %v", err)
	}
	var table struct {
		Cases []struct {
			Name   string               `json:"name"`
			Params [][2]json.RawMessage `json:"params"`
			Data   string               `json:"data"`
		} `json:"cases"`
	}
	if err := json.Unmarshal(raw, &table); err != nil {
		t.Fatalf("the table is JSON: %v", err)
	}
	if len(table.Cases) < 8 {
		t.Fatalf("the table lost cases: %d", len(table.Cases))
	}
	for _, c := range table.Cases {
		// The params a `<send>` evaluated, in the order the document
		// declared them.
		params := make([]EventDataParam, 0, len(c.Params))
		for _, pair := range c.Params {
			var name string
			if err := json.Unmarshal(pair[0], &name); err != nil {
				t.Fatalf("%s: a param name is a string: %v", c.Name, err)
			}
			var value interface{}
			if err := json.Unmarshal(pair[1], &value); err != nil {
				t.Fatalf("%s: a param value is JSON: %v", c.Name, err)
			}
			params = append(params, EventDataParam{Name: name, Value: value})
		}
		if got := BuildJSONFromTypedParams(params); got != c.Data {
			t.Errorf("%s: wrote %q, the table says %q", c.Name, got, c.Data)
		}
	}
}

// typedJSON spells a JSON value through the typed payload writers, one call per
// constructor, as a generated raise spells a value of its type: an object as a
// record (RecordJSON), an array as a list (ListJSON), anything else as a value
// of a scalar type (ValueJSON). encoding/json hands an object's members over in
// no order, so the record's writer is the only thing that orders them.
func typedJSON(value interface{}) RawJSON {
	switch v := value.(type) {
	case map[string]interface{}:
		members := make([]PayloadPair, 0, len(v))
		for name, member := range v {
			members = append(members, Pair(name, typedJSON(member)))
		}
		return RecordJSON(members...)
	case []interface{}:
		return ListJSON(v, typedJSON)
	default:
		return ValueJSON(v)
	}
}

// TestARecordAndAListAreWrittenInTheOneOrderEveryEngineWrites holds the typed
// writers a generated raise spells a record and a list with (docs/adr/0086,
// section 5.3) to the same table, at every depth. A case whose name repeats is
// a `<send>`'s list of params, which only the parameter writer above collects:
// a record holds each field once.
func TestARecordAndAListAreWrittenInTheOneOrderEveryEngineWrites(t *testing.T) {
	raw, err := os.ReadFile(filepath.Join("..", "..", "..", "tests", "json_text", "object_key_order.json"))
	if err != nil {
		t.Fatalf("the shared key-order table: %v", err)
	}
	var table struct {
		Cases []struct {
			Name   string               `json:"name"`
			Params [][2]json.RawMessage `json:"params"`
			Data   string               `json:"data"`
		} `json:"cases"`
	}
	if err := json.Unmarshal(raw, &table); err != nil {
		t.Fatalf("the table is JSON: %v", err)
	}
	written := 0
	for _, c := range table.Cases {
		members := make([]PayloadPair, 0, len(c.Params))
		seen := make(map[string]bool, len(c.Params))
		repeats := false
		for _, pair := range c.Params {
			var name string
			if err := json.Unmarshal(pair[0], &name); err != nil {
				t.Fatalf("%s: a param name is a string: %v", c.Name, err)
			}
			var value interface{}
			if err := json.Unmarshal(pair[1], &value); err != nil {
				t.Fatalf("%s: a param value is JSON: %v", c.Name, err)
			}
			repeats = repeats || seen[name]
			seen[name] = true
			members = append(members, Pair(name, typedJSON(value)))
		}
		if repeats {
			continue
		}
		if got := string(RecordJSON(members...)); got != c.Data {
			t.Errorf("%s: wrote %q, the table says %q", c.Name, got, c.Data)
		}
		written++
	}
	if written < 10 {
		t.Fatalf("the table lost the cases a record is written for: %d", written)
	}
}
