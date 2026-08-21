// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A value leaves the data model two ways, and only one of them is a language.
//
// Until 2026-08-21 this package exported ToLuaLiteral — a free function, on no
// engine — and generated code used it for both directions: seeding an
// <invoke> child's datamodel, where the text IS source that some engine has to
// parse back, and a §scxml-C-2 HTTP <param>, where the text leaves the process
// for a reader that is not an engine at all.
//
// The second reading is the one with a visible cost. Measured on 2026-08-21,
// one form-encoded param had six spellings across six backends: this one sent
// `nil` for an absent value, C++ sent nothing, Python sent `None`, C11 sent
// the word `nil` too and, for a table, a POINTER. The peer at the other end of
// the socket cannot be expected to know which backend compiled the sender.
//
// ToWireString is the neutral rendering, and its rows below are C++
// ScriptResultUtils::resultToString arm for arm. The source direction is gone
// rather than relocated: an <invoke> <param> hands the child the value, and
// rendering it as source lost every value Lua cannot spell — `1/0` reached the
// child as the text `+Inf`, which is not a Lua expression, so the pair arrived
// as nothing. The clause that says so is held by
// integration_resources/invoke_param_seeds_declared_child_data/ on all seven
// channels; this file is only about what leaves the process.

package sce

import "testing"

func TestToWireStringRendersTheValueNotTheSendersLanguage(t *testing.T) {
	rows := []struct {
		name  string
		value interface{}
		want  string
	}{
		// The comment on each changed row is what used to reach the peer.
		{"absence is empty, not a word", nil, ""}, // was "nil"
		{"true", true, "true"},
		{"false", false, "false"},
		{"int", 42, "42"},
		{"int64", int64(-7), "-7"},
		{"integral float has no tail", 5.0, "5"}, // was "5.0"
		{"fractional float keeps its point", 2.5, "2.5"},
		{"a string is already text", "plain", "plain"},
		// The quotes belong to the value. The old path added its own and
		// then trimmed them off, which ate these.
		{"quotes in the value survive", `"quoted"`, `"quoted"`},
		{
			"array is JSON",
			[]interface{}{1, 2},
			"[1,2]",
		}, // was "{1, 2}"
		{
			"object is JSON",
			map[string]interface{}{"k": "v"},
			`{"k":"v"}`,
		}, // was `{["k"] = "v"}`
	}

	for _, row := range rows {
		if got := ToWireString(row.value); got != row.want {
			t.Errorf("%s: ToWireString(%#v) = %q, want %q", row.name, row.value, got, row.want)
		}
	}
}

// A structured value on the wire is the same bytes the payload direction
// already sends, because both are read by a parser and neither by an engine.
func TestToWireStringDelegatesStructuredValuesToTheJSONEncoder(t *testing.T) {
	value := map[string]interface{}{"b": 2, "a": []interface{}{1, "x"}}
	if got, want := ToWireString(value), ScriptValueToJSON(value); got != want {
		t.Errorf("ToWireString(%#v) = %q, want the JSON encoder's %q", value, got, want)
	}
}
