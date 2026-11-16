// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The runner program of the generated suite `boundary_suite` of the corpus
// (docs/SCE_TTCN3_LANE.md §4, §12.1): every test case a world selects, run
// against its recorded input trace, its canonical test log written beside the
// others. The program is the suite's header's `main`, which sce-codegen writes
// from sce-build/tests/fixtures/test_profile/boundary/boundary_suite.scxml; this
// file only makes it a program, as runtime_suite_runner.cpp does for
// runtime_suite. The Rust twin is
// backends/rust/tests/src/bin/boundary_suite_runner.rs.
//
//   boundary_suite_runner --trace-dir <dir> --log-dir <dir>
//       [--world <feature>,...] [--module-parameter <id>=<value>]...

#include "boundary_suite.h"

int main(int argc, char **argv) {
    return SCE::Generated::BoundarySuite::main(argc, argv);
}
