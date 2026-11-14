// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The runner program of the generated suite `boundary_suite`
//! (docs/SCE_TTCN3_LANE.md §4, §12): every test case a world selects, run
//! against its recorded input trace, its canonical test log written beside the
//! others. The program is the suite's unit's `main`, which sce-codegen writes
//! from `sce-build/tests/fixtures/test_profile/boundary/boundary_suite.scxml`;
//! this target only makes it a program.
//!
//! ```text
//! boundary_suite_runner --trace-dir <dir> --log-dir <dir> [--world <feature>,...] [--module-parameter <id>=<value>]...
//! ```

fn main() -> std::process::ExitCode {
    sce_rust_tests::integration::static_datamodel::boundary_suite::main()
}
