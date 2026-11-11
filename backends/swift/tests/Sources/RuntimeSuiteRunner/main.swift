// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The runner program of the generated suite `runtime_suite` (docs/SCE_TTCN3_LANE.md §4): every test case
// a world selects, run against its recorded input trace, its canonical test log written beside the
// others. The program is the suite's unit's entry, `RuntimeSuite.main()`, which sce-codegen writes from
// sce-build/tests/fixtures/test_profile/runtime/runtime_suite.scxml into `StaticDatamodelMachines`; this
// target only makes it a program, as a package that holds a generated suite does. The Rust twin is
// backends/rust/tests/src/bin/runtime_suite_runner.rs, the C++ one tests/integration/runtime_suite_runner.cpp.
//
//   RuntimeSuiteRunner --trace-dir <dir> --log-dir <dir> [--world <feature>,...] [--module-parameter <id>=<value>]...

import StaticDatamodelMachines

RuntimeSuite.main()
