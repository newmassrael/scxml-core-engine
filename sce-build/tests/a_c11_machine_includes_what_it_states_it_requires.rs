// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A generated C11 machine states what it needs linked beside it
//! (`GeneratedOutput::requires`, `RuntimeRequirement`), and what it states is
//! what it includes.
//!
//! The C11 conformance suite used to say which of its 202 fixtures link Lua,
//! the DOM binding or the HTTP client with flags written beside each id, a
//! second account of a decision the generator makes when it renders the
//! machine. The build now links what the generator states
//! (`sce-codegen --plan --plan-requires`), so the statement has to be the
//! generator's decision itself and not a classifier beside it that can drift:
//! the templates include each requirement's interface because the set names
//! it. What holds that here is the generated text, which is what a compiler
//! reads: for every machine of every fixture the registry lists, each
//! requirement is stated exactly when the machine includes its interface. A
//! machine that included `lua.h` and stated no `lua` would compile against a
//! link that lacks Lua; one that stated `lua` and included nothing would link
//! an engine it never calls.
//!
//! Two further facts are held to it. The registry's `http` harness, which the
//! build reads to start the BasicHTTP server before a fixture runs, names
//! exactly the fixtures whose machines state `http`: the harness is how a
//! fixture is run, the statement is what it links, and the two describe one
//! fact from two sides. And every requirement the generator can state is one
//! the C11 runtime names a library for (`SCE_C_RUNTIME_REQUIREMENTS`,
//! backends/c/runtime/CMakeLists.txt), so a requirement added to the
//! generator cannot reach a build that links nothing for it.
//!
//! Each fixture is generated as the conformance route generates it
//! (cmake/SCEStaticW3CTest.cmake): `sce-codegen generate` on the staged
//! document with the fixture's directory as input root, then every child
//! generation its plan holds, each statement read from the run's manifest. A
//! library entry of its own refused documents the route accepts (test183's
//! discarded self-send), so it judged a generation no build runs.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use sce_build::generator::RuntimeRequirement;
use sce_build::w3c_registry::{W3cFixture, W3cRegistry, W3C_REGISTRY_RELATIVE_PATH};

/// The C11 runtime's build, which names the library of every requirement.
const C11_RUNTIME: &str = include_str!("../../backends/c/runtime/CMakeLists.txt");

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// Whether a C translation unit or header among `files` includes the
/// interface of `requirement`, by the line its templates write.
fn includes(files: &[(String, String)], requirement: RuntimeRequirement) -> bool {
    files
        .iter()
        .filter(|(name, _)| name.ends_with(".c") || name.ends_with(".h"))
        .any(|(_, text)| {
            text.lines()
                .any(|line| line.trim() == requirement.c11_include())
        })
}

/// Every requirement `stated` and `files` disagree on: stated and not
/// included, or included and not stated.
fn misstatements(files: &[(String, String)], stated: &BTreeSet<RuntimeRequirement>) -> Vec<String> {
    RuntimeRequirement::ALL
        .into_iter()
        .filter_map(|requirement| {
            let included = includes(files, requirement);
            let named = stated.contains(&requirement);
            (included != named).then(|| {
                if included {
                    format!(
                        "includes `{}` and does not state `{}`",
                        requirement.c11_include(),
                        requirement.wire_name()
                    )
                } else {
                    format!(
                        "states `{}` and does not include `{}`",
                        requirement.wire_name(),
                        requirement.c11_include()
                    )
                }
            })
        })
        .collect()
}

/// One generation the conformance route runs for a fixture: the C and header
/// files it wrote, each with its text, and what its manifest states it
/// requires (`runtime_requires`).
struct Generation {
    files: Vec<(String, String)>,
    requires: BTreeSet<RuntimeRequirement>,
}

/// One fixture's C11 output: its id, its registry harness, and every
/// generation the route runs for it, the fixture's own first.
type FixtureOutput = (String, String, Vec<Generation>);

/// What every generation of a fixture requires.
fn fixture_requires(generations: &[Generation]) -> BTreeSet<RuntimeRequirement> {
    generations
        .iter()
        .flat_map(|generation| generation.requires.iter().copied())
        .collect()
}

/// `sce-codegen <args>`, which has to succeed; its standard output.
fn codegen(args: &[&str], what: &str) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
        .args(args)
        .output()
        .expect("spawn sce-codegen");
    assert!(
        output.status.success(),
        "{what} was refused:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// One generation run as the route runs it, read back from its manifest.
fn generation(
    document: &str,
    extra: &[String],
    out: &str,
    input_root: &str,
    what: &str,
) -> Generation {
    let mut args = vec!["generate", document];
    args.extend(extra.iter().map(String::as_str));
    args.extend([
        "-l",
        "c11",
        "-o",
        out,
        "--input-root",
        input_root,
        "--no-format",
    ]);
    let stdout = codegen(&args, what);
    let manifest: serde_json::Value = serde_json::from_str(
        stdout
            .lines()
            .last()
            .unwrap_or_else(|| panic!("{what} wrote no manifest")),
    )
    .unwrap_or_else(|e| panic!("{what}: the manifest is not JSON: {e}"));
    let files = manifest["artifacts"]
        .as_array()
        .unwrap_or_else(|| panic!("{what}: the manifest lists no artifacts"))
        .iter()
        .map(|artifact| artifact["path"].as_str().expect("an artifact has a path"))
        .filter(|path| path.ends_with(".c") || path.ends_with(".h"))
        .map(|path| {
            let text = std::fs::read_to_string(path)
                .unwrap_or_else(|e| panic!("{what}: read {path}: {e}"));
            (path.to_string(), text)
        })
        .collect();
    let requires = manifest["runtime_requires"]
        .as_array()
        .unwrap_or_else(|| panic!("{what}: a C11 manifest states no runtime_requires"))
        .iter()
        .map(|name| {
            let name = name.as_str().expect("a requirement is a string");
            RuntimeRequirement::ALL
                .into_iter()
                .find(|requirement| requirement.wire_name() == name)
                .unwrap_or_else(|| panic!("{what}: `{name}` is no requirement the generator knows"))
        })
        .collect();
    Generation { files, requires }
}

/// Every generation the conformance route (cmake/SCEStaticW3CTest.cmake) runs
/// for `fixture`, run as it runs them: the fixture's document staged into the
/// output directory with its sidecar and generated with the fixture's
/// directory as input root, then each child document the plan of it holds,
/// with the arguments the plan states for it.
fn route_generations(root: &Path, fixture: &W3cFixture) -> Vec<Generation> {
    let stem = format!("test{}", fixture.id);
    let resources = root.join(format!("resources/{}", fixture.id));
    let dir = tempfile::tempdir().expect("tempdir");
    let out = dir.path();
    for staged in [format!("{stem}.scxml"), format!("{stem}.txt")] {
        if resources.join(&staged).exists() {
            std::fs::copy(resources.join(&staged), out.join(&staged))
                .unwrap_or_else(|e| panic!("{stem}: stage {staged}: {e}"));
        }
    }
    let document = out.join(format!("{stem}.scxml"));
    let committed = resources.join(format!("{stem}.scxml"));
    let (document, committed, out_dir, input_root) = (
        document.to_str().expect("a UTF-8 path"),
        committed.to_str().expect("a UTF-8 path"),
        out.to_str().expect("a UTF-8 path"),
        resources.to_str().expect("a UTF-8 path"),
    );

    // The child generations the route declares, from the plan it asks of the
    // committed document: a generation's first line is
    // `<child document>\t\t<argument>...`.
    let plan = codegen(
        &[
            "--plan",
            "--plan-children",
            "generate",
            committed,
            "-l",
            "c11",
            "-o",
            out_dir,
            "--input-root",
            input_root,
            "--no-format",
        ],
        &format!("{stem}: the plan of the C11 generation the conformance route builds"),
    );
    let children: Vec<(String, Vec<String>)> = plan
        .lines()
        .filter_map(|line| {
            let (child, rest) = line.split_once('\t')?;
            let arguments = rest.strip_prefix('\t')?;
            Some((
                child.to_string(),
                arguments
                    .split('\t')
                    .filter(|argument| !argument.is_empty())
                    .map(str::to_string)
                    .collect(),
            ))
        })
        .collect();
    // The child documents the fixture's directory holds are staged beside it
    // (`CHILDREN_HELD_IN`): the run reads them rather than writing them.
    for line in plan.lines().filter(|line| !line.contains('\t')) {
        let Some(name) = Path::new(line).file_name() else {
            continue;
        };
        if line.ends_with(".scxml") && resources.join(name).exists() {
            std::fs::copy(resources.join(name), out.join(name))
                .unwrap_or_else(|e| panic!("{stem}: stage the child {line}: {e}"));
        }
    }

    let mut generations = vec![generation(
        document,
        &[],
        out_dir,
        input_root,
        &format!("{stem}: the C11 generation the conformance route builds"),
    )];
    for (child, arguments) in &children {
        generations.push(generation(
            child,
            arguments,
            out_dir,
            input_root,
            &format!("{stem}: the generation of its child {child}"),
        ));
    }
    generations
}

/// The C11 output of every fixture the registry lists, generated once for
/// every test here, by the generator the conformance route runs and with its
/// arguments: a library entry of its own would be a second route, and one that
/// refused documents the route accepts.
fn registry_outputs() -> &'static [FixtureOutput] {
    static OUTPUTS: OnceLock<Vec<FixtureOutput>> = OnceLock::new();
    OUTPUTS.get_or_init(|| {
        let root = repo_root();
        let registry =
            W3cRegistry::load(&root.join(W3C_REGISTRY_RELATIVE_PATH)).expect("the registry loads");
        let fixtures = registry.fixtures();
        let workers = std::thread::available_parallelism().map_or(4, usize::from);
        let chunk = fixtures.len().div_ceil(workers).max(1);
        std::thread::scope(|scope| {
            let handles: Vec<_> = fixtures
                .chunks(chunk)
                .map(|part| {
                    let root = &root;
                    scope.spawn(move || {
                        part.iter()
                            .map(|fixture| {
                                (
                                    fixture.id.clone(),
                                    fixture.harness.clone(),
                                    route_generations(root, fixture),
                                )
                            })
                            .collect::<Vec<FixtureOutput>>()
                    })
                })
                .collect();
            handles
                .into_iter()
                .flat_map(|handle| handle.join().expect("a fixture's generation panicked"))
                .collect()
        })
    })
}

#[test]
fn every_registry_fixture_states_what_its_c11_machine_includes() {
    let outputs = registry_outputs();
    assert!(
        outputs.len() >= 200,
        "only {} fixture(s) generated; the registry read is broken, and a clean result \
         would prove nothing",
        outputs.len()
    );

    let mut found = Vec::new();
    for (id, _, generations) in outputs {
        for generation in generations {
            for misstatement in misstatements(&generation.files, &generation.requires) {
                found.push(format!("test{id}: {misstatement}"));
            }
        }
    }
    assert!(
        found.is_empty(),
        "a generated C11 machine states something other than what it includes, so the \
         build would link it with the wrong libraries:\n  {}",
        found.join("\n  ")
    );

    // Each requirement is stated by some fixture and left out by another, or
    // the comparison above measured nothing for it.
    for requirement in RuntimeRequirement::ALL {
        let stating = outputs
            .iter()
            .filter(|(_, _, generations)| fixture_requires(generations).contains(&requirement))
            .count();
        assert!(
            stating > 0 && stating < outputs.len(),
            "`{}` is stated by {stating} of {} fixtures; the corpus no longer tells a \
             machine that needs it from one that does not",
            requirement.wire_name(),
            outputs.len()
        );
    }
    assert!(
        outputs
            .iter()
            .any(|(_, _, generations)| fixture_requires(generations).is_empty()),
        "no fixture's machine states nothing, so the freestanding profile is unmeasured"
    );
}

#[test]
fn the_http_harness_names_the_machines_that_state_http() {
    let found: Vec<String> = registry_outputs()
        .iter()
        .filter_map(|(id, harness, generations)| {
            let runs_the_server = harness == "http";
            let sends = fixture_requires(generations).contains(&RuntimeRequirement::Http);
            (runs_the_server != sends).then(|| {
                format!(
                    "test{id}: harness `{harness}`, and its machine {} `http`",
                    if sends { "states" } else { "does not state" }
                )
            })
        })
        .collect();
    assert!(
        found.is_empty(),
        "the registry's `http` harness and the generator's `http` requirement disagree, so a \
         fixture would run without the server its machine posts to, or with one nothing \
         posts to:\n  {}",
        found.join("\n  ")
    );
}

/// What `misstatements` is asked of a real machine, wrong in each direction,
/// is reported: a statement that drops what the machine includes, and one
/// that adds what it does not.
#[test]
fn a_statement_that_is_not_what_the_machine_includes_is_refused() {
    let outputs = registry_outputs();
    let every = || {
        outputs
            .iter()
            .flat_map(|(id, _, generations)| generations.iter().map(move |g| (id, g)))
    };
    let (scripted_id, scripted) = every()
        .find(|(_, generation)| generation.requires.contains(&RuntimeRequirement::Lua))
        .expect("a fixture whose machine embeds Lua");
    let mut dropped = scripted.requires.clone();
    dropped.remove(&RuntimeRequirement::Lua);
    let found = misstatements(&scripted.files, &dropped);
    assert!(
        found.iter().any(|v| v.contains("does not state `lua`")),
        "test{scripted_id}: a statement without the Lua its machine includes was accepted: \
         {found:?}"
    );

    let (plain_id, plain) = every()
        .find(|(_, generation)| generation.requires.is_empty())
        .expect("a fixture whose machine requires nothing");
    let added: BTreeSet<RuntimeRequirement> = [RuntimeRequirement::Http].into_iter().collect();
    let found = misstatements(&plain.files, &added);
    assert!(
        found
            .iter()
            .any(|v| v.contains("states `http` and does not include")),
        "test{plain_id}: a statement of an HTTP client its machine never includes was \
         accepted: {found:?}"
    );

    // A header that is not C is not read: the include line in a document the
    // generation wrote beside the machine says nothing about the machine.
    let beside = [(
        "child.scxml".to_string(),
        "#include \"lua.h\"\n".to_string(),
    )];
    assert!(misstatements(&beside, &BTreeSet::new()).is_empty());
}

/// The requirements the C11 runtime's build knows, in the order it lists
/// them (`set_property(GLOBAL PROPERTY SCE_C_RUNTIME_REQUIREMENTS ...)`).
fn runtime_vocabulary(cmake: &str) -> Vec<String> {
    let re =
        regex::Regex::new(r"set_property\(GLOBAL PROPERTY SCE_C_RUNTIME_REQUIREMENTS([^)]*)\)")
            .expect("pattern");
    re.captures(cmake)
        .map(|c| c[1].split_whitespace().map(str::to_string).collect())
        .unwrap_or_default()
}

/// Every requirement the generator can state and the runtime's build does
/// not know, or knows and registers no library for.
fn unmapped_requirements(cmake: &str) -> Vec<String> {
    let known = runtime_vocabulary(cmake);
    RuntimeRequirement::ALL
        .into_iter()
        .map(RuntimeRequirement::wire_name)
        .filter_map(|name| {
            if !known.iter().any(|k| k == name) {
                Some(format!("`{name}` is not in SCE_C_RUNTIME_REQUIREMENTS"))
            } else if !cmake.contains(&format!(
                "set_property(GLOBAL PROPERTY SCE_C_RUNTIME_REQUIREMENT_{name} "
            )) {
                Some(format!("no tier registers a library for `{name}`"))
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn every_requirement_the_generator_states_is_one_the_runtime_links() {
    let known = runtime_vocabulary(C11_RUNTIME);
    assert!(
        !known.is_empty(),
        "backends/c/runtime/CMakeLists.txt declares no SCE_C_RUNTIME_REQUIREMENTS; the \
         pattern is broken, not the build file"
    );
    let found = unmapped_requirements(C11_RUNTIME);
    assert!(
        found.is_empty(),
        "the generator can state a requirement the C11 runtime does not link:\n  {}",
        found.join("\n  ")
    );
    let stated: Vec<&str> = RuntimeRequirement::ALL
        .into_iter()
        .map(RuntimeRequirement::wire_name)
        .collect();
    let unknown: Vec<&String> = known
        .iter()
        .filter(|k| !stated.contains(&k.as_str()))
        .collect();
    assert!(
        unknown.is_empty(),
        "backends/c/runtime/CMakeLists.txt knows requirement(s) the generator never states: \
         {unknown:?}"
    );
}

/// The runtime's build with `left_out` taken out of its vocabulary, the
/// declaration rebuilt from the vocabulary it parses rather than from a
/// spelling of it, so a requirement added to both sides keeps the case live.
fn without_in_vocabulary(cmake: &str, left_out: &str) -> String {
    let re = regex::Regex::new(r"set_property\(GLOBAL PROPERTY SCE_C_RUNTIME_REQUIREMENTS[^)]*\)")
        .expect("pattern");
    let declared = re
        .find(cmake)
        .expect("backends/c/runtime/CMakeLists.txt declares SCE_C_RUNTIME_REQUIREMENTS")
        .as_str();
    let kept: Vec<String> = runtime_vocabulary(cmake)
        .into_iter()
        .filter(|name| name != left_out)
        .collect();
    cmake.replacen(
        declared,
        &format!(
            "set_property(GLOBAL PROPERTY SCE_C_RUNTIME_REQUIREMENTS {})",
            kept.join(" ")
        ),
        1,
    )
}

#[test]
fn a_requirement_the_runtime_does_not_link_is_refused() {
    // Each requirement, left out of the vocabulary in turn, is reported.
    for requirement in RuntimeRequirement::ALL.map(RuntimeRequirement::wire_name) {
        let forgotten = without_in_vocabulary(C11_RUNTIME, requirement);
        assert_ne!(
            forgotten, C11_RUNTIME,
            "`{requirement}` is not in the vocabulary this case parsed"
        );
        let found = unmapped_requirements(&forgotten);
        assert!(
            found.iter().any(|v| v.contains(&format!(
                "`{requirement}` is not in SCE_C_RUNTIME_REQUIREMENTS"
            ))),
            "`{requirement}` missing from the vocabulary was accepted: {found:?}"
        );
    }

    let unregistered = C11_RUNTIME.replace(
        "set_property(GLOBAL PROPERTY SCE_C_RUNTIME_REQUIREMENT_dom sce_c_scripting)",
        "",
    );
    assert_ne!(
        unregistered, C11_RUNTIME,
        "the DOM binding's registration is no longer spelled as this case expects"
    );
    let found = unmapped_requirements(&unregistered);
    assert!(
        found
            .iter()
            .any(|v| v.contains("no tier registers a library for `dom`")),
        "a requirement no tier registers was accepted: {found:?}"
    );
}
