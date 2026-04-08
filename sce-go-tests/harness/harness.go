// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

// Package harness provides the W3C SCXML conformance test harness for Go.
//
// Ports the Rust SimpleAotTest trait and run_simple_aot_test() from
// sce-rust-tests/src/harness.rs. Each generated test calls RunTest()
// or uses AssertFinalState() to verify W3C conformance.
package harness

import (
	"testing"

	scelua "github.com/newmassrael/sce-go-lua"
)

// RegisterLuaEngine registers the Lua script engine for tests requiring it.
func RegisterLuaEngine() {
	scelua.Register()
}

// AssertFinalState checks that the engine reached the expected final state.
func AssertFinalState[S comparable](t *testing.T, actual, expected S, testID string) {
	t.Helper()
	if actual != expected {
		t.Fatalf("Test %s reached wrong final state: got %v, want %v", testID, actual, expected)
	}
}
