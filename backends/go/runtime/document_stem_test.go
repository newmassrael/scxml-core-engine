// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The value a hybrid <invoke srcexpr> computes is reduced to the stem of the
// document it names (docs/SCE_ACCEPTED_SUBSET.md §2.13), and every engine
// reduces it the same way. The cases live in
// tests/document_stem/document_stem.json, read here with encoding/json, so
// this reader is measured against the same table as every other engine's and
// the build's.

package sce

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

func TestAValueIsReducedToTheOneStemEveryEngineReducesItTo(t *testing.T) {
	raw, err := os.ReadFile(filepath.Join("..", "..", "..", "tests", "document_stem", "document_stem.json"))
	if err != nil {
		t.Fatalf("the shared document-stem table: %v", err)
	}
	var table struct {
		Cases []struct {
			Name  string `json:"name"`
			Value string `json:"value"`
			Stem  string `json:"stem"`
		} `json:"cases"`
	}
	if err := json.Unmarshal(raw, &table); err != nil {
		t.Fatalf("the table is JSON: %v", err)
	}
	if len(table.Cases) < 15 {
		t.Fatalf("the table lost cases: %d", len(table.Cases))
	}
	for _, c := range table.Cases {
		if got := DocumentStem(c.Value); got != c.Stem {
			t.Errorf("%s: DocumentStem(%q) = %q, want %q", c.Name, c.Value, got, c.Stem)
		}
	}
}
