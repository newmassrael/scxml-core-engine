// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A float is spelled the way ECMAScript spells it by every writer that spells
// one (ARCHITECTURE.md, "JSON Number Text"). The Lua `JSON.stringify` is shared
// by every Lua engine, and go-lua is the one that is not Lua 5.4: it has no
// integer subtype, so every number reaches the float path, and its string
// library is the one the shared file is written around. The cases live in
// tests/json_text/real_text.json, and the C++ build's Lua engine is held to the
// same table.

package scelua

import (
	"encoding/json"
	"math"
	"os"
	"path/filepath"
	"strconv"
	"testing"
)

func TestJSONStringifySpellsAFloatAsEcmascriptSpellsIt(t *testing.T) {
	raw, err := os.ReadFile(filepath.Join("..", "..", "..", "tests", "json_text", "real_text.json"))
	if err != nil {
		t.Fatalf("the shared real-text table: %v", err)
	}
	var table struct {
		Cases []struct {
			Name string `json:"name"`
			Bits string `json:"bits"`
			Text string `json:"text"`
		} `json:"cases"`
	}
	if err := json.Unmarshal(raw, &table); err != nil {
		t.Fatalf("the table is JSON: %v", err)
	}
	if len(table.Cases) < 40 {
		t.Fatalf("the table lost cases: %d", len(table.Cases))
	}
	e := NewLuaEngine()
	if err := e.CreateSession("s"); err != nil {
		t.Fatalf("CreateSession: %v", err)
	}
	for _, c := range table.Cases {
		pattern, err := strconv.ParseUint(c.Bits, 16, 64)
		if err != nil {
			t.Fatalf("%s: bits %q: %v", c.Name, c.Bits, err)
		}
		if err := e.SetVariable("s", "v", math.Float64frombits(pattern)); err != nil {
			t.Fatalf("%s: SetVariable: %v", c.Name, err)
		}
		got, err := e.EvaluateExpression("s", "JSON.stringify(v)")
		if err != nil {
			t.Errorf("%s: JSON.stringify: %v", c.Name, err)
			continue
		}
		if got != c.Text {
			t.Errorf("%s: JSON.stringify wrote %v, the table says %q", c.Name, got, c.Text)
		}
	}
}
