// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// Package testprofile is the runtime of the test profile
// (docs/SCE_TEST_PROFILE.md §12.2): the round scheduler, `NOW`, port queues and
// snapshots, timer bindings, verdicts, the entity table and the guard, written
// once for every suite.
//
// A suite's generated units reach it from here: a test component's machine
// through its ComponentContext, which it keeps from ComponentMachine.Start on;
// a configuration as a Configuration and a MachineFactory; a test case and a
// suite as a TestCase and a Suite, which RunSuite runs. A template's length
// restriction on a text counts it with TextLength. A run's test log (§12.5) is
// its TestCaseResult.Records, written in its one canonical text by
// TestCaseResult.CanonicalLog; a message's value reaches it in the canonical
// form package canonical spells, which the generated unit of a template over a
// record writes (`<template>.Canonical`). The algorithm is the one the
// document specifies and the C++ runtime (`sce/include/test/`), the Rust one
// (`sce_rust_runtime::test`) and the Kotlin one (`com.sce.runtime.test`)
// implement, type for type. When this package and the Rust one diverge, Rust
// wins, as it does for the rest of the Go runtime.
//
// # The name
//
// The other runtimes call the package `test`. Go gives that word to its own
// tooling: `go test` runs the files that end in `_test.go`, and a package
// whose name ends in `_test` is the external test package of the directory it
// stands in. A package named `test` or `sce_test` would read, at every import
// and every qualified name, as one of those; `testprofile` names the document
// the package implements and nothing of the toolchain.
//
// # Ownership
//
// The state of a run is shared between the run and a context per component, as
// the C++ runtime's context holds a reference to its run: the machine keeps its
// context from Start on, and so may the host processors it registers on its
// engine for the doors of a send and of a PTC. A run is driven on one goroutine
// (§12.4), the one that calls TestCaseRun.Run, so the state takes no lock,
// where the Rust runtime's takes a mutex because its engine holds those
// processors as `Send` closures.
//
// The run never holds its state across a call into a machine: every step of it
// marks the state held for its own length, and a machine is called between
// steps. So every call a machine makes into its context finds the state free. A
// call that finds it held is a call made while the run, or the same component,
// is already inside the state, which on one goroutine can only be a re-entrant
// call (a snapshot reader that calls its context, say), and it panics naming
// the component, as the Rust runtime does. A panic that leaves a step part way
// goes out of the run, as it does in Rust.
package testprofile
