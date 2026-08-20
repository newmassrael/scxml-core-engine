// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The engine that spells a literal is the engine that has to read it back.
//
// W3C SCXML 6.4.1 has the parent evaluate an <invoke>'s params in its own data
// model and hand the values to the child, and the child seeds them by
// evaluating source. So the literal is fed straight to an interpreter, and the
// only interpreter that can be relied on to accept it is the one that wrote
// it. That is why ToScriptLiteral is a method on sce.IScriptEngine and not a
// free function in the runtime package, where it knew Lua's grammar while
// belonging to no engine.
//
// 1:1 mirror of backends/rust/lua/tests/engine_owns_its_literal.rs. The round
// trip is what a table of expected strings cannot claim: it does not assert
// that the text LOOKS like Lua, it asserts that this engine parses it back
// into the value it started from.

package scelua

import (
	"reflect"
	"testing"
)

func roundTrip(t *testing.T, e *LuaEngine, session string, value interface{}) interface{} {
	t.Helper()
	literal := e.ToScriptLiteral(value)
	got, err := e.EvaluateExpression(session, literal)
	if err != nil {
		t.Fatalf("this engine could not read its own literal %q: %v", literal, err)
	}
	return got
}

func TestEveryValueSurvivesItsOwnEnginesLiteral(t *testing.T) {
	engine := NewLuaEngine()
	if err := engine.Initialize(); err != nil {
		t.Fatalf("Initialize: %v", err)
	}
	defer engine.Shutdown()
	const session = "literal_round_trip"
	if err := engine.CreateSession(session); err != nil {
		t.Fatalf("CreateSession: %v", err)
	}
	defer engine.DestroySession(session)

	for _, value := range []interface{}{
		true,
		false,
		int64(42),
		int64(-7),
		2.5,
		"plain",
		// The escapes exist for these; a literal that lost one would come
		// back as a different string or fail to load at all.
		"has \"quotes\" and \\ and\na newline",
	} {
		if got := roundTrip(t, engine, session, value); !reflect.DeepEqual(got, value) {
			t.Errorf("round trip of %#v produced %#v", value, got)
		}
	}

	// Absence: Lua has one word for it, and the round trip lands back on nil.
	if got := roundTrip(t, engine, session, nil); got != nil {
		t.Errorf("round trip of nil produced %#v", got)
	}
}

func TestAStructuredValueSurvivesAsATable(t *testing.T) {
	engine := NewLuaEngine()
	if err := engine.Initialize(); err != nil {
		t.Fatalf("Initialize: %v", err)
	}
	defer engine.Shutdown()
	const session = "structured_round_trip"
	if err := engine.CreateSession(session); err != nil {
		t.Fatalf("CreateSession: %v", err)
	}
	defer engine.DestroySession(session)

	array := []interface{}{int64(1), "two", true}
	if got, want := engine.ToScriptLiteral(array), `{1, "two", true}`; got != want {
		t.Fatalf("array literal = %q, want %q (brackets would be a syntax error here)", got, want)
	}
	if got := roundTrip(t, engine, session, array); !reflect.DeepEqual(got, array) {
		t.Errorf("round trip of %#v produced %#v", array, got)
	}

	object := map[string]interface{}{"k": int64(1), "j": "v"}
	if got, want := engine.ToScriptLiteral(object), `{["j"] = "v", ["k"] = 1}`; got != want {
		t.Fatalf("object literal = %q, want %q (keys sorted so equal content is equal text)", got, want)
	}
	if got := roundTrip(t, engine, session, object); !reflect.DeepEqual(got, object) {
		t.Errorf("round trip of %#v produced %#v", object, got)
	}
}

// The Lua column the Rust sibling quotes is this engine's too — the two Lua
// backends answer the source question identically, which is what makes the
// spelling a property of the LANGUAGE rather than of one implementation.
func TestTheLuaColumnIsTheSameInBothLuaBackends(t *testing.T) {
	engine := NewLuaEngine()
	if err := engine.Initialize(); err != nil {
		t.Fatalf("Initialize: %v", err)
	}
	defer engine.Shutdown()

	for _, row := range []struct {
		value interface{}
		want  string
	}{
		{nil, "nil"},
		{[]interface{}{int64(1), int64(2)}, "{1, 2}"},
		{map[string]interface{}{"k": int64(1)}, `{["k"] = 1}`},
		{5.0, "5.0"},
	} {
		if got := engine.ToScriptLiteral(row.value); got != row.want {
			t.Errorf("ToScriptLiteral(%#v) = %q, want %q", row.value, got, row.want)
		}
	}
}
