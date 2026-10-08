// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! Every place that opens a file goes through `confine`, unless it is listed as not a document's.
//!
//! # What is held
//!
//! A caller can confine the generator to one folder (`SCE_FILE_ROOT`), so that a document handed
//! over by someone who does not own the machine cannot make it open a file elsewhere, or tell
//! that one is there. That holds for the places that were found, and a place added later that
//! opens a file directly would be a way round it that nothing noticed: a document would reach it
//! and the folder would not.
//!
//! The rule is therefore the other way round from a list of the modules that read documents. A
//! list of those is a list of the places somebody thought of, and it was short by two when it was
//! first checked against all the sources (`<data src>` read by a template filter, and the copy of
//! an `sce:candidates` document into the output). So every source of the generator is scanned, a
//! file that is not in the table below is held to the rule (it opens nothing but through
//! `confine::{exists, read, read_to_string}`), and a place that opens a file otherwise is listed
//! here with the function it is in, how many times, and why the file it opens is not one a
//! document names. A new one fails this test, a listed one that is gone or has another count
//! fails it too, so that the list is no longer than what is true and a function that was listed
//! for one open cannot take a second without being looked at.
//!
//! The test reads the sources, which is what the rule is about: where a file is opened is a fact
//! of the text. It reads them without their comments, strings and tests.
//!
//! `canonicalize` is not among the ways of looking: it is used for keys (a cycle, a set of files
//! already read) and what it returns is not said. A file is opened or looked for by the others.

use std::fs;
use std::path::{Path, PathBuf};

/// How a file is opened or looked for, past `confine`.
const RAW: [&str; 11] = [
    "fs::read_to_string(",
    "fs::read(",
    "File::open(",
    ".exists()",
    ".is_file()",
    ".is_dir()",
    "fs::read_dir(",
    "fs::metadata(",
    ".metadata()",
    "try_exists(",
    "OpenOptions",
];

/// Why a place that opens a file is not one a document names.
#[derive(Clone, Copy)]
enum Why {
    /// A command-line argument, the operator's own deployment descriptor or record: the operator
    /// chose the file, a document did not.
    Operator,
    /// What the generator itself wrote, read back or cleaned.
    Output,
    /// The generator's own templates, schemas, toolchain and program.
    Installation,
    /// The checkout the operator works in: its conformance suite, its git metadata.
    Repository,
}

impl Why {
    fn words(self) -> &'static str {
        match self {
            Why::Operator => "a file the operator names, and not a document",
            Why::Output => "the generator's own output tree",
            Why::Installation => "the generator's own templates, schemas, toolchain and program",
            Why::Repository => "the checkout the operator works in",
        }
    }
}

/// The functions of one source that open a file, and how many opens each has.
type Functions = &'static [(&'static str, usize)];

/// The places that open a file without `confine`: the source, the function the open is in as the
/// scan attributes it (the nearest `fn` above it), how many opens that function has, and why.
const NOT_A_DOCUMENTS: &[(&str, Why, Functions)] = &[
    (
        "src/acceptance_record.rs",
        Why::Operator,
        &[
            ("succeeding", 1),
            ("recheck", 3),
            ("design_inputs", 1),
            ("file_sha256", 1),
        ],
    ),
    ("src/authoring_profile.rs", Why::Operator, &[("load", 1)]),
    (
        "src/commit_stamp.rs",
        Why::Repository,
        &[
            ("head_watch_paths", 2),
            ("files_backend_ref_paths", 2),
            ("symbolic_target", 1),
        ],
    ),
    (
        "src/conformance.rs",
        Why::Repository,
        &[
            ("document_exists", 1),
            ("read_document", 1),
            ("load", 1),
            ("render_harness", 2),
        ],
    ),
    (
        "src/formatter.rs",
        Why::Installation,
        &[("with_locator", 1)],
    ),
    (
        "src/generator.rs",
        Why::Installation,
        &[
            ("load_templates", 2),
            ("shared_macro_dir", 1),
            ("collect", 3),
        ],
    ),
    (
        "src/generator_witness.rs",
        Why::Installation,
        &[("walk", 2), ("hash_file", 1)],
    ),
    // The other machines of a deployment, from `deploy.yaml`.
    (
        "src/lib.rs",
        Why::Operator,
        &[
            ("validate_scxml_invoke_target_exclusivity", 1),
            ("collect_scxml_remote_peers", 1),
        ],
    ),
    (
        "src/lib.rs",
        Why::Installation,
        &[("find_template_base", 3)],
    ),
    ("src/requirement_manifest.rs", Why::Operator, &[("load", 1)]),
    ("src/requirement_sidecar.rs", Why::Operator, &[("load", 1)]),
    ("src/scenario_judge.rs", Why::Operator, &[("load", 1)]),
    (
        "src/scenario_set.rs",
        Why::Operator,
        &[("load_with_digest", 1)],
    ),
    (
        "src/toolchain.rs",
        Why::Installation,
        &[
            ("is_executable_file", 3),
            ("family_members_in", 1),
            ("discover_versioned_bin_dirs", 1),
        ],
    ),
    (
        "src/w3c_dist_manifest.rs",
        Why::Repository,
        &[("parse_file", 1)],
    ),
    (
        "src/w3c_registry.rs",
        Why::Repository,
        &[("load", 1), ("check_aot_header_briefs", 1)],
    ),
    // The command-line program: what it is given to read, what it writes, and where it is run.
    (
        "src/bin/sce_codegen.rs",
        Why::Operator,
        &[
            ("emit_orchestrate_asts", 1),
            ("load_deploy_config", 1),
            ("forge_document_positions", 1),
            ("cmd_check", 1),
            ("cmd_generate", 1),
            ("cmd_fix_scxml_name", 1),
            ("cmd_read_metadata", 1),
            ("cmd_manifest", 1),
            ("read_review_input", 1),
            ("file_documents", 1),
            ("cmd_acceptance_check", 1),
            ("cmd_acceptance_impact", 1),
            ("cmd_acceptance_delta", 1),
            ("cmd_scenarios", 1),
            ("cmd_unresolved", 1),
            ("cmd_expand", 1),
            ("addr2sce_load_symbol_table", 1),
            ("load_sourcemap", 1),
        ],
    ),
    (
        "src/bin/sce_codegen.rs",
        Why::Output,
        &[
            ("try_emit_generated", 1),
            ("remove_generated", 2),
            ("read_generated", 1),
            ("finish_generated_output", 1),
            ("cmd_orchestrate", 1),
            ("clean", 8),
            ("clean_stale", 5),
            ("cmd_verify", 1),
            ("walk", 2),
        ],
    ),
    (
        "src/bin/sce_codegen.rs",
        Why::Installation,
        &[("write_depfile", 1)],
    ),
    (
        "src/bin/sce_codegen.rs",
        Why::Repository,
        &[
            ("find_project_root", 3),
            ("read_metadata", 2),
            ("find_scxml", 1),
            ("cmd_generate_integration", 3),
            ("validates", 1),
        ],
    ),
    (
        "src/forge/drift.rs",
        Why::Operator,
        &[
            ("collect_within", 2),
            ("walk", 2),
            ("read_to_string", 1),
            ("observe", 1),
            ("walk_filtered_recursive", 5),
        ],
    ),
    (
        "src/forge/manifest.rs",
        Why::Operator,
        &[("build_manifest", 1), ("collect_scxml_files", 1)],
    ),
    // `verify` hands this the algorithm document the operator names on the command line.
    (
        "src/forge/static_js.rs",
        Why::Operator,
        &[("lower_algorithm_document", 1)],
    ),
    (
        "src/forge/target_plugin.rs",
        Why::Operator,
        &[("parse_target_plugin_yaml", 1)],
    ),
    (
        "src/forge/xsd_validator.rs",
        Why::Installation,
        &[("find_schema_path", 3)],
    ),
    (
        "src/mesh/codegen.rs",
        Why::Installation,
        &[("generate_cpp_mesh", 2), ("generate_host_mesh", 2)],
    ),
    ("src/mesh/deploy.rs", Why::Operator, &[("parse_deploy", 1)]),
    // The `source:` of a machine in the operator's `deploy.yaml`.
    (
        "src/mesh/topology.rs",
        Why::Operator,
        &[("parse_machine_source", 1)],
    ),
    ("src/mesh/vsomeip_config.rs", Why::Operator, &[("load", 1)]),
];

/// The rule itself: the one source that opens a file directly on purpose.
const THE_RULE: &str = "src/confine.rs";

// ---- reading the sources --------------------------------------------------------------------

/// `source` with its comments, the contents of its strings and of its characters taken out, so that
/// what is left is what the compiler sees as code. Lines are kept, so that a line number is the
/// source's.
fn code_of(source: &str) -> String {
    let bytes = source.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    let at = |i: usize| bytes.get(i).copied().unwrap_or(0);
    let is_word = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'/' && at(i + 1) == b'/' {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
        } else if b == b'/' && at(i + 1) == b'*' {
            let mut depth = 1;
            i += 2;
            while i < bytes.len() && depth > 0 {
                if bytes[i] == b'/' && at(i + 1) == b'*' {
                    depth += 1;
                    i += 2;
                } else if bytes[i] == b'*' && at(i + 1) == b'/' {
                    depth -= 1;
                    i += 2;
                } else {
                    if bytes[i] == b'\n' {
                        out.push(b'\n');
                    }
                    i += 1;
                }
            }
        } else if b == b'r' && (i == 0 || !is_word(bytes[i - 1])) && {
            let mut j = i + 1;
            while at(j) == b'#' {
                j += 1;
            }
            at(j) == b'"'
        } {
            // A raw string: `r"..."`, `r#"..."#`.
            let mut hashes = 0;
            let mut j = i + 1;
            while at(j) == b'#' {
                hashes += 1;
                j += 1;
            }
            j += 1;
            loop {
                if j >= bytes.len() {
                    break;
                }
                if bytes[j] == b'"' && (0..hashes).all(|k| at(j + 1 + k) == b'#') {
                    j += 1 + hashes;
                    break;
                }
                if bytes[j] == b'\n' {
                    out.push(b'\n');
                }
                j += 1;
            }
            out.extend_from_slice(b"\"\"");
            i = j;
        } else if b == b'"' {
            i += 1;
            while i < bytes.len() && bytes[i] != b'"' {
                if bytes[i] == b'\\' {
                    i += 1;
                }
                if at(i) == b'\n' {
                    out.push(b'\n');
                }
                i += 1;
            }
            i += 1;
            out.extend_from_slice(b"\"\"");
        } else if b == b'\'' {
            // A character literal (`'x'`, `'\n'`, one of several bytes), whose quote must not be
            // taken for the start of a string, or a lifetime (`'a`), which is code.
            if at(i + 1) == b'\\' {
                let mut j = i + 2;
                while j < bytes.len() && bytes[j] != b'\'' {
                    j += 1;
                }
                out.extend_from_slice(b"''");
                i = j + 1;
            } else {
                let width = match at(i + 1) {
                    0..=0x7f => 1,
                    0xc0..=0xdf => 2,
                    0xe0..=0xef => 3,
                    _ => 4,
                };
                if at(i + 1) != b'\'' && at(i + 1 + width) == b'\'' {
                    out.extend_from_slice(b"''");
                    i += 2 + width;
                } else {
                    out.push(b);
                    i += 1;
                }
            }
        } else {
            out.push(b);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// The lines of `code` before its tests, which have their own files to open: the first
/// `#[cfg(test)]` at the left margin that is followed by a module with a body.
fn before_its_tests(code: &str) -> Vec<&str> {
    let lines: Vec<&str> = code.lines().collect();
    for (at, line) in lines.iter().enumerate() {
        if line.trim_end() != "#[cfg(test)]" {
            continue;
        }
        let next = lines[at + 1..]
            .iter()
            .map(|l| l.trim())
            .find(|l| !l.is_empty());
        let is_module_with_body = next.is_some_and(|l| {
            let l = l
                .strip_prefix("pub(crate) ")
                .or_else(|| l.strip_prefix("pub "))
                .unwrap_or(l);
            l.starts_with("mod ") && l.trim_end().ends_with('{')
        });
        if is_module_with_body {
            return lines[..at].to_vec();
        }
    }
    lines
}

/// One place that opens a file, and the function the scan finds it in.
#[derive(Debug)]
struct Open {
    line: usize,
    function: String,
    text: String,
}

/// The name after the first `fn ` on a line, when there is one.
fn function_on(line: &str) -> Option<String> {
    let mut from = 0;
    while let Some(found) = line[from..].find("fn ") {
        let at = from + found;
        let before_ok = at == 0
            || !line.as_bytes()[at - 1].is_ascii_alphanumeric() && line.as_bytes()[at - 1] != b'_';
        if before_ok {
            let name: String = line[at + 3..]
                .trim_start()
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if !name.is_empty() {
                return Some(name);
            }
        }
        from = at + 3;
    }
    None
}

/// Every open in `source`, as `before_its_tests` of its code finds it.
fn opens_in(source: &str) -> Vec<Open> {
    let code = code_of(source);
    let mut function = String::from("<top>");
    let mut found = Vec::new();
    for (row, line) in before_its_tests(&code).into_iter().enumerate() {
        if let Some(name) = function_on(line) {
            function = name;
        }
        if RAW.iter().any(|raw| line.contains(raw)) {
            found.push(Open {
                line: row + 1,
                function: function.clone(),
                text: line.trim().to_string(),
            });
        }
    }
    found
}

/// The sources of the generator, as `src/...` and their text.
fn sources() -> Vec<(String, String)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut found = Vec::new();
    let mut pending: Vec<PathBuf> = vec![root.join("src")];
    while let Some(folder) = pending.pop() {
        for entry in fs::read_dir(&folder)
            .unwrap_or_else(|e| panic!("{} cannot be read: {e}", folder.display()))
            .flatten()
        {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|e| e == "rs") {
                let name = path
                    .strip_prefix(root)
                    .unwrap()
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy().into_owned())
                    .collect::<Vec<_>>()
                    .join("/");
                found.push((name, fs::read_to_string(&path).unwrap()));
            }
        }
    }
    found.sort();
    found
}

// ---- the rule -------------------------------------------------------------------------------

#[test]
fn a_place_that_opens_a_file_goes_through_confine_unless_it_is_listed_as_not_a_documents() {
    let mut unlisted = Vec::new();
    // The opens the table covers, by (source, function): how many were found.
    let mut found: std::collections::BTreeMap<(String, String), usize> = Default::default();

    for (name, source) in sources() {
        if name == THE_RULE {
            continue;
        }
        for open in opens_in(&source) {
            let listed = NOT_A_DOCUMENTS.iter().any(|(file, _, functions)| {
                *file == name && functions.iter().any(|(f, _)| *f == open.function)
            });
            if listed {
                *found
                    .entry((name.clone(), open.function.clone()))
                    .or_default() += 1;
            } else {
                unlisted.push(format!(
                    "{name}:{} in `{}`: {}",
                    open.line, open.function, open.text
                ));
            }
        }
    }

    assert!(
        unlisted.is_empty(),
        "a file is opened without `confine`, so a document that names it is not held to the \
         folder a caller confined the generator to. Open it with `confine::exists` / \
         `confine::read` / `confine::read_to_string`, or, when it is no file of a document's, \
         list it in this test with the function it is in and why:\n{}",
        unlisted.join("\n")
    );

    let mut wrong = Vec::new();
    for (file, why, functions) in NOT_A_DOCUMENTS {
        for (function, count) in *functions {
            let seen = found
                .get(&((*file).to_string(), (*function).to_string()))
                .copied()
                .unwrap_or(0);
            // A function listed under two reasons of one source would be counted for both; a
            // source lists a function once.
            if seen != *count {
                wrong.push(format!(
                    "{file}: `{function}` ({}) is listed with {count} open(s) and has {seen}",
                    why.words()
                ));
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "listed as a place that opens a file without `confine`, and that is no longer true of it: \
         correct the list so that it is no longer than what is true, and look at an open that \
         was added to a function that is listed:\n{}",
        wrong.join("\n")
    );
}

#[test]
fn every_source_of_the_generator_is_read_and_the_ones_that_read_documents_are_among_them() {
    // A scan that found no source, or a renamed one, would pass on what it does not find.
    let all = sources();
    let names: Vec<&str> = all.iter().map(|(name, _)| name.as_str()).collect();
    for needed in [
        "src/lib.rs",
        "src/parser.rs",
        "src/resolve.rs",
        "src/xinclude.rs",
        "src/template.rs",
        "src/filters.rs",
        "src/forge/import_source.rs",
        "src/forge/static_js.rs",
        "src/bin/sce_codegen.rs",
        THE_RULE,
    ] {
        assert!(
            names.contains(&needed),
            "{needed} is not scanned: {names:?}"
        );
    }
    assert!(
        all.len() > 80,
        "{} sources: the scan is cut short",
        all.len()
    );
    // Each is read to its end: a cut in the wrong place would leave a few lines.
    for (name, source) in &all {
        let before = before_its_tests(&code_of(source)).len();
        assert!(
            before > 5 || source.lines().count() < 40,
            "{name}: {before} lines are read of {}",
            source.lines().count()
        );
    }
    // And the table names only sources that are there.
    for (file, _, _) in NOT_A_DOCUMENTS {
        assert!(names.contains(file), "{file} is listed and is not a source");
    }
}

// ---- the scan -------------------------------------------------------------------------------

#[test]
fn the_scan_sees_a_code_open_and_not_one_in_a_comment_a_string_or_a_test() {
    let source = r##"
fn code() {
    let a = std::fs::read_to_string(p);
}
// fn a_comment() { std::fs::read(p); }
/* fn nested() { /* inner */ std::fs::read(p); } */
fn text() {
    let s = "std::fs::read(p) and .exists()";
    let t = r#"File::open("x")"#;
}
#[cfg(test)]
mod tests {
    fn only_a_test() { std::fs::read(p); }
}
"##;

    let found: Vec<(String, usize)> = opens_in(source)
        .into_iter()
        .map(|open| (open.function, open.line))
        .collect();

    assert_eq!(found, vec![("code".to_string(), 3)]);
}

#[test]
fn a_quote_in_a_character_does_not_open_a_string_and_a_lifetime_is_code() {
    // The `'"'` would otherwise swallow the lines to the next quote and hide the open between.
    let source = "
fn a<'x>(s: &'x str) -> bool {
    let q = '\"';
    let other = '\u{d55c}';
    std::fs::read(s).is_ok() && s.contains(q) && other != 'a'
}
";

    let found = opens_in(source);

    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].function, "a");
}

#[test]
fn a_test_module_without_a_body_does_not_end_the_part_that_is_read() {
    // `lib.rs` declares test-only modules near its top: `#[cfg(test)] mod name;`.
    let source = "
#[cfg(test)]
mod commit_stamp;
fn after() { std::fs::read(p); }
#[cfg(test)]
mod tests {
    fn t() { std::fs::read(p); }
}
";

    let found = opens_in(source);

    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].function, "after");
}

#[test]
fn an_open_is_attributed_to_the_nearest_function_above_it() {
    let source = "
fn outer() {
    fn inner() { std::fs::read(p); }
    inner();
}
pub(crate) fn another() -> bool {
    p.exists()
}
";

    let found: Vec<String> = opens_in(source).into_iter().map(|o| o.function).collect();

    assert_eq!(found, vec!["inner".to_string(), "another".to_string()]);
}
