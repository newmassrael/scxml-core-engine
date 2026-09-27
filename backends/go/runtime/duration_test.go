// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A <send> delay is read as one CSS2 time on every engine (ARCHITECTURE.md,
// "Durations (Single Source of Truth)"). The cases live in
// tests/durations/css2_time.json, read here with encoding/json, so this reader
// is measured against the same table as every other engine's.

package sce

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

func TestADelayIsReadAsTheOneCSS2TimeEveryEngineReads(t *testing.T) {
	raw, err := os.ReadFile(filepath.Join("..", "..", "..", "tests", "durations", "css2_time.json"))
	if err != nil {
		t.Fatalf("the shared duration table: %v", err)
	}
	var table struct {
		Cases []struct {
			Name string  `json:"name"`
			Text string  `json:"text"`
			Ms   *uint64 `json:"ms"`
		} `json:"cases"`
	}
	if err := json.Unmarshal(raw, &table); err != nil {
		t.Fatalf("the table is JSON: %v", err)
	}
	if len(table.Cases) < 20 {
		t.Fatalf("the table lost cases: %d", len(table.Cases))
	}
	for _, c := range table.Cases {
		got, ok := ParseDelayToMs(c.Text)
		switch {
		case c.Ms == nil && ok:
			t.Errorf("%s: read %q as %d ms, the table says it is not a time", c.Name, c.Text, got)
		case c.Ms != nil && !ok:
			t.Errorf("%s: refused %q, the table says %d ms", c.Name, c.Text, *c.Ms)
		case c.Ms != nil && got != *c.Ms:
			t.Errorf("%s: read %q as %d ms, the table says %d", c.Name, c.Text, got, *c.Ms)
		}
	}
}
