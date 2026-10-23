// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A C++ test unit compiles a bounded number of generated machines.
//!
//! A generated C++ machine is a header of templates the compiler instantiates in
//! every unit that includes it, so what a unit costs to compile grows with the
//! machines it includes. Measured 2026-10-23 on a build machine, RelWithDebInfo,
//! each unit of `static_datamodel_generated_cpp_test` compiled alone: the unit that
//! replayed every scenario included 75 machines and took 20.57 GB and 556 s; the
//! next largest included 11 and took 2.18 GB; the rest 0.8 to 1.4 GB. The hosted
//! runner of the C++ lane has 16 GB for four parallel jobs, and the lane died with
//! "the runner has received a shutdown signal" on every push from c31a261a72 on,
//! when the saved state grew each machine and pushed that one unit past it.
//!
//! The scenarios are now replayed one machine to a unit, made by the build from the
//! scenario directory (`tests/integration/static_scenarios/`), so that suite cannot
//! grow a unit again. What this holds is every unit written by hand: none includes
//! more than [`MOST_MACHINES_PER_UNIT`] generated machines. The two largest units
//! measured are about 0.27 GB a machine, which puts twelve near 3 GB and four such
//! units inside the runner. A test that needs more machines than that is two units.
//!
//! The count is of the generated headers a unit includes by name
//! (`#include "<machine>_sm.h"`), the one spelling a generated machine is reached
//! by in a test. A child a machine starts is generated into its parent's header,
//! so a unit that includes a parent pays for its children too; the bound is on
//! what the unit names, which is what an author adds to.

mod common;

/// The most generated machines one hand-written C++ test unit may include.
const MOST_MACHINES_PER_UNIT: usize = 12;

/// The fewest C++ test units that include a generated machine the sweep must find.
/// A sweep that stopped finding them would report every unit within the bound.
/// Measured 2026-10-23: 181, at roughly 90 % of which this sits.
const UNITS_FLOOR: usize = 160;

/// The generated machines `source` includes by name, in the order it includes them.
fn machines_included(source: &str) -> Vec<&str> {
    source
        .lines()
        .filter_map(|line| line.trim().strip_prefix("#include \""))
        .filter_map(|rest| rest.strip_suffix("_sm.h\""))
        .filter(|stem| {
            !stem.is_empty() && stem.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        })
        .collect()
}

#[test]
fn no_cpp_test_unit_compiles_more_than_a_bounded_number_of_machines() {
    let mut units = 0usize;
    let mut over = Vec::new();
    for path in common::repository::files_git_tracks(&["*.cpp"]) {
        let source = std::fs::read_to_string(&path).expect("a C++ unit reads as text");
        let machines = machines_included(&source);
        if machines.is_empty() {
            continue;
        }
        units += 1;
        if machines.len() > MOST_MACHINES_PER_UNIT {
            over.push(format!(
                "  {}: {} machines",
                path.strip_prefix(common::repository::root())
                    .unwrap_or(&path)
                    .display(),
                machines.len()
            ));
        }
    }
    assert!(
        units >= UNITS_FLOOR,
        "only {units} C++ unit(s) include a generated machine, floor {UNITS_FLOOR}: either the sweep \
         stopped finding them, or the tree shed them and the floor should be re-measured and lowered \
         in the same commit"
    );
    assert!(
        over.is_empty(),
        "a C++ test unit includes more than {MOST_MACHINES_PER_UNIT} generated machines:\n{}\n\n\
         Each is a header of templates compiled again in the unit, and a unit of 75 took 20.57 GB to \
         compile, past the C++ lane's runner. Split the unit by what its tests are about; a scenario \
         belongs in sce-build/tests/fixtures/static_datamodel/scenarios, which the build replays one \
         machine to a unit.",
        over.join("\n")
    );
}

/// The count is exercised on sources written here, in the direction the tree cannot
/// show: a scan of a tree within the bound can only come back green.
#[test]
fn the_count_reads_the_includes_of_generated_machines_and_nothing_else() {
    let source = "#include \"static_counter_sm.h\"\n\
                  #include \"integration/static_scenarios/StaticScenarioReplay.h\"\n\
                  #include <gtest/gtest.h>\n\
                  // #include \"in_a_comment_sm.h\" is not an include\n\
                  #include \"record_list_sm.h\"\n\
                  #include \"@SCE_SCENARIO_HEADER@\"\n";
    assert_eq!(
        machines_included(source),
        vec!["static_counter", "record_list"],
        "a generated machine's header is counted, and no other include"
    );
    let crowded: String = (0..=MOST_MACHINES_PER_UNIT)
        .map(|n| format!("#include \"machine{n}_sm.h\"\n"))
        .collect();
    assert!(
        machines_included(&crowded).len() > MOST_MACHINES_PER_UNIT,
        "a unit one machine past the bound is counted past it"
    );
}
