// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.3: a typed `<data>` whose id is a keyword somewhere, or no
// identifier in some backend, still gets a reader where every backend can
// spell one, and it reads its own variable. Go AOT.
//
// Go's PascalCase readers never meet a Go keyword, so what this channel shows
// is the other half: the ids no backend can give a reader (`auto`, `self`,
// `new`, `start`, `t`, and `a-b` beside `a_b`) are absent here too, decided
// once by `sce-build/src/reader_names.rs` rather than per backend, and
// `screen-rules` reads as `ScreenRules`.
//
// Fixture: integration_resources/typed_reader_names/typed_reader_names.scxml
// (canonical, shared with the C++ / C11 / Kotlin / Python / Rust channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_typed_reader_names_go.sh

package typed_reader_names

import (
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
)

func started(t *testing.T) (*sce.Engine[TypedReaderNamesState, TypedReaderNamesEvent], *TypedReaderNamesPolicy) {
	t.Helper()
	policy := NewTypedReaderNamesPolicy()
	policy.SessionID = sce.GenerateSessionID()
	policy.ScriptEngine = scegotest.NewLuaEngine()
	engine := sce.NewEngine[TypedReaderNamesState, TypedReaderNamesEvent](&policy)
	engine.Initialize()
	return engine, &policy
}

func expect(t *testing.T, name string, read func() (int64, bool), want int64) {
	t.Helper()
	if got, ok := read(); !ok || got != want {
		t.Fatalf("%s() = %d (ok=%v), want %d", name, got, ok, want)
	}
}

// Each reader reads the value its own variable was declared with.
func TestAReaderReadsItsOwnVariable(t *testing.T) {
	_, p := started(t)
	expect(t, "Box", p.Box, 1)
	expect(t, "Object", p.Object, 2)
	expect(t, "Pass", p.Pass, 3)
	expect(t, "ScreenRules", p.ScreenRules, 4)
	expect(t, "AB", p.AB, 10)
}

// And the value it holds now: `bump` adds 10 to four of them, and leaves
// `screen-rules` — which no ECMAScript expression can name — alone.
func TestAReaderReadsTheLiveValue(t *testing.T) {
	engine, p := started(t)
	engine.RaiseExternal(TypedReaderNamesEventBump, "", "")
	engine.Step()
	expect(t, "Box", p.Box, 11)
	expect(t, "Object", p.Object, 12)
	expect(t, "Pass", p.Pass, 13)
	expect(t, "ScreenRules", p.ScreenRules, 4)
	expect(t, "AB", p.AB, 20)
}
