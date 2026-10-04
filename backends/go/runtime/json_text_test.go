// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A string written into JSON text is written one way on every engine
// (ARCHITECTURE.md, "JSON Text (Single Source of Truth)"). The cases live in
// tests/json_text/string_escape.json, read here with encoding/json, so this
// writer is measured against the same table as every other engine's.

package sce

import (
	"encoding/json"
	"math"
	"os"
	"path/filepath"
	"strconv"
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

// A float is spelled the way ECMAScript spells it by every writer that spells
// one (ARCHITECTURE.md, "JSON Number Text"). The cases live in
// tests/json_text/real_text.json, read here with encoding/json.
func TestAFloatIsSpelledAsEcmascriptSpellsItByEveryWriter(t *testing.T) {
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
	for _, c := range table.Cases {
		pattern, err := strconv.ParseUint(c.Bits, 16, 64)
		if err != nil {
			t.Fatalf("%s: bits %q: %v", c.Name, c.Bits, err)
		}
		value := math.Float64frombits(pattern)
		if got := NumberText(value); got != c.Text {
			t.Errorf("%s: NumberText wrote %q, the table says %q", c.Name, got, c.Text)
		}
		if got := ScriptValueToJSON(value); got != c.Text {
			t.Errorf("%s: as JSON wrote %q, the table says %q", c.Name, got, c.Text)
		}
		if got := ToWireString(value); got != c.Text {
			t.Errorf("%s: as an untyped param wrote %q, the table says %q", c.Name, got, c.Text)
		}
		if got, want := PayloadJSON(Pair("x", value)), `{"x":`+c.Text+`}`; got != want {
			t.Errorf("%s: as a payload field wrote %s, the table says %s", c.Name, got, want)
		}
	}
}

// A float that is not finite has no JSON spelling, and the untyped wire text
// gives it the one ECMAScript does.
func TestAFloatThatIsNotFiniteHasNoJsonSpellingAndAWireOne(t *testing.T) {
	for _, c := range []struct {
		value float64
		wire  string
	}{
		{math.NaN(), "NaN"},
		{math.Inf(1), "Infinity"},
		{math.Inf(-1), "-Infinity"},
	} {
		if got := ScriptValueToJSON(c.value); got != "null" {
			t.Errorf("%v as JSON wrote %q, want null", c.value, got)
		}
		if got := ToWireString(c.value); got != c.wire {
			t.Errorf("%v as an untyped param wrote %q, want %q", c.value, got, c.wire)
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
