// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML B.2 / 5.10: a JSON event payload reaches `_event.data` as a table.
//
// 1:1 mirror of backends/rust/lua/tests/json_event_data_is_utf8.rs. The path
// only runs when the payload is NOT valid Lua — a consumer sending Lua table
// syntax succeeds one step earlier — so every in-tree fixture that exercises
// it carries plain ASCII objects, and JSON's own grammar had no reader here.

package scelua

import (
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"
)

func setEvent(t *testing.T, e *LuaEngine, session, data string) {
	t.Helper()
	if _, err := e.SetCurrentEvent(session, sce.SetCurrentEventArgs{
		Name: "brief",
		Data: data,
		Type: "external",
	}); err != nil {
		t.Fatalf("SetCurrentEvent: %v", err)
	}
}

func heldString(t *testing.T, e *LuaEngine, script string) string {
	t.Helper()
	if err := e.ExecuteScript("s", script); err != nil {
		t.Fatalf("ExecuteScript(%q): %v", script, err)
	}
	v, err := e.GetVariable("s", "held")
	if err != nil {
		t.Fatalf("GetVariable: %v", err)
	}
	s, ok := v.(string)
	if !ok {
		t.Fatalf("held is %T (%v), want string — `_event.data` is not a table", v, v)
	}
	return s
}

// Non-ASCII prose in a payload. Go writes bytes rather than widening them, so
// this is the case Go already answered correctly; it is here so the file states
// the whole contract rather than only its gaps.
func TestJSONEventDataCarriesNonASCIIWhole(t *testing.T) {
	e := NewLuaEngine()
	if err := e.CreateSession("s"); err != nil {
		t.Fatalf("CreateSession: %v", err)
	}
	const want = "북극성 — ship it"
	setEvent(t, e, "s", `{"north_star": "`+want+`"}`)
	if got := heldString(t, e, "held = _event.data.north_star"); got != want {
		t.Errorf("north_star: got %q want %q", got, want)
	}
}

// JSON's `\uXXXX`. Lua 5.4 spells the same thing `\u{XXXX}`, so generating Lua
// source from the payload makes valid JSON fail to compile — and the failure is
// not reported, it falls through to the string branch, leaving `_event.data` a
// string whose fields all read nil.
func TestJSONEventDataUnicodeEscapeYieldsATable(t *testing.T) {
	e := NewLuaEngine()
	if err := e.CreateSession("s"); err != nil {
		t.Fatalf("CreateSession: %v", err)
	}
	// Built from the backslash so this source carries no escape of its own:
	// the payload is the literal text a JSON encoder emits for "A북" when
	// asked to stay ASCII-only.
	bs := "\\"
	setEvent(t, e, "s", `{"north_star": "`+bs+`u0041`+bs+`ubd81"}`)
	if got := heldString(t, e, "held = _event.data.north_star"); got != "A북" {
		t.Errorf("north_star: got %q want %q", got, "A북")
	}
}

// `\/` is a legal JSON escape and not a Lua escape at all.
func TestJSONEventDataEscapedSolidusYieldsATable(t *testing.T) {
	e := NewLuaEngine()
	if err := e.CreateSession("s"); err != nil {
		t.Fatalf("CreateSession: %v", err)
	}
	bs := "\\"
	setEvent(t, e, "s", `{"path": "a`+bs+`/b"}`)
	if got := heldString(t, e, "held = _event.data.path"); got != "a/b" {
		t.Errorf("path: got %q want %q", got, "a/b")
	}
}

// A JSON array. Lua writes every table with braces, so `[` and `]` carried
// across verbatim are Lua's *index* syntax, which does not parse in value
// position.
func TestJSONEventDataArrayIsIndexable(t *testing.T) {
	e := NewLuaEngine()
	if err := e.CreateSession("s"); err != nil {
		t.Fatalf("CreateSession: %v", err)
	}
	setEvent(t, e, "s", `{"codes": [200, 201]}`)
	if err := e.ExecuteScript("s", "held = tostring(_event.data.codes[1]) .. ',' .. tostring(_event.data.codes[2])"); err != nil {
		t.Fatalf("ExecuteScript: %v", err)
	}
	v, err := e.GetVariable("s", "held")
	if err != nil {
		t.Fatalf("GetVariable: %v", err)
	}
	if got, want := v.(string), "200,201"; got != want {
		t.Errorf("codes: got %q want %q", got, want)
	}
}

// A top-level JSON array, which is the whole payload rather than a field of one.
func TestJSONEventDataTopLevelArrayIsATable(t *testing.T) {
	e := NewLuaEngine()
	if err := e.CreateSession("s"); err != nil {
		t.Fatalf("CreateSession: %v", err)
	}
	setEvent(t, e, "s", `["a", "b"]`)
	if got := heldString(t, e, "held = _event.data[1]"); got != "a" {
		t.Errorf("first element: got %q want %q", got, "a")
	}
}
