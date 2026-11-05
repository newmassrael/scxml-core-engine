// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The CMake route of the conformance suite declares, for every generation it
//! runs, exactly the files the generator says that generation writes.
//!
//! `cmake/SCEStaticW3CTest.cmake` used to list by hand the files a C++ or C11
//! fixture's generation writes: the machine, the children found by globbing the
//! fixture's directory, their headers. A document that declared something new
//! (`sce:transition-function="emit"` writes a header beside the machine) then
//! wrote a file no rule owned, so the corpus was held to never declaring it.
//! The route now asks `sce-codegen --plan generate` through the one helper the
//! integration route asks it through (`cmake/SCECodegenPlan.cmake`), and this
//! file holds the shape that makes that true, read from the CMake text, since
//! CMake is not run here:
//!
//! * every command the route declares runs the generator, and declares its
//!   OUTPUT, its DEPFILE and its byproducts as the variables a
//!   `sce_codegen_plan` call in the same file set, and asks the generator for
//!   the depfile it declares;
//! * the route spells no fragment of a generated file's name and finds no file
//!   by a name pattern, so there is no second list for a declaration to fall
//!   out of;
//! * one module asks the generator for a plan, and both routes ask it.
//!
//! A hand-kept declaration is caught by the first rule whatever it names,
//! which is what makes this stronger than a list of the names a declaration
//! could add today. What it cannot see is CMake's own evaluation: that the
//! helper's answer reaches the declaration is held by CMake refusing a command
//! whose OUTPUT is empty, and by the helper refusing a plan that does not name
//! the machine.
//!
//! The integration route (`cmake/SCEStaticIntegrationFixture.cmake`) is held
//! to the same rules. It spelled a hybrid child's machine by hand, because the
//! stub that child is generated from is written by the parent's generation
//! and so could not be planned before the build ran; the generator now plans
//! it from the text the parent's generation would write (`--plan-children`),
//! and the route declares it from that answer
//! (`sce_codegen_plan(<P> FROM <plan> CHILD <name>)`).
//!
//! Both routes declare every child generation that way, and only that way:
//! every plan a route asks of the generator asks for its children's
//! generations too, every child command is declared from a plan read FROM
//! that answer, and no route reads the child document names a plan sets
//! (`<PREFIX>_CHILDREN`, `<PREFIX>_SYNTH`) to plan a child by its name. The
//! conformance route planned each child document a fixture's answer named by
//! its path in the fixture's directory, so a hybrid `<invoke>`'s stub, which
//! only a generation writes, had to be tracked there for the route to find
//! it.
//!
//! The test tree's build (`tests/CMakeLists.txt`) declared its Mesh
//! fixtures' generations by hand, 139 commands as written, 45 of them a
//! transport-only run beside a machine run of the same document that wrote
//! the transport too, undeclared. It now declares every generation through
//! `sce_codegen_generation`, whose one command the module holds to its plan,
//! and is held to running no generation of its own, spelling no generated
//! name and splitting no generation ([`test_tree_violations`]).
//!
//! The module keeps each answer between configures and asks the plans it does
//! not keep many at once (`sce-codegen plan-batch`). That changes where an
//! answer is read from and not what it is: every plan is still the
//! generator's, run in the one place the module runs it, as a `--plan
//! generate` request; a plan the conformance route asks ahead is the plan it
//! then declares, word for word ([`ahead_violations`]); and a kept answer is
//! keyed on every environment variable the generator reads
//! ([`the_key_of_a_kept_plan_carries_the_environment_the_generator_reads`]).
//! That a batch's answer is the single plan's is held by
//! `configure_plans_are_kept_and_asked_together`, and that a kept answer is
//! reused only while its inputs are unchanged by
//! `a_configure_asks_each_plan_once_per_input_state`, which runs `cmake`.
//!
//! A plan asked when it is declared, with no request ahead of it, is sent to
//! the generator alone: one process the configure waits for by itself. A
//! fresh configure of the test tree with the Mesh fixtures sent 15 so, every
//! one a `sce_codegen_generation` written with no request beside it. That
//! call now asks its plan ahead itself and declares at the end of its
//! directory, after the batch, and no CMake file of the tree may ask a plan
//! no request of the same file asks ahead ([`plans_sent_alone`]). Which file
//! a plan is asked from is not a list kept here: the scan reads every CMake
//! file git tracks, so this suite is one of the tree-wide gates
//! (`UNFILTERABLE_GATES` in `workflow_trigger_coverage`).
//!
//! No CMake file of the tree declares a generation by a command of its own,
//! with no exception ([`hand_declared_generations`]): a generation is one of
//! the three commands whose files a build compiles (`generate`, a document
//! set's `orchestrate`, a harness's `generate-conformance`, each of which the
//! generator plans), and a command assembled in a variable and run as
//! `COMMAND ${<var>}` is found and refused too. The projects configured on
//! their own (`backends/<c|cpp>/forge-runtime/tests/<round_trip|conformance>`)
//! and the public functions a consumer generates with
//! (`cmake/SCECodegen.cmake`) were the last to declare generations so; each
//! now includes the module and declares every generation through
//! `sce_codegen_generation` ([`their_generations_are_declared_through_the_module`]).

mod common;

use common::cmake_text::cmake_code;

/// The route under test. Compiled in, so an edit to it starts the suite that
/// judges it (`include-str-coverage` in tools/git-hooks/gate_registry.py).
const W3C_ROUTE: &str = include_str!("../../cmake/SCEStaticW3CTest.cmake");
/// The other route that generates at build time.
const INTEGRATION_ROUTE: &str = include_str!("../../cmake/SCEStaticIntegrationFixture.cmake");
/// The one module that asks the generator for a plan.
const PLAN_MODULE: &str = include_str!("../../cmake/SCECodegenPlan.cmake");
/// Where [`PLAN_MODULE`] is, as git names it.
const PLAN_MODULE_PATH: &str = "cmake/SCECodegenPlan.cmake";
/// The test tree's build, which declares every generation it runs, the Mesh
/// fixtures' among them, through `sce_codegen_generation`
/// ([`test_tree_violations`]): tests/CMakeLists.txt and the file it includes
/// for the W3C programs, which the emscripten tree without threads includes
/// alone.
const TEST_TREE: &str = concat!(
    include_str!("../../tests/CMakeLists.txt"),
    "\n",
    include_str!("../../tests/w3c/W3CPrograms.cmake")
);

/// What every line of the Mesh rule-3 override's block names: its output
/// directory, and the prefixes of the plans it asks.
const MESH_OVERRIDE: &str = "MESH_SYNTH_OVERRIDE";

/// The fragment of a hybrid `<invoke>`'s stub's name, which the test tree's
/// own documents also carry ([`test_tree_violations`]).
const HYBRID_STUB_FRAGMENT: &str = "_hybrid";

/// Fragments of the names the generator gives the files it writes. A route
/// that spells one is naming a generated file itself.
const GENERATED_NAME_FRAGMENTS: &[&str] = &[
    // A machine's header, its C++ inline part, its C translation unit.
    "_sm.",
    // The transport a run with a deploy writes beside the machine.
    "_transport.",
    // The header of a declared transition function.
    "_transition_function",
    // A child synthesized from an inline `<invoke>`.
    "__sce_synth_invoke__",
    // The stub of a hybrid `<invoke>`.
    HYBRID_STUB_FRAGMENT,
    // The source map every generation writes.
    "sce_sourcemap",
];

/// The keywords of `add_custom_command`, which end the values of the one
/// before them.
const CUSTOM_COMMAND_KEYWORDS: &[&str] = &[
    "OUTPUT",
    "COMMAND",
    "MAIN_DEPENDENCY",
    "DEPENDS",
    "BYPRODUCTS",
    "IMPLICIT_DEPENDS",
    "WORKING_DIRECTORY",
    "COMMENT",
    "DEPFILE",
    "JOB_POOL",
    "JOB_SERVER_AWARE",
    "VERBATIM",
    "APPEND",
    "USES_TERMINAL",
    "CODEGEN",
    "COMMAND_EXPAND_LISTS",
    "DEPENDS_EXPLICIT_ONLY",
];

fn is_identifier(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// One invocation of a CMake command: the line it starts on and its arguments,
/// each as written (a quoted argument keeps its quotes).
struct Invocation {
    line: usize,
    args: Vec<String>,
}

/// Every invocation of the command `name` in `code`, which carries no
/// comments. Command names are matched as CMake matches them, ignoring case,
/// and a quoted argument is never read as an invocation.
fn invocations(code: &str, name: &str) -> Vec<Invocation> {
    let chars: Vec<char> = code.chars().collect();
    let target: Vec<char> = name.chars().collect();
    let mut found = Vec::new();
    let mut quoted = false;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if quoted {
            if c == '\\' {
                i += 2;
                continue;
            }
            if c == '"' {
                quoted = false;
            }
            i += 1;
            continue;
        }
        if c == '"' {
            quoted = true;
            i += 1;
            continue;
        }
        let named = chars.len() - i >= target.len()
            && chars[i..i + target.len()]
                .iter()
                .zip(&target)
                .all(|(a, b)| a.eq_ignore_ascii_case(b));
        let bounded = i == 0 || !is_identifier(chars[i - 1]);
        if named && bounded {
            let mut open = i + target.len();
            while matches!(chars.get(open), Some(' ' | '\t')) {
                open += 1;
            }
            if chars.get(open) == Some(&'(') {
                let line = chars[..i].iter().filter(|&&ch| ch == '\n').count() + 1;
                let (args, end) = arguments(&chars, open + 1);
                found.push(Invocation { line, args });
                i = end;
                continue;
            }
        }
        i += 1;
    }
    found
}

/// The arguments of an invocation whose opening parenthesis ends just before
/// `from`, and the index just past its closing one.
fn arguments(chars: &[char], from: usize) -> (Vec<String>, usize) {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut depth = 0usize;
    let mut quoted = false;
    let mut i = from;
    while i < chars.len() {
        let c = chars[i];
        if quoted {
            current.push(c);
            if c == '\\' {
                if let Some(&escaped) = chars.get(i + 1) {
                    current.push(escaped);
                    i += 2;
                    continue;
                }
            } else if c == '"' {
                quoted = false;
            }
            i += 1;
            continue;
        }
        match c {
            '"' => {
                quoted = true;
                current.push(c);
            }
            '(' => {
                depth += 1;
                current.push(c);
            }
            ')' if depth == 0 => {
                if !current.is_empty() {
                    args.push(current);
                }
                return (args, i + 1);
            }
            ')' => {
                depth -= 1;
                current.push(c);
            }
            c if c.is_whitespace() => {
                if !current.is_empty() {
                    args.push(std::mem::take(&mut current));
                }
            }
            c => current.push(c),
        }
        i += 1;
    }
    if !current.is_empty() {
        args.push(current);
    }
    (args, i)
}

fn unquoted(arg: &str) -> &str {
    arg.strip_prefix('"')
        .and_then(|inner| inner.strip_suffix('"'))
        .unwrap_or(arg)
}

/// A value the plan helper sets, spelled as the expansion of `<prefix><suffix>`.
fn planned(prefix: &str, suffix: &str) -> String {
    format!("${{{prefix}{suffix}}}")
}

/// Whether `arg` ends the values of the keyword before it: another keyword, or
/// the expansion that carries the `BYPRODUCTS` keyword itself.
fn ends_values(arg: &str) -> bool {
    CUSTOM_COMMAND_KEYWORDS.contains(&arg) || arg.ends_with("_BYPRODUCTS_ARG}")
}

/// The values given to `keyword` in `args`, or `None` when it is not given.
fn keyword_values<'a>(args: &'a [String], keyword: &str) -> Option<&'a [String]> {
    let at = args.iter().position(|arg| arg == keyword)?;
    let rest = &args[at + 1..];
    let len = rest
        .iter()
        .position(|arg| ends_values(arg.as_str()))
        .unwrap_or(rest.len());
    Some(&rest[..len])
}

/// Every way one declared command departs from declaring what a plan answered.
fn generation_violations(command: &Invocation, prefixes: &[String]) -> Vec<String> {
    let at = format!("the command declared at line {}", command.line);
    let args = &command.args;
    if !runs_generate(command) {
        return vec![format!(
            "{at} runs no `generate`. Every file this route's build writes is a \
             generation's, declared from its plan; a document is staged when the tree \
             is configured, not by a build step"
        )];
    }

    let mut found = Vec::new();
    let output = keyword_values(args, "OUTPUT").unwrap_or_default();
    let prefix = output_plan(command).filter(|prefix| prefixes.iter().any(|asked| asked == prefix));
    let Some(prefix) = prefix else {
        return vec![format!(
            "{at} declares OUTPUT {output:?}, which is not the `<PREFIX>_OUTPUTS` of a \
             `sce_codegen_plan` call in this file ({prefixes:?})"
        )];
    };

    let depfile = planned(prefix, "_DEPFILE");
    let declared_depfile: Vec<&str> = keyword_values(args, "DEPFILE")
        .unwrap_or_default()
        .iter()
        .map(|arg| unquoted(arg.as_str()))
        .collect();
    if declared_depfile != [depfile.as_str()] {
        found.push(format!(
            "{at} declares DEPFILE {declared_depfile:?}, not the plan's {depfile}"
        ));
    }
    let written_depfile = args
        .iter()
        .position(|arg| arg == "--write-deps")
        .and_then(|flag| args.get(flag + 1))
        .map(|arg| unquoted(arg.as_str()));
    if written_depfile != Some(depfile.as_str()) {
        found.push(format!(
            "{at} asks the generator for the depfile {written_depfile:?}, not the {depfile} \
             it declares"
        ));
    }
    let byproducts = planned(prefix, "_BYPRODUCTS_ARG");
    if args.iter().any(|arg| arg == "BYPRODUCTS") {
        found.push(format!(
            "{at} lists BYPRODUCTS of its own; a generation's byproducts are the plan's \
             {byproducts}"
        ));
    }
    if !args.contains(&byproducts) {
        found.push(format!(
            "{at} does not declare {byproducts}, so the files its generation writes beside \
             the machine are declared by nothing"
        ));
    }
    found
}

/// Every way a CMake route departs from declaring only what the generator
/// plans. Empty is the route this file holds.
fn violations(text: &str) -> Vec<String> {
    let code = cmake_code(text);
    let mut found = Vec::new();

    for (index, line) in code.lines().enumerate() {
        for fragment in GENERATED_NAME_FRAGMENTS {
            if line.contains(fragment) {
                found.push(format!(
                    "line {}: `{fragment}` names a generated file by hand: {}",
                    index + 1,
                    line.trim()
                ));
            }
        }
    }
    for file in invocations(&code, "file") {
        if file
            .args
            .first()
            .is_some_and(|mode| mode.starts_with("GLOB"))
        {
            found.push(format!(
                "line {}: a fixture's files are found by a name pattern; the files a \
                 generation reads and writes are the plan's answer",
                file.line
            ));
        }
    }
    for process in invocations(&code, "execute_process") {
        found.push(format!(
            "line {}: the route runs a process when the tree is configured; it asks the \
             generator only through `sce_codegen_plan`",
            process.line
        ));
    }

    let prefixes: Vec<String> = invocations(&code, "sce_codegen_plan")
        .iter()
        .filter_map(|plan| plan.args.first().cloned())
        .collect();
    if prefixes.is_empty() {
        found.push("the route never asks `sce_codegen_plan` for a plan".to_string());
    }
    let commands = invocations(&code, "add_custom_command");
    if commands.is_empty() {
        found.push(
            "the route declares no command at all: the scan read nothing, or the route \
             no longer generates"
                .to_string(),
        );
    }
    for command in &commands {
        found.extend(generation_violations(command, &prefixes));
    }
    found
}

/// The plan expansions a route reads to name a child document itself: the
/// names a plan sets of the child documents its run writes or stages
/// (cmake/SCECodegenPlan.cmake). A route that reads them builds a child's
/// path from its name rather than declaring the generation the plan holds.
const CHILD_NAME_LISTS: &[&str] = &["_CHILDREN}", "_SYNTH}"];

/// Every way a route departs from declaring each child generation from the
/// plan its parent's answer holds. Empty is the route this file holds.
fn child_generation_violations(text: &str) -> Vec<String> {
    let code = cmake_code(text);
    let plans = invocations(&code, "sce_codegen_plan");
    let commands = invocations(&code, "add_custom_command");
    let mut found = child_rules(&plans, &commands);
    for (index, line) in code.lines().enumerate() {
        found.extend(child_name_lists_read(index, line));
    }
    found
}

/// The child document names a plan sets, read on `line` (the `index`th of
/// the code) to name a child document by hand.
fn child_name_lists_read(index: usize, line: &str) -> Vec<String> {
    CHILD_NAME_LISTS
        .iter()
        .filter(|list| line.contains(**list))
        .map(|list| {
            format!(
                "line {}: `{list}` reads the names of the child documents a plan names, to \
                 name a child document by hand: {}",
                index + 1,
                line.trim()
            )
        })
        .collect()
}

/// The prefix of the plan a command declares its OUTPUT from, when it
/// declares it as one plan's `<PREFIX>_OUTPUTS`.
fn output_plan(command: &Invocation) -> Option<&str> {
    match keyword_values(&command.args, "OUTPUT")? {
        [only] => only
            .strip_prefix("${")
            .and_then(|rest| rest.strip_suffix("_OUTPUTS}")),
        _ => None,
    }
}

/// The arguments a child's generation is run with that the plan states for
/// it, which a command spelling them itself would state a second time.
const PLANNED_CHILD_ARGUMENTS: &[&str] = &["--as-child", "--deploy"];

/// Every way the plans and the commands of one route depart from declaring
/// each child generation from the plan its parent's answer holds, and from
/// running it with the arguments that plan states (`<P>_ARGS`).
fn child_rules(plans: &[Invocation], commands: &[Invocation]) -> Vec<String> {
    let mut found = Vec::new();
    let mut read_from_a_plan = 0;
    let mut from_prefixes: Vec<&str> = Vec::new();
    for plan in plans {
        if plan.args.iter().any(|arg| arg == "FROM") {
            read_from_a_plan += 1;
            from_prefixes.extend(plan.args.first().map(String::as_str));
        } else if !plan.args.iter().any(|arg| arg == "CHILD_GENERATIONS") {
            found.push(format!(
                "line {}: a plan that does not ask for the generations of the child documents \
                 its run writes (CHILD_GENERATIONS), so a child's generation is declared from \
                 something other than the generator's answer",
                plan.line
            ));
        }
    }
    if read_from_a_plan == 0 {
        found.push(
            "no child generation is declared from a plan read FROM its parent's answer \
             (`sce_codegen_plan(<P> FROM <plan> CHILD <name>)`)"
                .to_string(),
        );
    }
    for command in commands {
        let Some(prefix) = output_plan(command).filter(|prefix| from_prefixes.contains(prefix))
        else {
            continue;
        };
        let arguments = planned(prefix, "_ARGS");
        if !command.args.contains(&arguments) {
            found.push(format!(
                "the command declared at line {} runs a child's generation without the \
                 arguments its plan states for it ({arguments}), so it runs a generation \
                 other than the one that was planned",
                command.line
            ));
        }
        for spelled in PLANNED_CHILD_ARGUMENTS {
            if command.args.iter().any(|arg| arg == spelled) {
                found.push(format!(
                    "the command declared at line {} spells `{spelled}` itself; which \
                     generation a child has is its plan's to state ({arguments})",
                    command.line
                ));
            }
        }
    }
    found
}

#[test]
fn neither_route_names_a_child_document_by_hand() {
    for (route, text) in [
        ("cmake/SCEStaticW3CTest.cmake", W3C_ROUTE),
        ("cmake/SCEStaticIntegrationFixture.cmake", INTEGRATION_ROUTE),
    ] {
        let found = child_generation_violations(text);
        assert!(
            found.is_empty(),
            "{route} declares a child generation that is not the one its parent's plan \
             holds:\n  {}\n\nAsk the plan with CHILD_GENERATIONS and declare each child from \
             `sce_codegen_plan(<P> FROM <plan> CHILD <name>)`.",
            found.join("\n  ")
        );
    }
}

/// The conformance route asks one plan of a fixture, for the fixture and the
/// children its generation writes, and declares each child's generation from
/// it: one plan asked of the generator, one read FROM it.
#[test]
fn the_w3c_route_declares_each_child_generation_from_the_fixtures_plan() {
    let code = cmake_code(W3C_ROUTE);
    let plans = invocations(&code, "sce_codegen_plan");
    let asks_for_children = plans
        .iter()
        .filter(|plan| plan.args.iter().any(|arg| arg == "CHILD_GENERATIONS"))
        .count();
    let read_from_a_plan = plans
        .iter()
        .filter(|plan| plan.args.iter().any(|arg| arg == "FROM"))
        .count();
    assert_eq!(
        (asks_for_children, read_from_a_plan),
        (1, 1),
        "the conformance route asks the fixture's plan for its children's generations and \
         declares each from it"
    );
}

/// A child planned by its own name in the fixture's directory, which is what
/// the conformance route did while it found a hybrid stub only by tracking a
/// copy of it there, is refused.
#[test]
fn a_child_planned_by_its_name_is_refused() {
    let by_name = format!(
        "{PLANNED_GENERATION}\nforeach(C ${{_PLAN_CHILDREN}})\n\
         sce_codegen_plan(_CHILD STEM \"${{C}}\" DOCUMENT \"${{R}}/${{C}}.scxml\" \
         OUTPUT_DIR \"${{O}}\" LANGUAGE cpp)\n\
         add_custom_command(\n\
             OUTPUT ${{_CHILD_OUTPUTS}}\n\
             COMMAND \"${{SCE_CODEGEN}}\" generate \"${{O}}/${{C}}.scxml\" \
         --write-deps \"${{_CHILD_DEPFILE}}\"\n\
             DEPFILE \"${{_CHILD_DEPFILE}}\"\n\
             ${{_CHILD_BYPRODUCTS_ARG}}\n\
             VERBATIM\n\
         )\nendforeach()\n"
    );
    assert_eq!(
        violations(&by_name),
        Vec::<String>::new(),
        "the shape is a planned generation, which the declaration rules alone accept"
    );
    let found = child_generation_violations(&by_name);
    assert!(
        found.iter().any(|v| v.contains("`_CHILDREN}`")),
        "a route reading the plan's child names was accepted: {found:?}"
    );
    assert!(
        found.iter().any(|v| v.contains("CHILD_GENERATIONS")),
        "a plan that does not ask for its children's generations was accepted: {found:?}"
    );
    assert!(
        found.iter().any(|v| v.contains("FROM its parent's answer")),
        "a route with no child generation read FROM a plan was accepted: {found:?}"
    );
}

/// The shape both routes use is accepted by the child-generation rules.
#[test]
fn a_child_generation_read_from_its_parents_plan_names_no_child_by_hand() {
    let from_the_plan = r#"
sce_codegen_plan(_PLAN STEM "${M}" DOCUMENT "${D}" OUTPUT_DIR "${O}" LANGUAGE cpp CHILD_GENERATIONS)
foreach(C ${_PLAN_GENERATIONS})
    sce_codegen_plan(_CHILD_PLAN FROM _PLAN CHILD "${C}")
endforeach()
"#;
    assert_eq!(
        child_generation_violations(from_the_plan),
        Vec::<String>::new()
    );
}

#[test]
fn the_w3c_route_declares_only_what_the_generator_plans() {
    let found = violations(W3C_ROUTE);
    assert!(
        found.is_empty(),
        "cmake/SCEStaticW3CTest.cmake declares a generated file the generator was not \
         asked about:\n  {}\n\nDeclare each generation's OUTPUT, DEPFILE and byproducts as \
         the variables `sce_codegen_plan` sets (cmake/SCECodegenPlan.cmake).",
        found.join("\n  ")
    );
}

#[test]
fn the_integration_route_declares_only_what_the_generator_plans() {
    let found = violations(INTEGRATION_ROUTE);
    assert!(
        found.is_empty(),
        "cmake/SCEStaticIntegrationFixture.cmake declares a generated file the generator \
         was not asked about:\n  {}\n\nDeclare each generation's OUTPUT, DEPFILE and \
         byproducts as the variables `sce_codegen_plan` sets (cmake/SCECodegenPlan.cmake), \
         a child document's generation through `sce_codegen_plan(<P> FROM <plan> CHILD \
         <name>)`.",
        found.join("\n  ")
    );
}

/// The integration route declares the generation of every child document the
/// parent's generation writes from the plan the parent's answer holds for it,
/// not from a name it builds: it asks for the children's plans, and each child
/// command is declared from a plan read FROM that answer.
#[test]
fn the_integration_route_declares_each_child_generation_from_the_parents_plan() {
    let code = cmake_code(INTEGRATION_ROUTE);
    let plans = invocations(&code, "sce_codegen_plan");
    let asks_for_children = plans
        .iter()
        .filter(|plan| plan.args.iter().any(|arg| arg == "CHILD_GENERATIONS"))
        .count();
    let read_from_a_plan = plans
        .iter()
        .filter(|plan| plan.args.iter().any(|arg| arg == "FROM"))
        .count();
    assert_eq!(
        asks_for_children, 2,
        "both of the integration route's generators (C++ and C11) ask for the plans of \
         the child documents their parent writes"
    );
    assert_eq!(
        read_from_a_plan, 2,
        "both declare each child's generation from the plan the parent's answer holds"
    );
}

/// The fragments of a generated file's name the test tree's build may not
/// spell anywhere in its code: [`GENERATED_NAME_FRAGMENTS`] less the hybrid
/// stub's ([`HYBRID_STUB_FRAGMENT`]), which the static integration fixtures'
/// own documents carry in their names (`static_invoke_hybrid`). Every file a
/// generation of the tree writes is the plan's to name, and a target compiles
/// the headers its generation's plan sets (`<PREFIX>_HEADERS`).
fn test_tree_name_fragments() -> impl Iterator<Item = &'static str> {
    GENERATED_NAME_FRAGMENTS
        .iter()
        .copied()
        .filter(|fragment| *fragment != HYBRID_STUB_FRAGMENT)
}

/// The lines of the test tree's code that spell a generated name and are no
/// declaration of a generation of the tree, each with why: the marker the
/// line holds, and the reason.
const GENERATED_NAMES_NOT_OF_THIS_TREE: &[(&str, &str)] = &[(
    "set(SCE_SCENARIO_HEADER",
    "the header a configured scenario unit includes; the machine is generated by \
     `sce_add_state_machine` (cmake/SCECodegen.cmake), which declares it from its plan",
)];

/// The build steps of the test tree that are no generation, each with why:
/// the marker one of the step's arguments holds, and the reason. Every other
/// step the tree declares is a generation, and is declared by
/// `sce_codegen_generation`.
const BUILD_STEPS_NOT_GENERATIONS: &[(&str, &str)] = &[
    (
        "generate_test_list.py",
        "renders the visualizer's test list from the conformance registry",
    ),
    (
        "standalone_http_server.js",
        "copies a source file beside the built W3C test binary",
    ),
    (
        "check-aot-briefs",
        "checks the AOT test headers' briefs and writes a stamp",
    ),
    (
        "visualizer.wasm",
        "deploys the built visualizer to web/visualizer/",
    ),
];

/// The subcommands of the generator whose files a build compiles: one
/// document (`generate`), a document set compiled together (`orchestrate`)
/// and a conformance harness (`generate-conformance`). A command that runs one
/// of them is a generation, and each has a plan (`sce-codegen --plan`).
const GENERATING_SUBCOMMANDS: &[&str] = &["generate", "orchestrate", "generate-conformance"];

/// Whether `arg` is the command a plan names for its generation,
/// `${<PREFIX>_RUN}`: `sce_codegen_plan` sets it to one of
/// [`GENERATING_SUBCOMMANDS`] and what that reads.
fn is_planned_run(arg: &str) -> bool {
    arg.starts_with("${") && arg.ends_with("_RUN}")
}

/// Whether `command` runs a generation: the program, then a generating
/// subcommand or the run its plan names.
fn runs_generate(command: &Invocation) -> bool {
    command.args.windows(2).any(|pair| {
        pair[0].contains("SCE_CODEGEN}")
            && (GENERATING_SUBCOMMANDS.contains(&pair[1].as_str()) || is_planned_run(&pair[1]))
    })
}

/// Every way the test tree's build departs from declaring each generation it
/// runs from the generator's plan of it. Empty is the build this file holds.
///
/// * a command of its own that runs `generate` is refused: a generation is
///   declared by `sce_codegen_generation` (cmake/SCECodegenPlan.cmake), whose
///   one command is held to its plan by
///   [`the_generation_command_is_declared_from_its_plan`];
/// * every other build step is one [`BUILD_STEPS_NOT_GENERATIONS`] names;
/// * no line spells a fragment of a generated name
///   ([`test_tree_name_fragments`]) but the ones
///   [`GENERATED_NAMES_NOT_OF_THIS_TREE`] names;
/// * no generation is split into a machine and a transport-only run: one run
///   with the deploy writes both, and declares both from its plan;
/// * the tree asks for a plan only through `sce_codegen_generation`.
fn test_tree_violations(text: &str) -> Vec<String> {
    let code = cmake_code(text);
    let mut found = Vec::new();

    for command in invocations(&code, "add_custom_command") {
        if runs_generate(&command) {
            found.push(format!(
                "the command declared at line {} runs `generate` itself; a generation of the \
                 test tree is declared by `sce_codegen_generation`, from its plan",
                command.line
            ));
            continue;
        }
        let said = BUILD_STEPS_NOT_GENERATIONS
            .iter()
            .any(|(marker, _)| command.args.iter().any(|arg| arg.contains(marker)));
        if !said {
            found.push(format!(
                "the command declared at line {} is neither a generation nor a step this \
                 test says why it is not one (BUILD_STEPS_NOT_GENERATIONS)",
                command.line
            ));
        }
    }

    for (index, line) in code.lines().enumerate() {
        let exempt = GENERATED_NAMES_NOT_OF_THIS_TREE
            .iter()
            .any(|(marker, _)| line.contains(marker));
        for fragment in test_tree_name_fragments() {
            if line.contains(fragment) && !exempt {
                found.push(format!(
                    "line {}: `{fragment}` names a generated file by hand: {}",
                    index + 1,
                    line.trim()
                ));
            }
        }
        if line.contains("--transport-only") {
            found.push(format!(
                "line {}: a generation split into a transport-only run; one run with the \
                 deploy writes the machine and its transport: {}",
                index + 1,
                line.trim()
            ));
        }
    }

    for plan in invocations(&code, "sce_codegen_plan") {
        found.push(format!(
            "line {}: the test tree asks for a plan of its own; it declares a generation, \
             and asks for its plan, through `sce_codegen_generation`",
            plan.line
        ));
    }
    if invocations(&code, "sce_codegen_generation").is_empty() {
        found.push(
            "the test tree declares no generation: the scan read nothing, or the tree no \
             longer generates"
                .to_string(),
        );
    }
    found
}

/// Every generation the test tree runs is declared from its plan: no command
/// of its own runs `generate`, no generated name is spelled, and no
/// generation is split into a machine and a transport-only run.
#[test]
fn the_test_tree_declares_every_generation_from_its_plan() {
    let found = test_tree_violations(TEST_TREE);
    assert!(
        found.is_empty(),
        "tests/CMakeLists.txt declares a generation that is not its plan's:\n  {}\n\n\
         Declare it with `sce_codegen_generation` (cmake/SCECodegenPlan.cmake) and compile \
         the `<PREFIX>_HEADERS` it sets.",
        found.join("\n  ")
    );
}

/// The one command that runs a generation for the test tree is the module's,
/// and it declares its OUTPUT, DEPFILE and byproducts from the plan it asked
/// and runs the generation with the arguments the plan states.
#[test]
fn the_generation_command_is_declared_from_its_plan() {
    let code = cmake_code(PLAN_MODULE);
    let prefixes: Vec<String> = invocations(&code, "sce_codegen_plan")
        .iter()
        .filter_map(|plan| plan.args.first().cloned())
        .collect();
    let commands = invocations(&code, "add_custom_command");
    assert_eq!(
        commands.len(),
        1,
        "cmake/SCECodegenPlan.cmake declares one command, the generation \
         `sce_codegen_generation` declares"
    );
    let command = &commands[0];
    let mut found = generation_violations(command, &prefixes);
    if let Some(prefix) = output_plan(command) {
        let arguments = planned(prefix, "_ARGS");
        if !command.args.contains(&arguments) {
            found.push(format!(
                "the generation is not run with the arguments its plan states ({arguments})"
            ));
        }
        // The command and what it reads are the plan's too: a document, a set
        // compiled together or a harness, each the run its plan was asked of.
        let run = planned(prefix, "_RUN");
        if !command.args.contains(&run) {
            found.push(format!(
                "the generation does not run the command its plan names ({run})"
            ));
        }
    }
    for spelled in PLANNED_CHILD_ARGUMENTS {
        if command.args.iter().any(|arg| arg == spelled) {
            found.push(format!(
                "the generation spells `{spelled}` itself; its plan states its arguments"
            ));
        }
    }
    assert!(
        found.is_empty(),
        "cmake/SCECodegenPlan.cmake declares the generation other than from its plan:\n  {}",
        found.join("\n  ")
    );
}

/// The Mesh rule-3 override declares both of its generations, the parent's
/// and the one of the partition the deploy places its in-line child on, from
/// the plan of the parent's document: one call asks the parent's plan with
/// CHILD_GENERATIONS, and the module declares the child's generation the
/// plan holds, read FROM it, with nothing of its own stated.
#[test]
fn the_mesh_override_declares_the_partitions_generation_from_the_parents_plan() {
    let code = cmake_code(TEST_TREE);
    let generations: Vec<Invocation> = invocations(&code, "sce_codegen_generation")
        .into_iter()
        .filter(|generation| {
            generation
                .args
                .first()
                .is_some_and(|prefix| prefix.contains(MESH_OVERRIDE))
        })
        .collect();
    assert_eq!(
        generations.len(),
        1,
        "the Mesh rule-3 override declares its two generations by one call, the parent's"
    );
    assert!(
        generations[0]
            .args
            .iter()
            .any(|arg| arg == "CHILD_GENERATIONS"),
        "the Mesh rule-3 override does not ask the parent's plan for its children's \
         generations, so the partition's has no command"
    );
    for stated in ["FROM", "CHILD", "--as-child"] {
        assert!(
            !generations[0].args.iter().any(|arg| arg == stated),
            "the Mesh rule-3 override states `{stated}`; the partition's generation is the \
             plan's to state"
        );
    }
    let module = cmake_code(PLAN_MODULE);
    let declared = in_order(
        function_body(&module, "_sce_codegen_generations_declare"),
        &[
            "_sce_codegen_generation_declare(_RUN ",
            "IN LISTS _RUN_GENERATIONS",
            "_sce_codegen_generation_declare(_CHILD_RUN ",
            "FROM _RUN CHILD",
        ],
    );
    assert!(
        declared.is_ok(),
        "cmake/SCECodegenPlan.cmake does not declare each generation a CHILD_GENERATIONS \
         plan holds from that plan: {declared:?}"
    );
}

/// The Mesh rule-3 override's former stage 3, a generation of the child's
/// partition with its outputs, depfile and deploy spelled by hand, is
/// refused, as is any generation the test tree declares by a command of its
/// own.
#[test]
fn a_hand_declared_generation_is_refused() {
    let hand = r#"
sce_codegen_generation(_MESH_SYNTH_OVERRIDE_PARENT DOCUMENT "${D}/p.scxml" OUTPUT_DIR "${D}" LANGUAGE cpp)
add_custom_command(
    OUTPUT "${MESH_SYNTH_OVERRIDE_OUTPUT_DIR}/p__sce_synth_invoke__r_sm.h"
    COMMAND "${SCE_CODEGEN}" generate "${MESH_SYNTH_OVERRIDE_OUTPUT_DIR}/p__sce_synth_invoke__r.scxml"
            --deploy "${MESH_SYNTH_OVERRIDE_OUTPUT_DIR}/d.yaml"
            --write-deps "${MESH_SYNTH_OVERRIDE_OUTPUT_DIR}/p__sce_synth_invoke__r_sm.h.d"
    DEPENDS "${SCE_CODEGEN}"
    DEPFILE "${MESH_SYNTH_OVERRIDE_OUTPUT_DIR}/p__sce_synth_invoke__r_sm.h.d"
    VERBATIM
)
"#;
    let found = test_tree_violations(hand);
    assert!(
        found.iter().any(|v| v.contains("runs `generate` itself")),
        "a generation declared by a command of the tree's own was accepted: {found:?}"
    );
    assert!(
        found.iter().any(|v| v.contains("`__sce_synth_invoke__`")),
        "the child's machine spelled by hand was not seen: {found:?}"
    );
    assert!(
        found.iter().any(|v| v.contains("`_sm.`")),
        "a machine's name spelled by hand was not seen: {found:?}"
    );
}

/// A generation split into a machine run and a transport-only run, the
/// shape the test tree used to declare a Mesh fixture with, is refused, as
/// is a header list a target spells.
#[test]
fn a_split_transport_generation_is_refused() {
    let split = r#"
sce_codegen_generation(_B DOCUMENT "${M}/brake.scxml" OUTPUT_DIR "${O}" LANGUAGE cpp DEPLOY "${M}/d.yaml")
sce_codegen_generation(_T DOCUMENT "${M}/brake.scxml" OUTPUT_DIR "${O}" LANGUAGE cpp DEPLOY "${M}/d.yaml" ARGS --transport-only)
add_executable(t t.cpp "${O}/brake_transport.h")
"#;
    let found = test_tree_violations(split);
    assert!(
        found.iter().any(|v| v.contains("transport-only run")),
        "a transport-only generation was accepted: {found:?}"
    );
    assert!(
        found.iter().any(|v| v.contains("`_transport.`")),
        "a transport header spelled by hand was not seen: {found:?}"
    );
}

/// A build step that is neither a generation nor one the test says why it is
/// not is refused, and one it names passes; a generation declared through the
/// module, its headers compiled from the plan, passes.
#[test]
fn every_build_step_of_the_test_tree_is_a_generation_or_says_why_not() {
    let declared = r#"
sce_codegen_generation(_B DOCUMENT "${M}/brake.scxml" OUTPUT_DIR "${O}" LANGUAGE cpp DEPLOY "${M}/d.yaml")
add_executable(t t.cpp ${_B_HEADERS})
add_custom_command(OUTPUT "${S}" COMMAND "${SCE_CODEGEN}" check-aot-briefs --manifest "${R}" VERBATIM)
set(SCE_SCENARIO_HEADER "${_machine}_sm.h")
"#;
    assert_eq!(test_tree_violations(declared), Vec::<String>::new());

    let staged = format!(
        "{declared}add_custom_command(OUTPUT \"${{O}}/x.scxml\" COMMAND ${{CMAKE_COMMAND}} -E \
         copy a b VERBATIM)\nset(OTHER \"${{m}}_sm.h\")\n"
    );
    let found = test_tree_violations(&staged);
    assert!(
        found.iter().any(|v| v.contains("neither a generation")),
        "a build step with no reason was accepted: {found:?}"
    );
    assert!(
        found.iter().any(|v| v.contains("`_sm.`")),
        "a machine's name outside the exempt line was accepted: {found:?}"
    );
}

/// A child's generation declared from the plan its parent's answer holds is
/// the shape the integration route uses, and passes.
#[test]
fn a_child_generation_declared_from_its_parents_plan_passes() {
    let child = format!(
        "{PLANNED_GENERATION}\nforeach(C ${{_PLAN_GENERATIONS}})\n\
         sce_codegen_plan(_CHILD_PLAN FROM _PLAN CHILD \"${{C}}\")\n\
         add_custom_command(\n\
             OUTPUT ${{_CHILD_PLAN_OUTPUTS}}\n\
             COMMAND \"${{SCE_CODEGEN}}\" generate \"${{_CHILD_PLAN_DOCUMENT}}\" \
         ${{_CHILD_PLAN_ARGS}} --write-deps \"${{_CHILD_PLAN_DEPFILE}}\"\n\
             DEPENDS ${{_CHILD_PLAN_DEPENDS}} \"${{SCE_CODEGEN}}\"\n\
             DEPFILE \"${{_CHILD_PLAN_DEPFILE}}\"\n\
             ${{_CHILD_PLAN_BYPRODUCTS_ARG}}\n\
             VERBATIM\n\
         )\nendforeach()\n"
    );
    assert_eq!(violations(&child), Vec::<String>::new());
}

/// A child's generation run with the arguments its plan states, and with
/// none of its own, is the shape both routes and the mesh rule-3 override
/// use; given `--as-child` by hand and not the plan's arguments, it is
/// refused: which generation a child has (in its parent's process, or on a
/// partition of its own with the deploy) is the plan's to say.
#[test]
fn a_child_generation_is_run_with_the_arguments_its_plan_states() {
    let declared = |arguments: &str| {
        format!(
            "sce_codegen_plan(_PLAN STEM \"${{M}}\" DOCUMENT \"${{D}}\" OUTPUT_DIR \"${{O}}\" \
             LANGUAGE cpp CHILD_GENERATIONS)\n\
             foreach(C ${{_PLAN_GENERATIONS}})\n\
             sce_codegen_plan(_CHILD_PLAN FROM _PLAN CHILD \"${{C}}\")\n\
             add_custom_command(\n\
                 OUTPUT ${{_CHILD_PLAN_OUTPUTS}}\n\
                 COMMAND \"${{SCE_CODEGEN}}\" generate \"${{_CHILD_PLAN_DOCUMENT}}\" {arguments} \
             --write-deps \"${{_CHILD_PLAN_DEPFILE}}\"\n\
                 DEPENDS ${{_CHILD_PLAN_DEPENDS}} \"${{SCE_CODEGEN}}\"\n\
                 DEPFILE \"${{_CHILD_PLAN_DEPFILE}}\"\n\
                 ${{_CHILD_PLAN_BYPRODUCTS_ARG}}\n\
                 VERBATIM\n\
             )\nendforeach()\n"
        )
    };
    assert_eq!(
        child_generation_violations(&declared("${_CHILD_PLAN_ARGS}")),
        Vec::<String>::new()
    );
    let by_hand = child_generation_violations(&declared("--as-child"));
    assert!(
        by_hand
            .iter()
            .any(|v| v.contains("${_CHILD_PLAN_ARGS}") && v.contains("without the arguments")),
        "a child generation without its plan's arguments was accepted: {by_hand:?}"
    );
    assert!(
        by_hand.iter().any(|v| v.contains("spells `--as-child`")),
        "a child generation spelling its own arguments was accepted: {by_hand:?}"
    );
    let both = child_generation_violations(&declared("${_CHILD_PLAN_ARGS} --deploy \"${Y}\""));
    assert!(
        both.iter().any(|v| v.contains("spells `--deploy`")),
        "a child generation adding a deploy of its own was accepted: {both:?}"
    );
}

/// A child's machine spelled from its name, which is what the integration
/// route did before the generator planned a child's generation, is refused.
#[test]
fn a_childs_machine_spelled_from_its_name_is_refused() {
    let hand = format!(
        "{PLANNED_GENERATION}\nforeach(C ${{_PLAN_CHILDREN}})\n\
         add_custom_command(\n\
             OUTPUT \"${{O}}/${{C}}_sm.h\"\n\
             COMMAND \"${{SCE_CODEGEN}}\" generate \"${{O}}/${{C}}.scxml\" --as-child \
         --write-deps \"${{O}}/${{C}}_sm.h.d\"\n\
             DEPENDS \"${{SCE_CODEGEN}}\"\n\
             DEPFILE \"${{O}}/${{C}}_sm.h.d\"\n\
             BYPRODUCTS \"${{O}}/${{C}}_sm.inl\"\n\
             VERBATIM\n\
         )\nendforeach()\n"
    );
    let found = violations(&hand);
    assert!(
        found.iter().any(|v| v.contains("`_sm.`")),
        "a child's machine spelled by hand was not seen: {found:?}"
    );
    assert!(
        found.iter().any(|v| v.contains("declares OUTPUT")),
        "a child's OUTPUT no plan set was accepted: {found:?}"
    );
}

#[test]
fn one_module_asks_the_generator_for_a_plan() {
    let module = cmake_code(PLAN_MODULE);
    assert_eq!(
        invocations(&module, "function")
            .iter()
            .filter(|function| function.args.first().map(String::as_str)
                == Some("sce_codegen_plan"))
            .count(),
        1,
        "cmake/SCECodegenPlan.cmake defines `sce_codegen_plan` once"
    );
    for helper in ["sce_codegen_plan_request", "sce_codegen_plan_prefetch"] {
        assert_eq!(
            invocations(&module, "function")
                .iter()
                .filter(|function| function.args.first().map(String::as_str) == Some(helper))
                .count(),
            1,
            "cmake/SCECodegenPlan.cmake defines `{helper}` once"
        );
    }
    // Every plan, asked now or ahead, is answered by the one batch the module
    // runs, and each request of it is a `--plan generate` run.
    let processes: Vec<Invocation> = invocations(&module, "execute_process");
    assert_eq!(
        processes.len(),
        1,
        "cmake/SCECodegenPlan.cmake runs the generator in more than one place; every plan \
         is answered by `sce-codegen plan-batch`, through `_sce_codegen_plan_ask`"
    );
    assert!(
        processes[0].args.iter().any(|arg| arg == "plan-batch"),
        "cmake/SCECodegenPlan.cmake no longer runs `sce-codegen plan-batch`"
    );
    assert!(
        invocations(&module, "set")
            .iter()
            .any(|set| set.args.iter().any(|arg| arg == "--plan")),
        "cmake/SCECodegenPlan.cmake no longer asks the batch for `--plan generate` runs"
    );

    for (route, text) in [
        ("cmake/SCEStaticW3CTest.cmake", W3C_ROUTE),
        ("cmake/SCEStaticIntegrationFixture.cmake", INTEGRATION_ROUTE),
    ] {
        let code = cmake_code(text);
        assert!(
            invocations(&code, "include").iter().any(|include| include
                .args
                .iter()
                .any(|arg| arg.ends_with("/SCECodegenPlan.cmake"))),
            "{route} does not include cmake/SCECodegenPlan.cmake"
        );
        assert!(
            !invocations(&code, "sce_codegen_plan").is_empty(),
            "{route} never asks `sce_codegen_plan` for a plan"
        );
        let own: Vec<usize> = invocations(&code, "execute_process")
            .iter()
            .map(|process| process.line)
            .collect();
        assert!(
            own.is_empty(),
            "{route} runs a process of its own at line(s) {own:?}: a second place that asks \
             the generator is a second answer to keep in step"
        );
    }
}

/// Every way a route's requests asked ahead (`sce_codegen_plan_request`)
/// depart from the plans it asks: a request whose options are not, word for
/// word, those of a plan the route asks (`sce_codegen_plan`, not FROM another)
/// is an answer nothing reads, and the plan it meant is asked again alone.
fn ahead_violations(text: &str) -> Vec<String> {
    let code = cmake_code(text);
    let asked: Vec<Vec<String>> = invocations(&code, "sce_codegen_plan")
        .into_iter()
        .filter(|plan| !plan.args.iter().any(|arg| arg == "FROM"))
        .map(|plan| plan.args.get(1..).unwrap_or_default().to_vec())
        .collect();
    let ahead = invocations(&code, "sce_codegen_plan_request");
    let mut found = Vec::new();
    if ahead.is_empty() {
        found.push("the route asks no plan ahead".to_string());
    }
    for request in &ahead {
        if !asked.contains(&request.args) {
            found.push(format!(
                "line {}: the request asked ahead ({:?}) is no plan the route asks \
                 ({asked:?}), so its answer is never read",
                request.line, request.args
            ));
        }
    }
    found
}

/// The conformance route asks every fixture's plan ahead, in one batch, with
/// the options it then asks it with one fixture at a time.
#[test]
fn the_w3c_route_asks_ahead_for_the_plan_it_declares() {
    let found = ahead_violations(W3C_ROUTE);
    assert!(
        found.is_empty(),
        "cmake/SCEStaticW3CTest.cmake asks ahead for a plan it does not ask:\n  {}\n\nGive \
         `sce_codegen_plan_request` the options `_sce_generate_w3c_fixture` gives \
         `sce_codegen_plan`.",
        found.join("\n  ")
    );
}

#[test]
fn a_request_ahead_that_is_not_the_plan_is_refused() {
    let request = |language: &str| {
        format!(
            "{PLANNED_GENERATION}\nsce_codegen_plan_request(STEM \"${{M}}\" DOCUMENT \"${{D}}\" \
             OUTPUT_DIR \"${{O}}\" LANGUAGE {language})\n"
        )
    };
    assert_eq!(ahead_violations(&request("cpp")), Vec::<String>::new());
    let other = ahead_violations(&request("c11"));
    assert!(
        other
            .iter()
            .any(|v| v.contains("is no plan the route asks")),
        "a request asked ahead with options no plan has was accepted: {other:?}"
    );
    let unasked = unrequested_plans(&request("c11"));
    assert!(
        unasked
            .iter()
            .any(|v| v.contains("is asked ahead by no request")),
        "a plan no request asks ahead was accepted: {unasked:?}"
    );
}

/// Every way a route's plans depart from being asked ahead: a root plan
/// (`sce_codegen_plan`, not FROM another) whose options no
/// `sce_codegen_plan_request` of the route spells, word for word, is asked of
/// the generator by itself when it is declared, one process for one plan.
fn unrequested_plans(text: &str) -> Vec<String> {
    let code = cmake_code(text);
    let ahead: Vec<Vec<String>> = invocations(&code, "sce_codegen_plan_request")
        .into_iter()
        .map(|request| request.args)
        .collect();
    invocations(&code, "sce_codegen_plan")
        .into_iter()
        .filter(|plan| !plan.args.iter().any(|arg| arg == "FROM"))
        .filter(|plan| !ahead.contains(&plan.args.get(1..).unwrap_or_default().to_vec()))
        .map(|plan| {
            format!(
                "line {}: the plan {:?} is asked ahead by no request, so it is asked of the \
                 generator alone when it is declared",
                plan.line, plan.args
            )
        })
        .collect()
}

/// Both build-time routes ask every plan they declare ahead, with the options
/// they then declare it with: the integration route's C++ and C11 fixtures
/// alike, and the conformance route's fixtures. So does the module for every
/// generation `sce_codegen_generation` declares: the call asks with the one
/// list the declaration then plans with.
#[test]
fn every_plan_a_route_declares_is_asked_ahead() {
    for (route, text) in [
        ("cmake/SCEStaticW3CTest.cmake", W3C_ROUTE),
        ("cmake/SCEStaticIntegrationFixture.cmake", INTEGRATION_ROUTE),
        (PLAN_MODULE_PATH, PLAN_MODULE),
    ] {
        let found: Vec<String> = ahead_violations(text)
            .into_iter()
            .chain(unrequested_plans(text))
            .collect();
        assert!(
            found.is_empty(),
            "{route} declares a plan it does not ask ahead, or asks ahead for one it does \
             not declare:\n  {}",
            found.join("\n  ")
        );
    }
}

/// The floor of tracked CMake files the tree-wide scan reads (`git ls-files`
/// named 51 outside the vendored trees on 2026-11-04), so a pathspec that
/// stopped matching reads as a broken scan and not as a clean tree.
const MIN_CMAKE_FILES: usize = 30;

/// The files the tree-wide scan has to have read: the module and the two
/// routes, which ask plans, the two test trees, which declare generations
/// through the module, and the files that declare every generation of theirs
/// through it besides ([`DECLARING_THROUGH_THE_MODULE`]).
const FILES_THAT_ASK_A_PLAN: &[&str] = &[
    PLAN_MODULE_PATH,
    "cmake/SCEStaticW3CTest.cmake",
    "cmake/SCEStaticIntegrationFixture.cmake",
    "tests/CMakeLists.txt",
    "tests/w3c/W3CPrograms.cmake",
    "backends/c/tests/CMakeLists.txt",
    "cmake/SCECodegen.cmake",
    "backends/c/forge-runtime/tests/round_trip/CMakeLists.txt",
    "backends/cpp/forge-runtime/tests/round_trip/CMakeLists.txt",
    "backends/c/forge-runtime/tests/conformance/CMakeLists.txt",
    "backends/cpp/forge-runtime/tests/conformance/CMakeLists.txt",
];

/// Every CMake file git tracks, with its text, the vendored trees left out:
/// a plan can be asked from a directory that has no CMake file today, so the
/// files are found where git says they are rather than named.
fn tracked_cmake_files() -> Vec<(String, String)> {
    let root = common::repository::root();
    let files: Vec<(String, String)> =
        common::repository::paths_git_tracks(&["*CMakeLists.txt", "*.cmake"])
            .into_iter()
            .filter(|path| !path.starts_with("third_party/") && !path.starts_with("vendor/"))
            .map(|path| {
                let text = std::fs::read_to_string(root.join(&path))
                    .unwrap_or_else(|e| panic!("read {path}: {e}"));
                (path, text)
            })
            .collect();
    assert!(
        files.len() >= MIN_CMAKE_FILES,
        "only {} tracked CMake file(s) reached the scan (floor {MIN_CMAKE_FILES}); the \
         pathspec is broken, not the tree",
        files.len()
    );
    files
}

/// Every way the CMake files given (each a path and its text) send a plan to
/// the generator alone, one process for one plan when it is declared:
///
/// * a root plan (`sce_codegen_plan`, not FROM another) whose options no
///   `sce_codegen_plan_request` of the same file spells word for word
///   ([`unrequested_plans`]): the module's own, which plans each generation
///   `sce_codegen_generation` asked ahead, is held by this as much as a
///   route's;
/// * a generation declared by a call of the module's declaration outside the
///   module, where no `sce_codegen_generation` asked its plan ahead; inside
///   the module, by a call outside the deferred declaration that sends the
///   batch first ([`order_violations`]).
fn plans_sent_alone(files: &[(String, String)]) -> Vec<String> {
    let mut found = Vec::new();
    for (path, text) in files {
        found.extend(
            unrequested_plans(text)
                .into_iter()
                .map(|why| format!("{path}: {why}")),
        );
        let code = cmake_code(text);
        let declarations = invocations(&code, "_sce_codegen_generation_declare");
        let deferred = if path == PLAN_MODULE_PATH {
            invocations(
                function_body(&code, "_sce_codegen_generations_declare"),
                "_sce_codegen_generation_declare",
            )
            .len()
        } else {
            0
        };
        if declarations.len() > deferred {
            let lines: Vec<usize> = declarations.iter().map(|call| call.line).collect();
            found.push(format!(
                "{path}: `_sce_codegen_generation_declare` is called outside the deferred \
                 declaration that sends the batch first (calls at lines {lines:?}, {deferred} \
                 of them in `_sce_codegen_generations_declare`), so the plan it declares \
                 from is asked alone; call `sce_codegen_generation`"
            ));
        }
    }
    found
}

/// No CMake file of the tree sends a plan to the generator alone: every plan
/// it declares was asked ahead and answered in a batch before any was
/// declared. A configure says how many it sent alone
/// (`a_configure_asks_each_plan_once_per_input_state` reads the line); this
/// is what keeps the tree's count at none, read from every tracked CMake file
/// rather than from the ones that ask a plan today.
#[test]
fn no_cmake_file_of_the_tree_sends_a_plan_alone() {
    let files = tracked_cmake_files();
    for asking in FILES_THAT_ASK_A_PLAN {
        assert!(
            files.iter().any(|(path, _)| path == asking),
            "{asking} did not reach the scan, which reads the plans it asks"
        );
    }
    let found = plans_sent_alone(&files);
    assert!(
        found.is_empty(),
        "a CMake file of the tree sends a plan to the generator alone, one process for one \
         plan:\n  {}\n\nDeclare the generation with `sce_codegen_generation` \
         (cmake/SCECodegenPlan.cmake), which asks its plan ahead and declares it after the \
         batch, or queue the plan with `sce_codegen_plan_request` and send the batch with \
         `sce_codegen_plan_prefetch` before the plan is asked.",
        found.join("\n  ")
    );
}

/// The files besides the module's routes and test trees that declare every
/// generation of theirs through `sce_codegen_generation`: the projects
/// configured on their own (`cmake -S <their directory>`), which the root
/// build never configures, and the public functions a consumer's build
/// generates with (`sce_add_state_machine` and the rest). Each includes the
/// module ([`their_generations_are_declared_through_the_module`]).
const DECLARING_THROUGH_THE_MODULE: &[&str] = &[
    "cmake/SCECodegen.cmake",
    "backends/c/forge-runtime/tests/round_trip/CMakeLists.txt",
    "backends/cpp/forge-runtime/tests/round_trip/CMakeLists.txt",
    "backends/c/forge-runtime/tests/conformance/CMakeLists.txt",
    "backends/cpp/forge-runtime/tests/conformance/CMakeLists.txt",
];

/// Whether `command` runs a generation, with or without global options
/// between the program and the subcommand (`"${SCE_CODEGEN}" --limit n=1
/// generate`): a generating subcommand, or the run a plan names, anywhere
/// after the program.
fn runs_generate_at_all(command: &Invocation) -> bool {
    command
        .args
        .iter()
        .position(|arg| arg.contains("SCE_CODEGEN}"))
        .is_some_and(|program| {
            command.args[program + 1..]
                .iter()
                .any(|arg| GENERATING_SUBCOMMANDS.contains(&unquoted(arg)) || is_planned_run(arg))
        })
}

/// The variables `code` (without comments) assembles a command that runs a
/// generation in: a `set(<var> ...)` or `list(APPEND <var> ...)` whose values
/// run the generator with a generating subcommand, or expand a variable that
/// does (`set(<var> ${CMAKE_COMMAND} -E env ... ${<other>})`). A command run
/// as `COMMAND ${<var>}` is a generation the CMake text of its
/// `add_custom_command` does not show, and `SCECodegen.cmake` declared its
/// generations so until 2026-11-04.
fn generator_variables(code: &str) -> Vec<String> {
    let mut assignments: Vec<(String, Vec<String>)> = Vec::new();
    for set in invocations(code, "set") {
        if let Some((name, values)) = set.args.split_first() {
            assignments.push((unquoted(name).to_string(), values.to_vec()));
        }
    }
    for list in invocations(code, "list") {
        if list.args.first().map(String::as_str) == Some("APPEND") {
            if let Some((name, values)) = list.args[1..].split_first() {
                assignments.push((unquoted(name).to_string(), values.to_vec()));
            }
        }
    }
    let mut found: Vec<String> = Vec::new();
    loop {
        let before = found.len();
        for (name, values) in &assignments {
            if found.contains(name) {
                continue;
            }
            let runs = values
                .iter()
                .position(|value| value.contains("SCE_CODEGEN}"))
                .is_some_and(|program| {
                    values[program + 1..]
                        .iter()
                        .any(|value| GENERATING_SUBCOMMANDS.contains(&unquoted(value)))
                });
            let expands = values.iter().any(|value| {
                found
                    .iter()
                    .any(|variable| value.contains(&format!("${{{variable}}}")))
            });
            if runs || expands {
                found.push(name.clone());
            }
        }
        if found.len() == before {
            return found;
        }
    }
}

/// Every `add_custom_command` of the CMake files given (each a path and its
/// text) that runs a generation and is not declared from a plan the same file
/// asks: its OUTPUT, DEPFILE and byproducts are not the `<PREFIX>_OUTPUTS`,
/// `<PREFIX>_DEPFILE` and `<PREFIX>_BYPRODUCTS_ARG` of a `sce_codegen_plan`
/// call ([`generation_violations`]), or it runs a command assembled in a
/// variable ([`generator_variables`]), whose arguments no reading of the
/// command can hold to a plan. The module's own command, and the routes', are
/// declared from their plans; a generation anywhere else is declared by
/// `sce_codegen_generation`. A test that runs the generator to see it refuse
/// is an `add_test`, and no command of the build.
fn hand_declared_generations(files: &[(String, String)]) -> Vec<String> {
    let mut found = Vec::new();
    for (path, text) in files {
        let code = cmake_code(text);
        let prefixes: Vec<String> = invocations(&code, "sce_codegen_plan")
            .iter()
            .filter_map(|plan| plan.args.first().cloned())
            .collect();
        let assembled = generator_variables(&code);
        for command in invocations(&code, "add_custom_command") {
            let through: Vec<&String> = assembled
                .iter()
                .filter(|variable| {
                    command
                        .args
                        .iter()
                        .any(|arg| arg.contains(&format!("${{{variable}}}")))
                })
                .collect();
            if !through.is_empty() {
                found.push(format!(
                    "{path}: line {}: a generation run through a command assembled in a \
                     variable ({through:?}), so what it writes and reads is no plan's answer; \
                     declare it with `sce_codegen_generation`",
                    command.line
                ));
                continue;
            }
            if !runs_generate_at_all(&command) {
                continue;
            }
            let departures = generation_violations(&command, &prefixes);
            if !departures.is_empty() {
                found.push(format!(
                    "{path}: line {}: a generation declared by a command of its own ({})",
                    command.line,
                    departures.join("; ")
                ));
            }
        }
    }
    found
}

/// No CMake file of the tree declares a generation by a command of its own,
/// and none is excepted: the files a generation writes, its depfile and what
/// it requires are its plan's answer, and a command that names them itself is
/// a second list of them, which fell out of date each time a document came to
/// write one more file. The C11 tests declared 19 such commands, among them
/// every algorithm, schema and library statechart the fixtures call, until
/// 2026-11; the four projects configured on their own and the public
/// functions of `cmake/SCECodegen.cmake`, the last of them, until 2026-11-04.
#[test]
fn no_cmake_file_of_the_tree_declares_a_generation_by_hand() {
    let files = tracked_cmake_files();
    for declaring in DECLARING_THROUGH_THE_MODULE {
        assert!(
            files.iter().any(|(path, _)| path == declaring),
            "{declaring} did not reach the scan, which reads every generation it declares"
        );
    }
    let found = hand_declared_generations(&files);
    assert!(
        found.is_empty(),
        "a CMake file of the tree declares a generation by a command of its own:\n  {}\n\n\
         Declare it with `sce_codegen_generation` (cmake/SCECodegenPlan.cmake), which \
         declares the command from the generator's plan.",
        found.join("\n  ")
    );
}

/// The projects configured on their own and the public functions declare
/// every generation of theirs through the module: each includes it, and calls
/// `sce_codegen_generation`. The conformance projects declare their document
/// sets (`orchestrate`, DOCUMENTS) and their harness (`generate-conformance`,
/// MANIFEST) so too, the steps they ran by hand with no plan and, for a set,
/// no depfile.
#[test]
fn their_generations_are_declared_through_the_module() {
    let files = tracked_cmake_files();
    for declaring in DECLARING_THROUGH_THE_MODULE {
        let (_, text) = files
            .iter()
            .find(|(path, _)| path == declaring)
            .unwrap_or_else(|| panic!("{declaring} is not tracked"));
        let code = cmake_code(text);
        assert!(
            invocations(&code, "include").iter().any(|include| include
                .args
                .iter()
                .any(|arg| unquoted(arg).ends_with("/SCECodegenPlan.cmake"))),
            "{declaring} does not include cmake/SCECodegenPlan.cmake"
        );
        let generations = invocations(&code, "sce_codegen_generation");
        assert!(
            !generations.is_empty(),
            "{declaring} declares no generation through `sce_codegen_generation`"
        );
        if declaring.ends_with("conformance/CMakeLists.txt") {
            for form in ["DOCUMENTS", "MANIFEST"] {
                assert!(
                    generations
                        .iter()
                        .any(|generation| generation.args.iter().any(|arg| arg == form)),
                    "{declaring} declares no generation of a {form} through the module"
                );
            }
        }
    }
}

/// The C11 tests' former shape, an algorithm's header named and generated by
/// a command of its own, is refused; so is one with a global option before the
/// subcommand. The shape that replaced it, and the module's own command, pass.
#[test]
fn a_generation_declared_by_a_command_of_its_own_is_refused() {
    let path = "backends/c/tests/CMakeLists.txt".to_string();
    assert!(
        hand_declared_generations(&[(path.clone(), C11_TESTS.to_string())]).is_empty(),
        "the C11 tests declare a generation by hand"
    );
    assert!(
        hand_declared_generations(&[(PLAN_MODULE_PATH.to_string(), PLAN_MODULE.to_string())])
            .is_empty(),
        "the module's own command is not read as declared from its plan"
    );
    let by_hand = r#"
sce_c_integration_generation(_DAY_REPEAT "${FIXTURES}/algorithm_day_repeat.scxml")
add_custom_command(
    OUTPUT "${DIR}/day_run.h"
    COMMAND "${SCE_CODEGEN}" generate "${FIXTURES}/algorithm_day_run.scxml"
            -l c11 -o "${DIR}" --write-deps "${DIR}/day_run.h.d"
    DEPENDS "${FIXTURES}/algorithm_day_run.scxml" "${SCE_CODEGEN}"
    DEPFILE "${DIR}/day_run.h.d"
    VERBATIM
)
add_custom_command(
    OUTPUT "${DIR}/todo_peer_sm.h" "${DIR}/todo_peer_sm.c"
    COMMAND "${SCE_CODEGEN}" --limit merge_values=65536
            generate --with-imports "${FIXTURES}/todo_peer.scxml"
            -l c11 -o "${DIR}" --write-deps "${DIR}/todo_peer_sm.c.d"
    DEPFILE "${DIR}/todo_peer_sm.c.d"
    VERBATIM
)
add_test(NAME refuses COMMAND "${SCE_CODEGEN}" generate "${FIXTURES}/broken.scxml")
"#;
    let found = hand_declared_generations(&[(path.clone(), by_hand.to_string())]);
    assert_eq!(
        found.len(),
        2,
        "the two hand-declared generations, and not the refusal test: {found:?}"
    );

    // A document set and a harness run by hand are generations as much as a
    // document is: the conformance projects' former `orchestrate` and
    // `generate-conformance` steps.
    let steps = r#"
add_custom_command(
    OUTPUT ${BLOG_C_RESOURCE_HEADERS}
    COMMAND "${SCE_CODEGEN}" orchestrate ${BLOG_C_RESOURCE_ARGS}
            --language c11 --output-dir "${BLOG_C_GENERATED_DIR}"
    DEPENDS ${BLOG_C_RESOURCE_DEPENDS} "${SCE_CODEGEN}"
    VERBATIM)
add_custom_command(
    OUTPUT "${HARNESS_SRC}"
    COMMAND "${SCE_CODEGEN}" generate-conformance --language c11
            --manifest "${FIXTURE_MANIFEST}" --output-dir "${GENERATED_DIR}"
            --write-deps "${HARNESS_SRC}.d"
    DEPFILE "${HARNESS_SRC}.d"
    VERBATIM)
add_custom_command(OUTPUT "${S}" COMMAND "${SCE_CODEGEN}" check-aot-briefs --manifest "${R}" VERBATIM)
"#;
    let found = hand_declared_generations(&[(path.clone(), steps.to_string())]);
    assert_eq!(
        found.len(),
        2,
        "the set's and the harness's steps, and not a step that generates nothing: {found:?}"
    );

    // `SCECodegen.cmake`'s former shape: the command assembled in a variable,
    // wrapped in another, and run as `COMMAND ${<var>}`, where no reading of
    // the `add_custom_command` sees the generator. A variable that runs no
    // generation is no generation.
    let assembled = r#"
set(_SCE_CODEGEN_CMD "${SCE_CODEGEN}" generate
    "${SCXML_ABS_PATH}" -o "${SCE_OUTPUT_DIR}" -l "${SCE_LANGUAGE}"
    --write-deps "${_SCE_DEPFILE}")
list(APPEND _SCE_CODEGEN_CMD --as-child)
set(_SCE_WRAPPED ${CMAKE_COMMAND} -E env "SCE_TEMPLATE_DIR=${SCE_TEMPLATE_DIR}" ${_SCE_CODEGEN_CMD})
add_custom_command(
    OUTPUT "${GENERATED_OUTPUT}"
    COMMAND ${_SCE_WRAPPED}
    DEPENDS "${SCE_CODEGEN}"
    DEPFILE "${_SCE_DEPFILE}"
    VERBATIM
)
set(_COPY ${CMAKE_COMMAND} -E copy_if_different a b)
add_custom_command(OUTPUT "${O}/b" COMMAND ${_COPY} VERBATIM)
"#;
    let found = hand_declared_generations(&[(path, assembled.to_string())]);
    assert_eq!(
        found.len(),
        1,
        "the command assembled in a variable, and not the copy: {found:?}"
    );
    assert!(
        found[0].contains("assembled in a variable") && found[0].contains("_SCE_WRAPPED"),
        "the refusal does not name the variable the generation runs through: {found:?}"
    );
}

/// The variables a command running a generation is assembled in, and only
/// those: one that runs the generator with a generating subcommand, one that
/// wraps it, and not one that runs the generator for another subcommand or
/// another program.
#[test]
fn a_command_assembled_in_a_variable_is_found() {
    let code = cmake_code(
        r#"
set(_RUN "${SCE_CODEGEN}" --limit n=1 generate "${D}")
list(APPEND _SET "${SCE_CODEGEN}" orchestrate --document "${A}")
set(_ENV ${CMAKE_COMMAND} -E env X=1 ${_RUN})
set(_BRIEFS "${SCE_CODEGEN}" check-aot-briefs --manifest "${R}")
set(_CMD ${CMAKE_COMMAND} -E echo generate)
"#,
    );
    let mut found = generator_variables(&code);
    found.sort();
    assert_eq!(found, vec!["_ENV", "_RUN", "_SET"]);
}

/// The commands that declare a generation, a file calling any of which
/// composes the inputs of its plans when it is configured: the module's, and
/// the public functions that declare through it.
const DECLARING_COMMANDS: &[&str] = &[
    "sce_codegen_generation",
    "sce_codegen_plan",
    "sce_codegen_plan_request",
    "sce_add_state_machine",
    "sce_add_state_machines_from_dir",
    "sce_add_algorithm",
    "sce_create_state_machine_library",
];

/// The `file(WRITE)`s a file that declares a generation may make, each with
/// why: the file, the start of the path written, and the reason. Every other
/// file such a file composes when it is configured is written through
/// `sce_codegen_write_input`.
const WRITES_NO_GENERATION_READS: &[(&str, &str, &str)] = &[
    (
        PLAN_MODULE_PATH,
        "\"${_CACHE}/",
        "the module's cache of answers, which it reads itself and no generation does",
    ),
    (
        PLAN_MODULE_PATH,
        "\"${PATH}\"",
        "the one write `sce_codegen_write_input` makes, once it found the content changed",
    ),
];

/// Every `file(WRITE)` and `file(APPEND)` of the CMake files given (each a
/// path and its text) that declare a generation ([`DECLARING_COMMANDS`]),
/// but the ones [`WRITES_NO_GENERATION_READS`] names. Such a write runs on
/// every configure and dates its file anew whatever it holds, and a document,
/// deploy or catalog a plan read is then newer than the kept answer: the plan
/// is sent to the generator again on every configure, and its generation runs
/// again on every build after. The conformance projects wrote their queue
/// documents and deploys so until 2026-11-04, and a reconfigure that changed
/// nothing asked those plans again.
fn configure_time_rewrites(files: &[(String, String)]) -> Vec<String> {
    let mut found = Vec::new();
    for (path, text) in files {
        let code = cmake_code(text);
        let declares = DECLARING_COMMANDS
            .iter()
            .any(|name| !invocations(&code, name).is_empty());
        if !declares {
            continue;
        }
        for write in invocations(&code, "file") {
            let Some(mode) = write.args.first() else {
                continue;
            };
            if mode != "WRITE" && mode != "APPEND" {
                continue;
            }
            let target = write.args.get(1).map(String::as_str).unwrap_or_default();
            if WRITES_NO_GENERATION_READS
                .iter()
                .any(|(at, start, _)| *at == path.as_str() && target.starts_with(*start))
            {
                continue;
            }
            found.push(format!(
                "{path}: line {}: `file({mode} {target} ...)` dates the file anew on every \
                 configure, so a plan that read it is asked again and its generation runs \
                 again; write it with `sce_codegen_write_input` (cmake/SCECodegenPlan.cmake), \
                 which keeps unchanged content as it is",
                write.line
            ));
        }
    }
    found
}

/// No file of the tree that declares a generation rewrites a file on every
/// configure: a reconfigure that changed nothing asks no plan of the
/// generator.
#[test]
fn no_file_that_declares_a_generation_rewrites_an_input_on_every_configure() {
    let files = tracked_cmake_files();
    let found = configure_time_rewrites(&files);
    assert!(
        found.is_empty(),
        "a CMake file of the tree rewrites a file on every configure:\n  {}",
        found.join("\n  ")
    );
}

/// The conformance projects' former shape, a queue document and its deploy
/// written with `file(WRITE)` beside the generation that reads them, is
/// refused; written through `sce_codegen_write_input` it passes, as does a
/// file that declares no generation, and the module's own writes. A write of
/// the module outside its cache and the helper is refused.
#[test]
fn a_document_rewritten_on_every_configure_is_refused() {
    let file = |path: &str, text: &str| vec![(path.to_string(), text.to_string())];
    let rewritten = r#"
file(WRITE "${QUEUE_C_DEPLOY}" [=[
version: "1.0"
]=])
file(WRITE "${QUEUE_C_DOCS_DIR}/${name}.scxml" "${_text}")
sce_codegen_generation(_QUEUE DOCUMENT "${QUEUE_C_DOCS_DIR}/q.scxml" OUTPUT_DIR "${O}"
    LANGUAGE c11 DEPLOY "${QUEUE_C_DEPLOY}")
"#;
    let found = configure_time_rewrites(&file("backends/probe/CMakeLists.txt", rewritten));
    assert_eq!(
        found.len(),
        2,
        "the deploy and the document rewritten on every configure: {found:?}"
    );
    let kept = rewritten.replace("file(WRITE ", "sce_codegen_write_input(");
    assert_eq!(
        configure_time_rewrites(&file("backends/probe/CMakeLists.txt", &kept)),
        Vec::<String>::new(),
        "inputs written through sce_codegen_write_input were refused"
    );
    let no_generation = "file(WRITE \"${D}/notes.txt\" \"x\")\n";
    assert_eq!(
        configure_time_rewrites(&file("tests/probe/CMakeLists.txt", no_generation)),
        Vec::<String>::new(),
        "a file that declares no generation was held to its writes"
    );
    assert_eq!(
        configure_time_rewrites(&file(PLAN_MODULE_PATH, PLAN_MODULE)),
        Vec::<String>::new(),
        "the module's own writes were refused"
    );
    let stray = format!("{PLAN_MODULE}\nfile(WRITE \"${{OUTPUT_DIR}}/stray.scxml\" \"x\")\n");
    assert_eq!(
        configure_time_rewrites(&file(PLAN_MODULE_PATH, &stray)).len(),
        1,
        "a write of the module outside its cache and the helper was accepted"
    );
}

/// A plan no request asks ahead is refused in whichever file of the tree it
/// is written, and a plan a request of the same file asks word for word
/// passes; so is a generation declared outside the module's deferred
/// declaration, and the module itself when it plans what it did not ask.
#[test]
fn a_plan_sent_alone_is_refused_wherever_it_is() {
    let file = |path: &str, text: &str| vec![(path.to_string(), text.to_string())];
    let asked = format!(
        "sce_codegen_plan_request(STEM \"${{M}}\" DOCUMENT \"${{D}}\" OUTPUT_DIR \"${{O}}\" \
         LANGUAGE cpp)\nsce_codegen_plan_prefetch()\n{PLANNED_GENERATION}"
    );
    assert_eq!(
        plans_sent_alone(&file("examples/probe/CMakeLists.txt", &asked)),
        Vec::<String>::new()
    );
    let alone = plans_sent_alone(&file("examples/probe/CMakeLists.txt", PLANNED_GENERATION));
    assert!(
        alone
            .iter()
            .any(|v| v.starts_with("examples/probe/CMakeLists.txt: ")
                && v.contains("is asked ahead by no request")),
        "a plan no request asks ahead, in a file no route is, was accepted: {alone:?}"
    );
    let declared = plans_sent_alone(&file(
        "tests/probe/CMakeLists.txt",
        "_sce_codegen_generation_declare(_P 1)\n",
    ));
    assert!(
        declared
            .iter()
            .any(|v| v.contains("called outside the deferred declaration")),
        "a generation declared outside the module was accepted: {declared:?}"
    );
    assert_eq!(
        plans_sent_alone(&file(PLAN_MODULE_PATH, PLAN_MODULE)),
        Vec::<String>::new(),
        "the module as it stands sends a plan alone"
    );
    let unasked = PLAN_MODULE.replace(
        "sce_codegen_plan_request(${_PLAN_OPTIONS})",
        "set(_ASKED ${_PLAN_OPTIONS})",
    );
    assert_ne!(
        unasked, PLAN_MODULE,
        "the module no longer asks ahead as this case expects"
    );
    let found = plans_sent_alone(&file(PLAN_MODULE_PATH, &unasked));
    assert!(
        found
            .iter()
            .any(|v| v.contains("is asked ahead by no request")),
        "a module that plans a generation it did not ask ahead was accepted: {found:?}"
    );
}

/// The C11 conformance suite's build, which registers every fixture the
/// registry lists through `sce_generate_static_w3c_c_test`, in a loop over the
/// ids `sce_w3c_registry_fixtures` read (`w3c_registry_drives_cmake`), and
/// declares them together after the last registration
/// ([`the_routes_ask_every_plan_before_they_declare_one`]).
const C11_TESTS: &str = include_str!("../../backends/c/tests/CMakeLists.txt");

/// The body of the CMake function `name` in `code` (without comments), from
/// its `function(` to its `endfunction(`.
fn function_body<'a>(code: &'a str, name: &str) -> &'a str {
    let opening = format!("function({name}");
    let start = code
        .match_indices(&opening)
        .map(|(at, _)| at)
        .find(|&at| {
            code[at + opening.len()..]
                .chars()
                .next()
                .is_some_and(|next| next == ')' || next.is_whitespace())
        })
        .unwrap_or_else(|| panic!("no function `{name}` is defined"));
    let end = code[start..]
        .find("endfunction(")
        .map_or(code.len(), |offset| start + offset);
    &code[start..end]
}

/// Whether each of `steps` is in `code`, each first found after the one
/// before it: `Err` names the first that is missing or out of its place.
fn in_order(code: &str, steps: &[&str]) -> Result<(), String> {
    let mut from = 0;
    for step in steps {
        match code[from..].find(step) {
            Some(at) => from += at + step.len(),
            None => {
                return Err(format!(
                    "`{step}` is not there after {:?}",
                    &steps[..steps.iter().position(|s| s == step).unwrap_or(0)]
                ))
            }
        }
    }
    Ok(())
}

/// Where `needle` last occurs in `code`, or `None`.
fn last(code: &str, needle: &str) -> Option<usize> {
    code.rfind(needle)
}

/// Every way the routes, the module and their callers depart from "request
/// first, declare after": each route queues the plans of every generation it
/// will declare, sends them in one batch (`sce_codegen_plan_prefetch`), and
/// only then declares; `sce_codegen_generation` queues its plan and defers
/// its declaration to the end of the directory, which sends the batch before
/// it declares; and a caller declares its registered fixtures after the last
/// registration and before the first target that compiles them.
fn order_violations(
    module: &str,
    w3c: &str,
    integration: &str,
    tree: &str,
    c11: &str,
) -> Vec<String> {
    let (module, w3c, integration, tree, c11) = (
        cmake_code(module),
        cmake_code(w3c),
        cmake_code(integration),
        cmake_code(tree),
        cmake_code(c11),
    );
    let mut found = Vec::new();
    let mut must = |what: &str, result: Result<(), String>| {
        if let Err(why) = result {
            found.push(format!("{what}: {why}"));
        }
    };
    must(
        "cmake/SCEStaticW3CTest.cmake, sce_prefetch_static_w3c_tests",
        in_order(
            function_body(&w3c, "sce_prefetch_static_w3c_tests"),
            &["_sce_request_w3c_fixture(", "sce_codegen_plan_prefetch("],
        ),
    );
    must(
        "cmake/SCEStaticW3CTest.cmake, sce_declare_registered_static_w3c_c_tests",
        in_order(
            function_body(&w3c, "sce_declare_registered_static_w3c_c_tests"),
            &[
                "_sce_request_w3c_fixture(",
                "sce_codegen_plan_prefetch(",
                "_sce_generate_w3c_fixture(",
            ],
        ),
    );
    must(
        "cmake/SCEStaticIntegrationFixture.cmake, sce_declare_registered_static_integration_tests",
        in_order(
            function_body(
                &integration,
                "sce_declare_registered_static_integration_tests",
            ),
            &[
                "_sce_static_integration_fixture(REQUEST",
                "sce_codegen_plan_prefetch(",
                "_sce_static_integration_fixture(DECLARE",
            ],
        ),
    );
    must(
        "cmake/SCEStaticIntegrationFixture.cmake, sce_generate_static_integration_c_test",
        in_order(
            function_body(&integration, "sce_generate_static_integration_c_test"),
            &[
                "_sce_static_integration_c_fixture(REQUEST",
                "cmake_language(DEFER CALL _sce_declare_registered_static_integration_c_tests",
            ],
        ),
    );
    must(
        "cmake/SCEStaticIntegrationFixture.cmake, _sce_declare_registered_static_integration_c_tests",
        in_order(
            function_body(&integration, "_sce_declare_registered_static_integration_c_tests"),
            &[
                "sce_codegen_plan_prefetch(",
                "_sce_static_integration_c_fixture(DECLARE",
            ],
        ),
    );
    must(
        "cmake/SCECodegenPlan.cmake, sce_codegen_generation",
        in_order(
            function_body(&module, "sce_codegen_generation"),
            &[
                "sce_codegen_plan_request(",
                "cmake_language(DEFER CALL _sce_codegen_generations_declare)",
            ],
        ),
    );
    must(
        "cmake/SCECodegenPlan.cmake, _sce_codegen_generations_declare",
        in_order(
            function_body(&module, "_sce_codegen_generations_declare"),
            &[
                "sce_codegen_plan_prefetch(",
                "_sce_codegen_generation_declare(",
            ],
        ),
    );
    // A registering call declares nothing itself.
    let generation = function_body(&module, "sce_codegen_generation");
    for (what, body, declares) in [
        (
            "sce_codegen_generation",
            generation,
            "_sce_codegen_generation_declare(",
        ),
        ("sce_codegen_generation", generation, "sce_codegen_plan("),
        ("sce_codegen_generation", generation, "add_custom_command("),
        (
            "sce_generate_static_integration_c_test",
            function_body(&integration, "sce_generate_static_integration_c_test"),
            "(DECLARE",
        ),
        (
            "sce_generate_static_integration_test",
            function_body(&integration, "sce_generate_static_integration_test"),
            "(DECLARE",
        ),
        (
            "sce_generate_static_w3c_c_test",
            function_body(&w3c, "sce_generate_static_w3c_c_test"),
            "_sce_generate_w3c_fixture(",
        ),
    ] {
        if body.contains(declares) {
            found.push(format!(
                "{what} declares a generation when it is called (`{declares}`), before the \
                 plans of the fixtures registered after it were asked"
            ));
        }
    }
    // The conformance fixtures' plans are asked before the loop that declares
    // them.
    let prefetched = tree.find("sce_prefetch_static_w3c_tests(");
    let declared = tree.find("sce_generate_static_w3c_test(");
    if !matches!((prefetched, declared), (Some(asked), Some(first)) if asked < first) {
        found.push(format!(
            "tests/CMakeLists.txt: `sce_prefetch_static_w3c_tests` is not called before the \
             first `sce_generate_static_w3c_test` ({prefetched:?}, {declared:?})"
        ));
    }
    // The registering callers: every registration before the one
    // declaration, and the declaration before the first target that compiles
    // what it declares.
    let callers = [
        (
            "tests/CMakeLists.txt",
            &tree,
            "sce_generate_static_integration_test(",
            "sce_declare_registered_static_integration_tests(",
            "${GENERATED_INTEGRATION_HEADERS}",
        ),
        (
            "backends/c/tests/CMakeLists.txt",
            &c11,
            "sce_generate_static_w3c_c_test(",
            "sce_declare_registered_static_w3c_c_tests(",
            "W3C_C_AOT_TEST_${",
        ),
    ];
    for (file, code, registration, declaration, use_) in callers {
        let (last_registration, place, first_use) = (
            last(code, registration),
            code.find(declaration),
            code.find(use_),
        );
        let ordered = matches!(
            (last_registration, place, first_use),
            (Some(registered), Some(place), Some(used)) if registered < place && place < used
        );
        if !ordered {
            found.push(format!(
                "{file}: `{declaration}` is not after every `{registration}` and before the \
                 first `{use_}` ({last_registration:?}, {place:?}, {first_use:?})"
            ));
        }
    }
    found
}

/// Every route asks the plans of all the generations it declares before it
/// declares one, in one batch, and its callers declare after their last
/// registration: on a fresh configure the generator is sent a handful of
/// batches, not one process per plan.
#[test]
fn the_routes_ask_every_plan_before_they_declare_one() {
    let found = order_violations(
        PLAN_MODULE,
        W3C_ROUTE,
        INTEGRATION_ROUTE,
        TEST_TREE,
        C11_TESTS,
    );
    assert!(
        found.is_empty(),
        "a route declares a generation before it asked the plans of the others:\n  {}",
        found.join("\n  ")
    );
}

/// A declaration made before the batch is sent, and a registration made
/// after the declaration, are refused; so is a `sce_codegen_generation`
/// that declares where it is called, or whose deferred declaration declares
/// before it sends the batch.
#[test]
fn a_declaration_before_its_batch_is_refused() {
    let eager = PLAN_MODULE.replace(
        "    sce_codegen_plan_request(${_PLAN_OPTIONS})\n",
        "    sce_codegen_plan_request(${_PLAN_OPTIONS})\n    \
         _sce_codegen_generation_declare(${PREFIX} 0)\n",
    );
    assert_ne!(
        eager, PLAN_MODULE,
        "the module no longer asks ahead as this case expects"
    );
    let found = order_violations(&eager, W3C_ROUTE, INTEGRATION_ROUTE, TEST_TREE, C11_TESTS);
    assert!(
        found
            .iter()
            .any(|v| v.contains("sce_codegen_generation declares a generation when it is called")),
        "a generation declared where it is called was accepted: {found:?}"
    );
    let unbatched = PLAN_MODULE.replace(
        "    sce_codegen_plan_prefetch()\n    get_property(_IDS",
        "    get_property(_IDS",
    );
    assert_ne!(
        unbatched, PLAN_MODULE,
        "the declaration no longer sends the batch first"
    );
    let found = order_violations(
        &unbatched,
        W3C_ROUTE,
        INTEGRATION_ROUTE,
        TEST_TREE,
        C11_TESTS,
    );
    assert!(
        found
            .iter()
            .any(|v| v.contains("_sce_codegen_generations_declare")),
        "a deferred declaration that sends no batch first was accepted: {found:?}"
    );

    assert!(in_order(
        "sce_codegen_plan_prefetch()\n_sce_generate_w3c_fixture(1 o c11)",
        &["sce_codegen_plan_prefetch(", "_sce_generate_w3c_fixture("]
    )
    .is_ok());
    assert!(in_order(
        "_sce_generate_w3c_fixture(1 o c11)\nsce_codegen_plan_prefetch()",
        &["sce_codegen_plan_prefetch(", "_sce_generate_w3c_fixture("]
    )
    .is_err());
    let late = C11_TESTS.replace(
        "sce_declare_registered_static_w3c_c_tests()",
        "sce_declare_registered_static_w3c_c_tests()\n\
         sce_generate_static_w3c_c_test(999 \"${SCE_C_GENERATED_DIR}\")",
    );
    let found = order_violations(PLAN_MODULE, W3C_ROUTE, INTEGRATION_ROUTE, TEST_TREE, &late);
    assert!(
        found
            .iter()
            .any(|v| v.contains("backends/c/tests/CMakeLists.txt")),
        "a fixture registered after the declaration was accepted: {found:?}"
    );
}

/// The environment variables of the generator that a plan's answer cannot
/// depend on, each with why. Every other `SCE_` variable it reads is one the
/// key of a kept answer carries (`_ENVIRONMENT` in `_sce_codegen_plan_entry`,
/// cmake/SCECodegenPlan.cmake).
const ENVIRONMENT_NO_PLAN_DEPENDS_ON: &[(&str, &str)] = &[
    (
        "SCE_REQUIRE_TOOLS",
        "a test lane's demand that the tools it compiles with are present; a plan compiles \
         nothing",
    ),
    (
        "SCE_REQUIRE_ALL_COMPILERS",
        "a test's demand that every compiler it runs is present; a plan compiles nothing",
    ),
];

/// Every `SCE_` environment variable the generator's sources read: a name
/// spelled on a line that reads the environment, or the constant such a line
/// reads it through.
fn environment_the_generator_reads() -> std::collections::BTreeSet<String> {
    fn sources(dir: &std::path::Path, found: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).expect("read a source directory") {
            let path = entry.expect("a directory entry").path();
            if path.is_dir() {
                sources(&path, found);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                found.push(path);
            }
        }
    }
    let literal = regex::Regex::new(r#""(SCE_[A-Z0-9_]+)""#).expect("pattern");
    let constant =
        regex::Regex::new(r#"const\s+(\w+)\s*:\s*&(?:'static\s+)?str\s*=\s*"(SCE_[A-Z0-9_]+)""#)
            .expect("pattern");
    let mut files = Vec::new();
    sources(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut files,
    );
    let mut names = std::collections::BTreeSet::new();
    for file in files {
        let text = std::fs::read_to_string(&file).expect("read a source");
        let constants: Vec<(String, String)> = constant
            .captures_iter(&text)
            .map(|found| (found[1].to_string(), found[2].to_string()))
            .collect();
        for line in text.lines().filter(|line| line.contains("env::var")) {
            names.extend(
                literal
                    .captures_iter(line)
                    .map(|found| found[1].to_string()),
            );
            names.extend(
                constants
                    .iter()
                    .filter(|(constant, _)| line.contains(constant.as_str()))
                    .map(|(_, name)| name.clone()),
            );
        }
    }
    names
}

/// A kept answer is keyed on every environment variable the generator reads
/// that could change it, and on none it does not read: a variable left out
/// lets an answer the generator would now give differently be reused.
#[test]
fn the_key_of_a_kept_plan_carries_the_environment_the_generator_reads() {
    let read = environment_the_generator_reads();
    assert!(
        read.contains("SCE_TEMPLATE_DIR") && read.contains("SCE_FILE_ROOT"),
        "the scan of the generator's sources found neither the template directory nor the \
         confinement it reads, so it reads nothing: {read:?}"
    );
    let module = cmake_code(PLAN_MODULE);
    let keyed: std::collections::BTreeSet<String> = invocations(&module, "set")
        .into_iter()
        .find(|set| set.args.first().map(String::as_str) == Some("_ENVIRONMENT"))
        .expect("cmake/SCECodegenPlan.cmake names the environment a kept answer is keyed on")
        .args
        .into_iter()
        .skip(1)
        .collect();
    let exempt: std::collections::BTreeSet<String> = ENVIRONMENT_NO_PLAN_DEPENDS_ON
        .iter()
        .map(|(name, _)| name.to_string())
        .collect();
    let unkeyed: Vec<&String> = read
        .iter()
        .filter(|name| !keyed.contains(*name) && !exempt.contains(*name))
        .collect();
    assert!(
        unkeyed.is_empty(),
        "the generator reads {unkeyed:?} and a kept plan is not keyed on it: add it to \
         `_ENVIRONMENT` in cmake/SCECodegenPlan.cmake, or say here why no plan depends on it"
    );
    let unread: Vec<&String> = keyed
        .union(&exempt)
        .filter(|name| !read.contains(*name))
        .collect();
    assert!(
        unread.is_empty(),
        "{unread:?} is named for a variable the generator no longer reads"
    );
}

// The scanner, held to what it must see and what it must not.

/// The shape the route uses, which the checker has to accept: a scan that
/// refused everything would make the test above meaningless the other way.
const PLANNED_GENERATION: &str = r#"
sce_codegen_plan(_PLAN STEM "${M}" DOCUMENT "${D}" OUTPUT_DIR "${O}" LANGUAGE cpp)
add_custom_command(
    OUTPUT ${_PLAN_OUTPUTS}
    COMMAND "${SCE_CODEGEN}" generate "${D}" -o "${O}" --write-deps "${_PLAN_DEPFILE}"
    DEPENDS "${D}" "${SCE_CODEGEN}"
    DEPFILE "${_PLAN_DEPFILE}"
    ${_PLAN_BYPRODUCTS_ARG}
    COMMENT "Generating: ${M}"
    VERBATIM
)
"#;

#[test]
fn a_generation_declared_from_its_plan_passes() {
    assert_eq!(violations(PLANNED_GENERATION), Vec::<String>::new());
}

#[test]
fn a_hand_named_output_is_refused() {
    let hand = PLANNED_GENERATION.replace("OUTPUT ${_PLAN_OUTPUTS}", "OUTPUT \"${O}/test1_sm.h\"");
    let found = violations(&hand);
    assert!(
        found.iter().any(|v| v.contains("declares OUTPUT")),
        "an OUTPUT the plan did not set was accepted: {found:?}"
    );
    assert!(
        found.iter().any(|v| v.contains("`_sm.`")),
        "a machine's name spelled by hand was not seen: {found:?}"
    );
}

#[test]
fn a_hand_listed_byproduct_is_refused() {
    let hand = PLANNED_GENERATION.replace(
        "${_PLAN_BYPRODUCTS_ARG}",
        "BYPRODUCTS \"${O}/${M}_transition_function.h\"",
    );
    let found = violations(&hand);
    assert!(
        found
            .iter()
            .any(|v| v.contains("lists BYPRODUCTS of its own")),
        "a BYPRODUCTS list of the route's own was accepted: {found:?}"
    );
    assert!(
        found
            .iter()
            .any(|v| v.contains("does not declare ${_PLAN_BYPRODUCTS_ARG}")),
        "a generation without the plan's byproducts was accepted: {found:?}"
    );
    assert!(
        found.iter().any(|v| v.contains("`_transition_function`")),
        "the function header's name spelled by hand was not seen: {found:?}"
    );
}

#[test]
fn an_output_of_a_prefix_nothing_planned_is_refused() {
    let unplanned =
        PLANNED_GENERATION.replace("OUTPUT ${_PLAN_OUTPUTS}", "OUTPUT ${_OTHER_OUTPUTS}");
    let found = violations(&unplanned);
    assert!(
        found.iter().any(|v| v.contains("declares OUTPUT")),
        "an OUTPUT no `sce_codegen_plan` call set was accepted: {found:?}"
    );
}

#[test]
fn a_depfile_the_generator_is_not_asked_for_is_refused() {
    let elsewhere = PLANNED_GENERATION.replace(
        "--write-deps \"${_PLAN_DEPFILE}\"",
        "--write-deps \"${O}/other.d\"",
    );
    let found = violations(&elsewhere);
    assert!(
        found
            .iter()
            .any(|v| v.contains("asks the generator for the depfile")),
        "a depfile other than the declared one was accepted: {found:?}"
    );
}

#[test]
fn a_build_step_that_is_no_generation_is_refused() {
    let staging = format!(
        "{PLANNED_GENERATION}\nadd_custom_command(OUTPUT \"${{O}}/x.scxml\" COMMAND \
         ${{CMAKE_COMMAND}} -E copy a b VERBATIM)\n"
    );
    let found = violations(&staging);
    assert!(
        found.iter().any(|v| v.contains("runs no `generate`")),
        "a build step that writes a file without a plan was accepted: {found:?}"
    );
}

#[test]
fn a_glob_over_a_fixture_is_refused() {
    let globbed = format!("{PLANNED_GENERATION}\nfile(GLOB _FOUND \"${{D}}/test1sub*.scxml\")\n");
    let found = violations(&globbed);
    assert!(
        found.iter().any(|v| v.contains("found by a name pattern")),
        "a `file(GLOB)` over a fixture was accepted: {found:?}"
    );
}

#[test]
fn a_comment_is_not_code_and_a_quoted_hash_is_not_a_comment() {
    let text = "# writes test1_sm.h\nmessage(\"#_parent\") #[[ test1_sm.inl\n]] set(A B)\n";
    let code = cmake_code(text);
    assert!(!code.contains("_sm."), "a comment survived: {code:?}");
    assert!(
        code.contains("\"#_parent\""),
        "a quoted `#` was read as a comment: {code:?}"
    );
    assert!(
        code.contains("set(A B)"),
        "code after a bracket comment was lost: {code:?}"
    );
    assert_eq!(
        code.lines().count(),
        text.lines().count(),
        "removing comments moved the line numbering"
    );
}

#[test]
fn a_command_named_inside_a_string_is_not_an_invocation() {
    let code = cmake_code("message(\"add_custom_command(OUTPUT x)\")\n");
    assert!(invocations(&code, "add_custom_command").is_empty());
    assert_eq!(invocations(&code, "message").len(), 1);
}
