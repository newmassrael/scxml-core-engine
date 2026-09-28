// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// A datamodel variable may be spelled as a Lua keyword (`local`, `end`), which
// the frontend reads as `_ENV["name"]`; this engine's ReferenceError check has
// to look at that name, not at `_ENV`. The cases are
// tests/scripting/undeclared_reads.json, which the Rust and Python engines'
// checks read too.

package scelua

import (
	"fmt"
	"path/filepath"
	"testing"
)

func TestAReadIsRefusedExactlyWhenTheSharedTableSays(t *testing.T) {
	var table struct {
		Cases []struct {
			Declared []string `json:"declared"`
			Expr     string   `json:"expr"`
			Refused  bool     `json:"refused"`
		} `json:"cases"`
	}
	readJSONFile(t, filepath.Join(repoRoot(t), "tests", "scripting", "undeclared_reads.json"), &table)
	// A floor: an empty table would pass every assertion below.
	if len(table.Cases) < 10 {
		t.Fatalf("the table lost cases: %d", len(table.Cases))
	}
	for i, c := range table.Cases {
		engine := NewLuaEngine()
		session := fmt.Sprintf("s%d", i)
		if err := engine.CreateSession(session); err != nil {
			t.Fatalf("create session: %v", err)
		}
		for _, name := range c.Declared {
			if err := engine.SetVariable(session, name, "#"+name); err != nil {
				t.Fatalf("declare %q: %v", name, err)
			}
		}
		_, err := engine.EvaluateExpression(session, c.Expr)
		if (err != nil) != c.Refused {
			t.Errorf("%q with %v: refused=%v (%v), want %v", c.Expr, c.Declared, err != nil, err, c.Refused)
		}
	}
}
