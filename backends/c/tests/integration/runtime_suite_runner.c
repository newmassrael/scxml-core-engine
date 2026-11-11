// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The runner program of the generated suite `runtime_suite`
// (docs/SCE_TTCN3_LANE.md §4): every test case a world selects, run against its
// recorded input trace, its canonical test log written beside the others. The
// program is the suite's header's `runtime_suite_main`, which sce-codegen writes
// from sce-build/tests/fixtures/test_profile/runtime/runtime_suite.scxml; this
// file only makes it a program, as a build that holds a generated suite does.
// The C++ twin is tests/integration/runtime_suite_runner.cpp.
//
//   sce_c_runtime_suite_runner --trace-dir <dir> --log-dir <dir>
//       [--world <feature>,...] [--module-parameter <id>=<value>]...

#include "runtime_suite.h"

int main(int argc, char **argv) {
    return runtime_suite_main(argc, argv);
}
