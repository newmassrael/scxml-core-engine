// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A string written into JSON text is written one way on every engine
// (ARCHITECTURE.md, "JSON Text (Single Source of Truth)"). The cases live in
// tests/json_text/string_escape.json, read here with encoding/json, so this
// writer is measured against the same table as every other engine's.

package sce

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

func TestAStringIsWrittenInTheOneFormEveryEngineWrites(t *testing.T) {
	raw, err := os.ReadFile(filepath.Join("..", "..", "..", "tests", "json_text", "string_escape.json"))
	if err != nil {
		t.Fatalf("the shared escape table: %v", err)
	}
	var table struct {
		Cases []struct {
			Name    string `json:"name"`
			Text    string `json:"text"`
			Escaped string `json:"escaped"`
		} `json:"cases"`
	}
	if err := json.Unmarshal(raw, &table); err != nil {
		t.Fatalf("the table is JSON: %v", err)
	}
	if len(table.Cases) < 8 {
		t.Fatalf("the table lost cases: %d", len(table.Cases))
	}
	for _, c := range table.Cases {
		if got := EscapeJSONString(c.Text); got != c.Escaped {
			t.Errorf("%s: wrote %q, the table says %q", c.Name, got, c.Escaped)
		}
	}
}

// An inject seam's payload is written in the order its schema declares the
// fields, as every other engine writes it — not in name order, which is what
// the map this took until 2026-09-28 produced.
func TestAPayloadKeepsTheOrderItsSchemaDeclares(t *testing.T) {
	got := PayloadJSON(Pair("payload", "q\"\u0001"), Pair("offset", 7), Pair("flag", true))
	want := `{"payload":"q\"\u0001","offset":7,"flag":true}`
	if got != want {
		t.Errorf("wrote %s, want %s", got, want)
	}
	if got := PayloadJSON(); got != "{}" {
		t.Errorf("an empty payload wrote %s", got)
	}
}
