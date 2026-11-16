// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// Command job_suite_runner is the runner program of the generated suite
// `job_suite` of the corpus (docs/SCE_TTCN3_LANE.md §4, §12.1): every test case
// a world selects, run against its recorded input trace, its canonical test log
// written beside the others. The program is the suite unit's Main, which
// sce-codegen writes from sce-build/tests/fixtures/test_profile/job_suite.scxml;
// this file only makes it a program, as cmd/runtime_suite_runner does for
// runtime_suite. The Rust twin is backends/rust/tests/src/bin/job_suite_runner.rs.
//
//	go run ./cmd/job_suite_runner --trace-dir <dir> --log-dir <dir> [--world <feature>,...] [--module-parameter <id>=<value>]...
package main

import "github.com/newmassrael/sce-go-tests/integration/static_datamodel/job_suite"

func main() {
	job_suite.Main()
}
