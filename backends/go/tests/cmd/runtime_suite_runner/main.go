// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// Command runtime_suite_runner is the runner program of the generated suite
// `runtime_suite` (docs/SCE_TTCN3_LANE.md §4): every test case a world selects,
// run against its recorded input trace, its canonical test log written beside
// the others. The program is the suite unit's Main, which sce-codegen writes
// from sce-build/tests/fixtures/test_profile/runtime/runtime_suite.scxml; this
// file only makes it a program, as a module that holds a generated suite does.
// It stands under cmd/, apart from the generated packages, which the
// regeneration clears and writes again. The Rust twin is
// backends/rust/tests/src/bin/runtime_suite_runner.rs, the C++ one
// tests/integration/runtime_suite_runner.cpp.
//
//	go run ./cmd/runtime_suite_runner --trace-dir <dir> --log-dir <dir> [--world <feature>,...] [--module-parameter <id>=<value>]...
package main

import "github.com/newmassrael/sce-go-tests/integration/static_datamodel/runtime_suite"

func main() {
	runtime_suite.Main()
}
