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
//!
//! The C++ route has one runner, `w3c_test_cli`, built natively and under
//! Emscripten, and the WASM build was the third copy. Its configure was
//! fenced off from the registration ("WASM cannot run sce-codegen") and read
//! the set back out of a CMake list the native configure wrote into the
//! source tree's `build/`, and the machines out of that native build tree:
//! without one it warned and went on with an empty set. A cross build runs a
//! host program at configure and at build time as a native one does, so the
//! set and the machines are read and generated under the conditions the
//! runner is declared under ([`wasm_fork_violations`]), and no file of a
//! route reaches into the source tree's `build/` ([`source_tree_build_violations`]).

use std::path::{Path, PathBuf};

use sce_build::w3c_registry::{W3cRegistry, W3C_REGISTRY_RELATIVE_PATH};

mod common;

use common::cmake_text::{cmake_code, strings_blanked};

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
    // The C++ route's programs and registrations, which every tree includes
    // (tests/CMakeLists.txt, and the emscripten tree without threads).
    Route {
        file: "tests/w3c/W3CPrograms.cmake",
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

// ── The WASM build reads the set the native build reads ───────────────

/// The module that declares the C++ route's registration function, and the
/// variables a registration accumulates into its caller's scope.
const CPP_ROUTE_MODULE: &str = "cmake/SCEStaticW3CTest.cmake";

/// The runner the C++ route registers its fixtures for, built natively and
/// under Emscripten from the one CMake file.
const CPP_RUNNER: &str = "w3c_test_cli";

/// The CMake list the native configure used to write into the source tree
/// for the WASM configure to `include()`. Spelled here because it is gone
/// from the tree: it is the shape this refuses, not a fact the tree states.
const RETIRED_LIST: &str = "W3CAotTestList";

/// The 1-based line of byte offset `at` of `text`.
fn line_of(text: &str, at: usize) -> usize {
    text[..at].matches('\n').count() + 1
}

/// The text between the parenthesis at `open` and the one that closes it,
/// with its whitespace collapsed: the condition of an `if(...)`.
fn argument_text(structure: &str, open: usize) -> String {
    let mut depth = 0usize;
    for (offset, c) in structure[open..].char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    let inner = &structure[open + 1..open + offset];
                    return inner.split_whitespace().collect::<Vec<_>>().join(" ");
                }
            }
            _ => {}
        }
    }
    structure[open + 1..]
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// One `if` block open at some point of a CMake file: the condition of every
/// branch reached so far, and whether its `else()` is the branch open.
struct OpenBlock {
    conditions: Vec<String>,
    in_else: bool,
}

/// The branch byte offset `at` of `structure` sits in, for every enclosing
/// `if` block whose conditions name the Emscripten toolchain, outermost
/// first. `structure` is a file's code with its strings blanked
/// (`strings_blanked`), so a block named inside a message opens nothing.
///
/// Two places of one file with the same answer are configured under the
/// same toolchains: neither is declared where the other is not.
fn emscripten_branches_at(structure: &str, at: usize) -> Vec<String> {
    let keyword = regex::Regex::new(r"(?i)\b(if|elseif|else|endif)\s*\(").expect("pattern");
    let emscripten = regex::Regex::new(r"\bEMSCRIPTEN\b").expect("pattern");
    let mut open: Vec<OpenBlock> = Vec::new();
    for found in keyword.captures_iter(&structure[..at]) {
        let whole = found.get(0).expect("the whole match");
        let condition = argument_text(structure, whole.end() - 1);
        match found[1].to_ascii_lowercase().as_str() {
            "if" => open.push(OpenBlock {
                conditions: vec![condition],
                in_else: false,
            }),
            "elseif" => {
                if let Some(block) = open.last_mut() {
                    block.conditions.push(condition);
                }
            }
            "else" => {
                if let Some(block) = open.last_mut() {
                    block.in_else = true;
                }
            }
            _ => {
                open.pop();
            }
        }
    }
    open.iter()
        .filter(|block| block.conditions.iter().any(|c| emscripten.is_match(c)))
        .map(|block| {
            let all = block.conditions.join(" / ");
            if block.in_else {
                format!("else() of if({all})")
            } else {
                format!("if({all}), branch {}", block.conditions.len())
            }
        })
        .collect()
}

/// The variables `function`'s body hands back to its caller as it found them
/// grown (`set(<V> ${<V>} PARENT_SCOPE)`): what a registration accumulates.
fn accumulated_variables(module: &str, function: &str) -> Vec<String> {
    let code = cmake_code(module);
    let Some(start) = code.find(&format!("function({function} ")) else {
        return Vec::new();
    };
    let end = code[start..]
        .find("endfunction(")
        .map_or(code.len(), |offset| start + offset);
    let re =
        regex::Regex::new(r"set\(\s*([A-Za-z0-9_]+)\s+\$\{([A-Za-z0-9_]+)\}\s+PARENT_SCOPE\s*\)")
            .expect("pattern");
    let mut found: Vec<String> = re
        .captures_iter(&code[start..end])
        .filter(|c| c[1] == c[2])
        .map(|c| c[1].to_string())
        .collect();
    found.sort();
    found.dedup();
    found
}

/// Every way `tree` lets the WASM build of [`CPP_RUNNER`] take its fixtures,
/// or their machines, from anywhere but where the native build takes them:
/// the inclusion of the route module, the reading of the registry, the
/// prefetch of the plans, every registration, and every assignment of a
/// variable a registration accumulates (`module`, [`accumulated_variables`])
/// or writes its machines into (the registration's second argument), each
/// under an Emscripten branch the runner's declaration is not under.
///
/// The variables are read from the route rather than named, so a variable a
/// registration starts to accumulate tomorrow is held as the ones it
/// accumulates today.
fn wasm_fork_violations(tree: &str, module: &str, registration: &str) -> Vec<String> {
    let mut found = Vec::new();
    let structure = strings_blanked(&cmake_code(tree));
    let Some(runner_at) = structure.find(&format!("add_executable({CPP_RUNNER}")) else {
        found.push(format!("no `add_executable({CPP_RUNNER} ...)`"));
        return found;
    };
    let expected = emscripten_branches_at(&structure, runner_at);

    let mut variables = accumulated_variables(module, registration);
    if variables.is_empty() {
        found.push(format!(
            "`{registration}` in the route module hands no accumulated variable back to its \
             caller, so nothing here says where the runner's set comes from"
        ));
    }
    let output_dir = regex::Regex::new(&format!(
        r"\b{}\(\s*\S+\s+\$\{{([A-Za-z0-9_]+)\}}",
        regex::escape(registration)
    ))
    .expect("pattern");
    let mut output_dirs: Vec<String> = output_dir
        .captures_iter(&structure)
        .map(|c| c[1].to_string())
        .collect();
    output_dirs.sort();
    output_dirs.dedup();
    if output_dirs.is_empty() {
        found.push(format!(
            "no `{registration}(<id> ${{<dir>}} ...)` names the directory its machines are \
             written into"
        ));
    }
    variables.extend(output_dirs);

    let module_name = Path::new(CPP_ROUTE_MODULE)
        .file_name()
        .and_then(|name| name.to_str())
        .expect("the route module has a file name");
    let anchors = [
        module_name.to_string(),
        "sce_w3c_registry_fixtures(".to_string(),
        "sce_prefetch_static_w3c_tests(".to_string(),
        format!("{registration}("),
    ];
    let mut places: Vec<(String, usize)> = Vec::new();
    for anchor in &anchors {
        let at: Vec<usize> = structure
            .match_indices(anchor.as_str())
            .map(|(i, _)| i)
            .collect();
        if at.is_empty() {
            found.push(format!(
                "`{anchor}` does not occur, so the route is not read here"
            ));
        }
        places.extend(at.into_iter().map(|i| (format!("`{anchor}`"), i)));
    }
    for variable in &variables {
        let assignment = regex::Regex::new(&format!(
            r"\b(?:set\(\s*|list\(\s*[A-Za-z_]+\s+){}[\s)]",
            regex::escape(variable)
        ))
        .expect("pattern");
        let at: Vec<usize> = assignment
            .find_iter(&structure)
            .map(|m| m.start())
            .collect();
        if at.is_empty() {
            found.push(format!("`{variable}` is never assigned here"));
        }
        places.extend(
            at.into_iter()
                .map(|i| (format!("the assignment of `{variable}`"), i)),
        );
    }

    for (what, at) in places {
        let branches = emscripten_branches_at(&structure, at);
        if branches != expected {
            found.push(format!(
                "{what} at line {} is configured under {branches:?}, and `{CPP_RUNNER}` under \
                 {expected:?}: one toolchain's runner would take its fixtures from somewhere \
                 else",
                line_of(&structure, at)
            ));
        }
    }
    found
}

#[test]
fn the_wasm_build_reads_the_set_the_native_build_reads() {
    let route = &ROUTES[0];
    let found = wasm_fork_violations(
        &read(route.file),
        &read(CPP_ROUTE_MODULE),
        route.registration,
    );
    assert!(
        found.is_empty(),
        "{} does not register and generate the fixtures of `{CPP_RUNNER}` under the \
         toolchains it builds `{CPP_RUNNER}` under:\n  {}\nThe generator is a host program: \
         a cross build runs it at configure and at build time as a native one does.",
        route.file,
        found.join("\n  ")
    );
}

/// The shape this replaced is refused, and the derived one is not: a set read
/// only outside Emscripten, with a WASM branch that points the machines at a
/// native build tree; a fence that is only a comment is no fence.
#[test]
fn a_wasm_branch_with_a_set_of_its_own_is_refused() {
    let module = read(CPP_ROUTE_MODULE);
    let registration = ROUTES[0].registration;
    let registered = format!(
        "include(${{CMAKE_SOURCE_DIR}}/{CPP_ROUTE_MODULE})\n\
         set(STATIC_W3C_OUTPUT_DIR \"${{CMAKE_CURRENT_BINARY_DIR}}/w3c_static_generated\")\n\
         set(GENERATED_W3C_HEADERS \"\")\n\
         set(W3C_AOT_TESTS \"\")\n\
         sce_w3c_registry_fixtures(_IDS _HARNESS_OF_)\n\
         sce_prefetch_static_w3c_tests(\"${{STATIC_W3C_OUTPUT_DIR}}\" cpp ${{_IDS}})\n\
         foreach(_ID ${{_IDS}})\n\
         \x20   {registration}(${{_ID}} ${{STATIC_W3C_OUTPUT_DIR}})\n\
         endforeach()\n"
    );
    let runner = format!(
        "if(EMSCRIPTEN)\n\
         \x20   message(\"if(NOT EMSCRIPTEN) is text\")\n\
         endif()\n\
         add_executable({CPP_RUNNER} ${{GENERATED_W3C_HEADERS}})\n"
    );

    let derived = format!("# if(NOT EMSCRIPTEN)\n{registered}{runner}");
    assert_eq!(
        wasm_fork_violations(&derived, &module, registration),
        Vec::<String>::new(),
        "the derived shape was refused"
    );

    let forked = format!(
        "if(NOT EMSCRIPTEN)\n{registered}endif()\n\
         if(EMSCRIPTEN)\n\
         \x20   set(STATIC_W3C_OUTPUT_DIR \"${{CMAKE_SOURCE_DIR}}/build/tests/w3c_static_generated\")\n\
         endif()\n{runner}"
    );
    let found = wasm_fork_violations(&forked, &module, registration);
    for refused in [
        "`sce_w3c_registry_fixtures(`",
        "`W3C_AOT_TESTS`",
        "`GENERATED_W3C_HEADERS`",
        "`STATIC_W3C_OUTPUT_DIR` at line 13",
    ] {
        assert!(
            found.iter().any(|v| v.contains(refused)),
            "{refused} was accepted in the forked shape: {found:?}"
        );
    }
}

// ── No route reaches into the source tree's build directory ───────────

/// Every CMake file a W3C route is made of, with its text: each route's own
/// file and every module it includes, followed through the modules' own
/// includes. Read from the `include()`s rather than listed, so a module a
/// route starts to include is scanned on the day it does.
fn route_files() -> Vec<(String, String)> {
    let include = regex::Regex::new(
        r#"\binclude\(\s*"?\$\{(CMAKE_SOURCE_DIR|CMAKE_CURRENT_LIST_DIR)\}/([^")\s]+)"?\s*\)"#,
    )
    .expect("pattern");
    let mut pending: Vec<String> = ROUTES.iter().map(|r| r.file.to_string()).collect();
    let mut files: Vec<(String, String)> = Vec::new();
    while let Some(path) = pending.pop() {
        if files.iter().any(|(seen, _)| *seen == path) {
            continue;
        }
        let text = read(&path);
        for c in include.captures_iter(&cmake_code(&text)) {
            let relative = &c[2];
            let included = if &c[1] == "CMAKE_SOURCE_DIR" {
                relative.to_string()
            } else {
                let dir = Path::new(&path).parent().expect("a file has a directory");
                dir.join(relative).to_string_lossy().into_owned()
            };
            pending.push(included);
        }
        files.push((path, text));
    }
    files
}

/// Every place `files` (each a path and its text) names the source tree's
/// own `build/` directory, or the retired list, in code: a configure that
/// writes there violates the out-of-source build, and one that reads from
/// there depends on a second build tree having been configured first under
/// a name nothing declares.
fn source_tree_build_violations(files: &[(String, String)]) -> Vec<String> {
    let build_dir = regex::Regex::new(
        r#"\$\{(?:CMAKE_SOURCE_DIR|PROJECT_SOURCE_DIR|CMAKE_CURRENT_SOURCE_DIR|CMAKE_CURRENT_LIST_DIR)\}(?:/\.\.)*/build(?:[/"\s)]|$)"#,
    )
    .expect("pattern");
    let mut found = Vec::new();
    for (path, text) in files {
        let code = cmake_code(text);
        for (number, line) in code.lines().enumerate() {
            if build_dir.is_match(line) {
                found.push(format!(
                    "{path}:{}: names the source tree's build directory: {}",
                    number + 1,
                    line.trim()
                ));
            }
            if line.contains(RETIRED_LIST) {
                found.push(format!(
                    "{path}:{}: names `{RETIRED_LIST}`: {}",
                    number + 1,
                    line.trim()
                ));
            }
        }
    }
    found
}

#[test]
fn no_route_file_reaches_into_the_source_trees_build_directory() {
    let files = route_files();
    let scanned: Vec<&str> = files.iter().map(|(path, _)| path.as_str()).collect();
    for expected in [REGISTRY_MODULE, CPP_ROUTE_MODULE] {
        assert!(
            scanned.contains(&expected),
            "{expected} did not reach the scan through the routes' includes ({scanned:?}); \
             the include reading is broken, not the tree"
        );
    }
    let found = source_tree_build_violations(&files);
    assert!(
        found.is_empty(),
        "a W3C route reaches into the source tree's build directory:\n  {}\nEvery file a \
         configure writes belongs in its own build tree, and a toolchain reads the set from \
         the registry rather than from another toolchain's build.",
        found.join("\n  ")
    );
}

#[test]
fn a_list_written_into_the_source_tree_is_refused() {
    let text = format!(
        "set(_LIST \"${{CMAKE_SOURCE_DIR}}/build/{RETIRED_LIST}.cmake\")\n\
         # include(\"${{CMAKE_SOURCE_DIR}}/build/{RETIRED_LIST}.cmake\")\n\
         set(_DIR \"${{CMAKE_SOURCE_DIR}}/build_wasm\")\n\
         set(_GEN ${{CMAKE_SOURCE_DIR}}/build)\n"
    );
    let found = source_tree_build_violations(&[("probe.cmake".to_string(), text)]);
    assert!(
        found
            .iter()
            .any(|v| v.starts_with("probe.cmake:1: names the source tree's build"))
            && found
                .iter()
                .any(|v| v.starts_with(&format!("probe.cmake:1: names `{RETIRED_LIST}`"))),
        "a list written into the source tree's build directory was accepted: {found:?}"
    );
    assert!(
        found.iter().any(|v| v.starts_with("probe.cmake:4:")),
        "the bare build directory was accepted: {found:?}"
    );
    assert!(
        !found
            .iter()
            .any(|v| v.starts_with("probe.cmake:2:") || v.starts_with("probe.cmake:3:")),
        "a comment, or a directory merely beginning with `build`, was refused: {found:?}"
    );
}
