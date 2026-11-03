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

/// The route under test. Compiled in, so an edit to it starts the suite that
/// judges it (`include-str-coverage` in tools/git-hooks/gate_registry.py).
const W3C_ROUTE: &str = include_str!("../../cmake/SCEStaticW3CTest.cmake");
/// The other route that generates at build time.
const INTEGRATION_ROUTE: &str = include_str!("../../cmake/SCEStaticIntegrationFixture.cmake");
/// The one module that asks the generator for a plan.
const PLAN_MODULE: &str = include_str!("../../cmake/SCECodegenPlan.cmake");
/// The test tree's build, which declares every generation it runs, the Mesh
/// fixtures' among them, through `sce_codegen_generation`
/// ([`test_tree_violations`]).
const TEST_TREE: &str = include_str!("../../tests/CMakeLists.txt");

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

/// The text of a CMake file with its comments removed and its line numbering
/// kept: a line comment runs from an unquoted `#` to the end of the line, a
/// bracket comment from `#[[` (or `#[=[`, with any number of `=`) to the
/// matching close. A `#` inside a quoted argument is text (`"#_parent"`).
fn cmake_code(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut quoted = false;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if quoted {
            out.push(c);
            if c == '\\' {
                if let Some(&escaped) = chars.get(i + 1) {
                    out.push(escaped);
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
                out.push(c);
                i += 1;
            }
            '#' => {
                let end = bracket_comment_end(&chars, i + 1).unwrap_or_else(|| {
                    chars[i..]
                        .iter()
                        .position(|&ch| ch == '\n')
                        .map_or(chars.len(), |offset| i + offset)
                });
                out.extend(chars[i..end].iter().filter(|&&ch| ch == '\n'));
                i = end;
            }
            _ => {
                out.push(c);
                i += 1;
            }
        }
    }
    out
}

/// Where a bracket comment opened at `at` (just past its `#`) ends, or `None`
/// when what follows the `#` opens no bracket.
fn bracket_comment_end(chars: &[char], at: usize) -> Option<usize> {
    if chars.get(at) != Some(&'[') {
        return None;
    }
    let mut j = at + 1;
    let mut level = 0;
    while chars.get(j) == Some(&'=') {
        level += 1;
        j += 1;
    }
    if chars.get(j) != Some(&'[') {
        return None;
    }
    let mut closing = vec![']'; level + 2];
    closing[1..=level].fill('=');
    let body = j + 1;
    chars[body..]
        .windows(closing.len())
        .position(|window| window == closing.as_slice())
        .map(|offset| body + offset + closing.len())
}

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
     `sce_add_state_machine` (cmake/SCECodegen.cmake), whose command declares it",
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

/// Whether `command` runs `sce-codegen generate`.
fn runs_generate(command: &Invocation) -> bool {
    command
        .args
        .windows(2)
        .any(|pair| pair[0].contains("SCE_CODEGEN}") && pair[1] == "generate")
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
/// the plan of the parent's document: the parent's is asked with
/// CHILD_GENERATIONS and the child's is read FROM it, with nothing of its
/// own stated.
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
    let parent = generations
        .iter()
        .filter(|generation| generation.args.iter().any(|arg| arg == "CHILD_GENERATIONS"))
        .count();
    let children: Vec<&Invocation> = generations
        .iter()
        .filter(|generation| generation.args.iter().any(|arg| arg == "FROM"))
        .collect();
    assert_eq!(
        (parent, children.len()),
        (1, 1),
        "the Mesh rule-3 override asks the parent's plan for its children's generations and \
         declares the partition's from it"
    );
    for stated in ["ARGS", "DEPLOY", "DOCUMENT", "--deploy", "--as-child"] {
        assert!(
            !children[0].args.iter().any(|arg| arg == stated),
            "the partition's generation states `{stated}` itself; it is the plan's to state"
        );
    }
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
/// alike, and the conformance route's fixtures.
#[test]
fn every_plan_a_route_declares_is_asked_ahead() {
    for (route, text) in [
        ("cmake/SCEStaticW3CTest.cmake", W3C_ROUTE),
        ("cmake/SCEStaticIntegrationFixture.cmake", INTEGRATION_ROUTE),
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

/// The C11 conformance suite's build, which registers its fixtures through
/// `sce_generate_static_w3c_c_test` and declares them together
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

/// Every way the routes and their callers depart from "request first, declare
/// after": each route queues the plans of every generation it will declare,
/// sends them in one batch (`sce_codegen_plan_prefetch`), and only then
/// declares, and a caller declares its registered fixtures after the last
/// registration and before the first target that compiles them.
fn order_violations(w3c: &str, integration: &str, tree: &str, c11: &str) -> Vec<String> {
    let (w3c, integration, tree, c11) = (
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
        "tests/CMakeLists.txt, sce_mesh_fixture_generations",
        in_order(
            function_body(&tree, "sce_mesh_fixture_generations"),
            &[
                "cmake_language(DEFER CALL _sce_declare_mesh_fixture_generations",
                "_sce_mesh_fixture_generation(REQUEST",
            ],
        ),
    );
    must(
        "tests/CMakeLists.txt, _sce_declare_mesh_fixture_generations",
        in_order(
            function_body(&tree, "_sce_declare_mesh_fixture_generations"),
            &[
                "sce_codegen_plan_prefetch(",
                "_sce_mesh_fixture_generation(DECLARE",
            ],
        ),
    );
    // A registering call declares nothing itself.
    for (what, body, declares) in [
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
        (
            "sce_mesh_fixture_generations",
            function_body(&tree, "sce_mesh_fixture_generations"),
            "(DECLARE",
        ),
    ] {
        if body.contains(declares) {
            found.push(format!(
                "{what} declares a generation when it is called, before the plans of the \
                 fixtures registered after it were asked"
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
            "target_sources(integration_tests",
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
    let found = order_violations(W3C_ROUTE, INTEGRATION_ROUTE, TEST_TREE, C11_TESTS);
    assert!(
        found.is_empty(),
        "a route declares a generation before it asked the plans of the others:\n  {}",
        found.join("\n  ")
    );
}

/// A declaration made before the batch is sent, and a registration made
/// after the declaration, are refused.
#[test]
fn a_declaration_before_its_batch_is_refused() {
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
    let found = order_violations(W3C_ROUTE, INTEGRATION_ROUTE, TEST_TREE, &late);
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
