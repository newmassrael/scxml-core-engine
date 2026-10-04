// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// An infinity is Infinity, and the largest finite double is not.
//
// The shared `ecma_semantics.lua` names the infinity by `math.huge`. go-lua
// defines that as `math.MaxFloat64` where Lua 5.2 defines it as HUGE_VAL, so on
// this engine alone `1/0` was an ordinary number to the shared Lua: it was
// spelled by `tostring` (`+Inf`), `ToUint32` of it was NaN where ECMA-262 9.6
// says 0, `parseFloat("Infinity")` was the largest finite double, and the
// largest finite double was written as `Infinity` and as `null` by
// `JSON.stringify`. The shared ECMA-262 table asks none of it, so nothing said
// so.

package scelua

import "testing"

func evaluated(t *testing.T, e *LuaEngine, expression string) interface{} {
	t.Helper()
	got, err := e.EvaluateExpression("s", expression)
	if err != nil {
		t.Fatalf("%s: %v", expression, err)
	}
	return got
}

func TestMathHugeIsTheInfinityLuaDefines(t *testing.T) {
	e := NewLuaEngine()
	if err := e.CreateSession("s"); err != nil {
		t.Fatalf("CreateSession: %v", err)
	}
	if got := evaluated(t, e, "math.huge == 1/0"); got != true {
		t.Errorf("math.huge is not the infinity 1/0 is: %v", got)
	}
	if got := evaluated(t, e, "math.huge > 1.7976931348623157e308"); got != true {
		t.Errorf("math.huge is not past the largest finite double: %v", got)
	}
}

func TestAnInfinityIsSpelledAsEcmascriptSpellsIt(t *testing.T) {
	e := NewLuaEngine()
	if err := e.CreateSession("s"); err != nil {
		t.Fatalf("CreateSession: %v", err)
	}
	for expression, want := range map[string]string{
		"_scxml_tostring(1/0)":                    "Infinity",
		"_scxml_tostring(-1/0)":                   "-Infinity",
		"_scxml_tostring(parseFloat('Infinity'))": "Infinity",
		"JSON.stringify(1.7976931348623157e308)":  "1.7976931348623157e+308",
		"JSON.stringify(1/0)":                     "null",
	} {
		if got := evaluated(t, e, expression); got != want {
			t.Errorf("%s is %v, want %q", expression, got, want)
		}
	}
}

func TestToUint32OfAnInfinityIsZero(t *testing.T) {
	e := NewLuaEngine()
	if err := e.CreateSession("s"); err != nil {
		t.Fatalf("CreateSession: %v", err)
	}
	for _, expression := range []string{"_scxml_touint32(1/0)", "_scxml_touint32(-1/0)"} {
		// An integral value is handed over as an int64 (`luaToGo`).
		if got := evaluated(t, e, expression); got != int64(0) {
			t.Errorf("%s is %v (%T), want 0 (ECMA-262 9.6: an infinity is 0)", expression, got, got)
		}
	}
}
