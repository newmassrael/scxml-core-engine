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

/// The route under test. Compiled in, so an edit to it starts the suite that
/// judges it (`include-str-coverage` in tools/git-hooks/gate_registry.py).
const W3C_ROUTE: &str = include_str!("../../cmake/SCEStaticW3CTest.cmake");
/// The other route that generates at build time.
const INTEGRATION_ROUTE: &str = include_str!("../../cmake/SCEStaticIntegrationFixture.cmake");
/// The one module that asks the generator for a plan.
const PLAN_MODULE: &str = include_str!("../../cmake/SCECodegenPlan.cmake");

/// Fragments of the names the generator gives the files it writes. A route
/// that spells one is naming a generated file itself.
const GENERATED_NAME_FRAGMENTS: &[&str] = &[
    // A machine's header, its C++ inline part, its C translation unit.
    "_sm.",
    // The header of a declared transition function.
    "_transition_function",
    // A child synthesized from an inline `<invoke>`.
    "__sce_synth_invoke__",
    // The stub of a hybrid `<invoke>`.
    "_hybrid",
    // The list of children a C++ generation writes.
    "_children.txt",
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
    let runs_generator = args
        .windows(2)
        .any(|pair| pair[0].contains("SCE_CODEGEN}") && pair[1] == "generate");
    if !runs_generator {
        return vec![format!(
            "{at} runs no `generate`. Every file this route's build writes is a \
             generation's, declared from its plan; a document is staged when the tree \
             is configured, not by a build step"
        )];
    }

    let mut found = Vec::new();
    let output = keyword_values(args, "OUTPUT").unwrap_or_default();
    let prefix = match output {
        [only] => only
            .strip_prefix("${")
            .and_then(|rest| rest.strip_suffix("_OUTPUTS}"))
            .filter(|prefix| prefixes.iter().any(|asked| asked == prefix)),
        _ => None,
    };
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

/// A child's generation declared from the plan its parent's answer holds is
/// the shape the integration route uses, and passes.
#[test]
fn a_child_generation_declared_from_its_parents_plan_passes() {
    let child = format!(
        "{PLANNED_GENERATION}\nforeach(C ${{_PLAN_GENERATIONS}})\n\
         sce_codegen_plan(_CHILD_PLAN FROM _PLAN CHILD \"${{C}}\")\n\
         add_custom_command(\n\
             OUTPUT ${{_CHILD_PLAN_OUTPUTS}}\n\
             COMMAND \"${{SCE_CODEGEN}}\" generate \"${{_CHILD_PLAN_DOCUMENT}}\" --as-child \
         --write-deps \"${{_CHILD_PLAN_DEPFILE}}\"\n\
             DEPENDS ${{_CHILD_PLAN_DEPENDS}} \"${{SCE_CODEGEN}}\"\n\
             DEPFILE \"${{_CHILD_PLAN_DEPFILE}}\"\n\
             ${{_CHILD_PLAN_BYPRODUCTS_ARG}}\n\
             VERBATIM\n\
         )\nendforeach()\n"
    );
    assert_eq!(violations(&child), Vec::<String>::new());
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
    let plans: Vec<Invocation> = invocations(&module, "execute_process");
    assert!(
        plans
            .iter()
            .any(|process| process.args.iter().any(|arg| arg == "--plan")),
        "cmake/SCECodegenPlan.cmake no longer runs `sce-codegen --plan generate`"
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
