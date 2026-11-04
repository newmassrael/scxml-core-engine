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

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use sce_build::generator::{GeneratedOutput, Language, RuntimeRequirement};
use sce_build::w3c_registry::{W3cRegistry, W3C_REGISTRY_RELATIVE_PATH};
use sce_build::{compile_scxml_lang_typed, find_template_dir_for};

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

/// One fixture's C11 output: its id, its registry harness, and what the
/// generation produced.
type FixtureOutput = (String, String, GeneratedOutput);

/// The C11 output of every fixture the registry lists, generated once for
/// every test here.
fn registry_outputs() -> &'static [FixtureOutput] {
    static OUTPUTS: OnceLock<Vec<FixtureOutput>> = OnceLock::new();
    OUTPUTS.get_or_init(|| {
        let root = repo_root();
        let registry =
            W3cRegistry::load(&root.join(W3C_REGISTRY_RELATIVE_PATH)).expect("the registry loads");
        let template_dir = find_template_dir_for(Language::C11);
        registry
            .fixtures()
            .iter()
            .map(|fixture| {
                let document = root.join(format!("resources/{0}/test{0}.scxml", fixture.id));
                let output = compile_scxml_lang_typed(
                    document.to_str().expect("a UTF-8 path"),
                    &template_dir,
                    Language::C11,
                )
                .unwrap_or_else(|e| {
                    panic!(
                        "test{}: the C11 generation the conformance route builds was refused: \
                         {e:?}",
                        fixture.id
                    )
                });
                (fixture.id.clone(), fixture.harness.clone(), output)
            })
            .collect()
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
    for (id, _, output) in outputs {
        for misstatement in misstatements(&output.files, &output.requires) {
            found.push(format!("test{id}: {misstatement}"));
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
            .filter(|(_, _, output)| output.requires.contains(&requirement))
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
            .any(|(_, _, output)| output.requires.is_empty()),
        "no fixture's machine states nothing, so the freestanding profile is unmeasured"
    );
}

#[test]
fn the_http_harness_names_the_machines_that_state_http() {
    let found: Vec<String> = registry_outputs()
        .iter()
        .filter_map(|(id, harness, output)| {
            let runs_the_server = harness == "http";
            let sends = output.requires.contains(&RuntimeRequirement::Http);
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
    let (scripted_id, _, scripted) = outputs
        .iter()
        .find(|(_, _, output)| output.requires.contains(&RuntimeRequirement::Lua))
        .expect("a fixture whose machine embeds Lua");
    let mut dropped = scripted.requires.clone();
    dropped.remove(&RuntimeRequirement::Lua);
    let found = misstatements(&scripted.files, &dropped);
    assert!(
        found.iter().any(|v| v.contains("does not state `lua`")),
        "test{scripted_id}: a statement without the Lua its machine includes was accepted: \
         {found:?}"
    );

    let (plain_id, _, plain) = outputs
        .iter()
        .find(|(_, _, output)| output.requires.is_empty())
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

#[test]
fn a_requirement_the_runtime_does_not_link_is_refused() {
    let forgotten = C11_RUNTIME.replace(
        "SCE_C_RUNTIME_REQUIREMENTS lua dom http)",
        "SCE_C_RUNTIME_REQUIREMENTS lua dom)",
    );
    assert_ne!(
        forgotten, C11_RUNTIME,
        "the vocabulary is no longer spelled as this case expects"
    );
    let found = unmapped_requirements(&forgotten);
    assert!(
        found
            .iter()
            .any(|v| v.contains("`http` is not in SCE_C_RUNTIME_REQUIREMENTS")),
        "a requirement missing from the vocabulary was accepted: {found:?}"
    );

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
