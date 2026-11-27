// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Each scenario of the properties judge's turnstile is the run of every lane
//! that replays it (docs/adr/0115, D15 and the converse of D8). A scenario is
//! the run TLC gave of a violation or a witness
//! (`sce-codegen formal properties --scenarios`); each lane's harness gives the
//! machine the scenario's inputs and records the run its trace port is told
//! beside it, `<name>.<lane>.json`: the Rust lane's
//! `backends/rust/tests/tests/a_scenario_of_the_turnstile_is_its_run.rs` and
//! the C11 lane's `backends/c/tests/integration/test_a_scenario_of_the_turnstile_is_its_run.c`.
//! Here every recorded run is held to be its scenario's, step for step, and
//! every scenario to have a run of each lane, so a scenario added without its
//! replays, or a replay that parts from the model, is seen without TLC.

use std::path::{Path, PathBuf};

use sce_build::formal::replay::judge_dir;

/// The lanes whose harness replays a scenario.
const LANES: [&str; 2] = ["rust", "c11"];

fn scenarios_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/formal_properties/scenarios")
}

#[test]
fn every_scenario_of_the_turnstile_is_the_run_of_every_lane() {
    let problems = judge_dir(&scenarios_dir(), &LANES);
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}
