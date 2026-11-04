// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Both CMake routes of the W3C conformance suite derive their registrations
//! from the conformance registry rather than spelling them.
//!
//! The C++ registrations were 202 hand-written
//! `sce_generate_static_w3c_test(144 ...)` lines, and `generate-w3c`
//! read them back out with a regex — CMake was the registry, and a
//! checkout without CMake could not enumerate the suite. The C11 route kept
//! 202 more, `sce_generate_static_w3c_c_test(144 ...)`, in an order of its
//! own and each flagged `NEEDS_LUA`, `NEEDS_DOM` or `NEEDS_HTTP` by hand,
//! until 2026-11. Now `tests/w3c/conformance/fixtures.json` is the source,
//! and both routes ask `list-fixtures` for the ids through the one reading of
//! it (`sce_w3c_registry_fixtures`, `cmake/SCEW3CRegistry.cmake`), the way
//! every forge conformance harness already consumes its own catalog.
//!
//! What this pins is that the derivation is real. A literal id creeping
//! back into a registration is the failure that matters: it would build
//! and run, so nothing else would notice, and the set would once again
//! be stated in two places that can disagree. The configure-time
//! dependency is pinned for the same reason — without it, editing the
//! registry leaves a configured tree building the previous set, and the
//! stale result looks like a correct one. A flag on a C11 registration is
//! the other copy that went: what a machine links is the generator's
//! statement (`a_c11_machine_includes_what_it_states_it_requires`).

use std::path::{Path, PathBuf};

use sce_build::w3c_registry::{W3cRegistry, W3C_REGISTRY_RELATIVE_PATH};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

fn read(relative: &str) -> String {
    std::fs::read_to_string(repo_root().join(relative))
        .unwrap_or_else(|e| panic!("read {relative}: {e}"))
}

/// The module both routes read the registry through.
const REGISTRY_MODULE: &str = "cmake/SCEW3CRegistry.cmake";

/// One route: the file that registers the fixtures and the function it
/// registers each with.
struct Route {
    file: &'static str,
    registration: &'static str,
}

const ROUTES: [Route; 2] = [
    Route {
        file: "tests/CMakeLists.txt",
        registration: "sce_generate_static_w3c_test",
    },
    Route {
        file: "backends/c/tests/CMakeLists.txt",
        registration: "sce_generate_static_w3c_c_test",
    },
];

/// The file without its comment lines: the files document the call shape
/// in prose, and a documented example is not a registration.
fn code_lines(cmake: &str) -> Vec<&str> {
    cmake
        .lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .collect()
}

/// Registration calls of `function`, each by the argument naming the test.
fn registration_arguments(cmake: &str, function: &str) -> Vec<String> {
    let re =
        regex::Regex::new(&format!(r"\b{}\(([^\s)]+)", regex::escape(function))).expect("pattern");
    code_lines(cmake)
        .into_iter()
        .filter_map(|l| re.captures(l))
        .map(|c| c[1].to_string())
        .collect()
}

/// Every way `cmake` registers a fixture by anything but an id the
/// registry's reading produced: a registration naming a literal id, a
/// registration naming a variable that is not the loop variable of a
/// `foreach` over the ids `sce_w3c_registry_fixtures` set, and a route that
/// never reads the registry.
fn registration_violations(cmake: &str, function: &str) -> Vec<String> {
    let mut found = Vec::new();
    let args = registration_arguments(cmake, function);
    if args.is_empty() {
        found.push(format!("no `{function}` call at all"));
        return found;
    }
    let reading =
        regex::Regex::new(r"\bsce_w3c_registry_fixtures\(\s*([A-Za-z0-9_]+)\s+").expect("pattern");
    let code = code_lines(cmake).join("\n");
    let Some(ids) = reading.captures(&code).map(|c| c[1].to_string()) else {
        found.push(
            "the route never calls `sce_w3c_registry_fixtures`, so its ids are not the \
             registry's"
                .to_string(),
        );
        return found;
    };
    let loop_re = regex::Regex::new(&format!(
        r"foreach\(\s*([A-Za-z0-9_]+)\s+\$\{{{}\}}\s*\)",
        regex::escape(&ids)
    ))
    .expect("pattern");
    let loop_variables: Vec<String> = loop_re
        .captures_iter(&code)
        .map(|c| c[1].to_string())
        .collect();
    for arg in &args {
        let derived = loop_variables
            .iter()
            .any(|variable| *arg == format!("${{{variable}}}"));
        if !derived {
            found.push(format!(
                "`{function}({arg} ...)` registers an id that is not one \
                 `sce_w3c_registry_fixtures` read (it set `{ids}`, iterated as \
                 {loop_variables:?})"
            ));
        }
    }
    found
}

#[test]
fn every_w3c_registration_takes_its_id_from_the_registry() {
    for route in &ROUTES {
        let found = registration_violations(&read(route.file), route.registration);
        assert!(
            found.is_empty(),
            "{} registers W3C fixtures the registry at {W3C_REGISTRY_RELATIVE_PATH} did not \
             name: {found:?}. The registry is the source of truth — an id written here \
             restates the set in a second place, and the two can then disagree without \
             anything failing.",
            route.file,
        );
    }
}

/// The hand-written shape this replaced, and a registration from a list of
/// the route's own, are refused; the derived one is not.
#[test]
fn a_registration_by_a_literal_id_is_refused() {
    let derived = "sce_w3c_registry_fixtures(_IDS _HARNESS_OF_)\n\
                   foreach(_ID ${_IDS})\n\
                   \x20   sce_generate_static_w3c_c_test(${_ID} \"${DIR}\")\n\
                   endforeach()\n";
    assert_eq!(
        registration_violations(derived, "sce_generate_static_w3c_c_test"),
        Vec::<String>::new()
    );

    let literal =
        format!("{derived}sce_generate_static_w3c_c_test(355 \"${{DIR}}\")  # W3C SCXML 3.6\n");
    let found = registration_violations(&literal, "sce_generate_static_w3c_c_test");
    assert!(
        found.iter().any(|v| v.contains("(355 ...)")),
        "a literal registration was accepted: {found:?}"
    );

    let own_list = "set(_MINE 144 147)\n\
                    foreach(_ID ${_MINE})\n\
                    \x20   sce_generate_static_w3c_c_test(${_ID} \"${DIR}\")\n\
                    endforeach()\n";
    let found = registration_violations(own_list, "sce_generate_static_w3c_c_test");
    assert!(
        found
            .iter()
            .any(|v| v.contains("never calls `sce_w3c_registry_fixtures`")),
        "a registration from the route's own list was accepted: {found:?}"
    );

    let commented = "# sce_generate_static_w3c_c_test(355 \"${DIR}\")\n";
    assert!(
        registration_arguments(commented, "sce_generate_static_w3c_c_test").is_empty(),
        "a documented call in a comment read as a registration"
    );
}

#[test]
fn the_registry_is_read_through_list_fixtures_in_one_module() {
    let module = read(REGISTRY_MODULE);
    assert!(
        module.contains(W3C_REGISTRY_RELATIVE_PATH),
        "{REGISTRY_MODULE} never names {W3C_REGISTRY_RELATIVE_PATH}, so the routes' \
         registrations cannot be coming from the registry",
    );
    assert!(
        module.contains("list-fixtures") && module.contains("--catalog w3c"),
        "{REGISTRY_MODULE} does not enumerate the registry through \
         `list-fixtures --catalog w3c`; a hand-rolled JSON read in CMake would be a \
         second parser of the same file",
    );
    assert!(
        module.contains("CMAKE_CONFIGURE_DEPENDS"),
        "{REGISTRY_MODULE} does not make the configure depend on the registry, so editing \
         it would leave a configured tree building the previous set of tests",
    );
    // The module is the one reading: no route runs `list-fixtures` itself.
    for route in &ROUTES {
        let code = code_lines(&read(route.file)).join("\n");
        assert!(
            !code.contains("--catalog w3c"),
            "{} enumerates the registry itself rather than through \
             `sce_w3c_registry_fixtures`: a second reading of one file",
            route.file,
        );
    }
}

/// The harnesses `sce_w3c_registry_fixtures` iterates.
fn iterated_harnesses(module: &str) -> Vec<String> {
    // Read the loop's own list rather than searching the file: `http` and
    // `simple` both occur in unrelated prose, so a substring match would pass
    // whatever the loop iterates.
    let re = regex::Regex::new(r"set\(_W3C_HARNESSES\s+([^)]*)\)").expect("pattern");
    let caps = re
        .captures(module)
        .expect("cmake/SCEW3CRegistry.cmake must declare _W3C_HARNESSES for the loop to iterate");
    caps[1].split_whitespace().map(str::to_string).collect()
}

/// Every harness the registry declares is one the registry reading
/// iterates and each route runs.
///
/// The reading maps every fixture to the harness a `list-fixtures --harness`
/// query named it under, and refuses a fixture no query named; each route
/// then says what its runner needs for each harness, and refuses one it does
/// not know. A harness added to the registry and understood nowhere would
/// fail at configure time on a real build — this says so at test time
/// instead, and names the harness.
#[test]
fn the_build_file_understands_every_declared_harness() {
    let registry =
        W3cRegistry::load(&repo_root().join(W3C_REGISTRY_RELATIVE_PATH)).expect("registry loads");
    let declared: Vec<&str> = registry.harnesses.keys().map(String::as_str).collect();
    assert!(
        declared.len() >= 2,
        "only {} harness(es) declared; this assertion would be near-vacuous",
        declared.len(),
    );

    let iterated = iterated_harnesses(&read(REGISTRY_MODULE));
    assert!(
        !iterated.is_empty(),
        "_W3C_HARNESSES is empty, so the reading names no fixture's harness",
    );
    let unmapped: Vec<&&str> = declared
        .iter()
        .filter(|h| !iterated.iter().any(|i| i == *h))
        .collect();
    assert!(
        unmapped.is_empty(),
        "the registry declares harness(es) the registry reading does not iterate, so \
         every fixture naming one would refuse the configure: {unmapped:?} (iterates \
         {iterated:?})",
    );

    for route in &ROUTES {
        let code = code_lines(&read(route.file)).join("\n");
        let unrun: Vec<&&str> = declared
            .iter()
            .filter(|h| !code.contains(&format!("STREQUAL \"{h}\"")))
            .collect();
        assert!(
            unrun.is_empty(),
            "{} does not say what its runner needs for harness(es) {unrun:?}",
            route.file,
        );
    }
}

/// The body of the C11 route's runner loop: what each fixture's binary is
/// built and linked from.
fn c11_runner_loop(cmake: &str) -> Option<String> {
    let code = code_lines(cmake).join("\n");
    let start = code.find("foreach(TEST_NUM ${W3C_C_AOT_TESTS})")?;
    let end = code[start..].find("endforeach()")? + start;
    Some(code[start..end].to_string())
}

/// Every way the C11 route states by hand what a fixture links: a flag on a
/// registration, or a runner loop that names the library of a requirement
/// rather than linking what the generator states.
fn c11_link_violations(cmake: &str) -> Vec<String> {
    let mut found = Vec::new();
    let code = code_lines(cmake).join("\n");
    for flag in ["NEEDS_LUA", "NEEDS_DOM", "NEEDS_HTTP"] {
        if code.contains(flag) {
            found.push(format!(
                "`{flag}` is stated by hand; what a machine links is the generator's statement"
            ));
        }
    }
    let Some(body) = c11_runner_loop(cmake) else {
        found.push("no runner loop over W3C_C_AOT_TESTS".to_string());
        return found;
    };
    if !(body.contains("sce_c_runtime_libraries(")
        && body.contains("W3C_C_AOT_TEST_${TEST_NUM}_REQUIRES")
        && body.contains("sce_c_link_runtime(${_target}"))
    {
        found.push(
            "the runner loop does not leave out what `W3C_C_AOT_TEST_<id>_REQUIRES` cannot be \
             linked here (`sce_c_runtime_libraries`), or does not link each runner through \
             `sce_c_link_runtime`"
                .to_string(),
        );
    }
    for library in [
        "lua54",
        "sce_c_scripting",
        "sce_c_base",
        "sce_c_runtime_posix",
    ] {
        if body
            .split(|c: char| !(c.is_alphanumeric() || c == '_'))
            .any(|w| w == library)
        {
            found.push(format!(
                "the runner loop names `{library}`, the library of a requirement, by hand"
            ));
        }
    }
    found
}

#[test]
fn the_c11_route_links_what_the_generator_states() {
    let found = c11_link_violations(&read("backends/c/tests/CMakeLists.txt"));
    assert!(
        found.is_empty(),
        "backends/c/tests/CMakeLists.txt states by hand what a W3C fixture links:\n  {}",
        found.join("\n  ")
    );
}

/// The shapes this replaced are refused: a flag back on a registration, and a
/// library linked for every fixture of a list kept in the loop.
#[test]
fn a_link_stated_by_hand_is_refused() {
    let tree = read("backends/c/tests/CMakeLists.txt");

    let flagged = tree.replace(
        "sce_generate_static_w3c_c_test(${_W3C_C_ID} \"${SCE_C_GENERATED_DIR}\")",
        "sce_generate_static_w3c_c_test(${_W3C_C_ID} \"${SCE_C_GENERATED_DIR}\" NEEDS_LUA)",
    );
    assert_ne!(
        flagged, tree,
        "the C11 registration is no longer spelled as this case expects"
    );
    let found = c11_link_violations(&flagged);
    assert!(
        found.iter().any(|v| v.contains("`NEEDS_LUA`")),
        "a flagged registration was accepted: {found:?}"
    );

    let hand_linked = tree.replace(
        "sce_c_link_runtime(${_target} REQUIRES clock)",
        "target_link_libraries(${_target} PRIVATE sce_c_runtime sce_c_runtime_posix lua54)",
    );
    assert_ne!(
        hand_linked, tree,
        "the runner loop no longer links as this case expects"
    );
    let found = c11_link_violations(&hand_linked);
    assert!(
        found.iter().any(|v| v.contains("names `lua54`"))
            && found.iter().any(|v| v.contains("`sce_c_link_runtime`")),
        "a runner loop that links Lua by hand was accepted: {found:?}"
    );
}
