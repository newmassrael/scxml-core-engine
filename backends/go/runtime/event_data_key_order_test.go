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
