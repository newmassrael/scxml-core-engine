// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! Every code an engine raises `error.*` for is one `stdlib/engine/error_reason.scxml`
//! declares, and is raised as the event its value range names (docs/adr/0106 #111).
//!
//! The vocabulary is generated into each runtime, so a runtime that names a code
//! the document does not declare fails to compile in that runtime's lane. Three
//! spellings are not held by any compiler, and this file holds them:
//!
//! * the names the generator itself hands `StaticTarget::raise_execution_error`
//!   (`"expressionFailed"`, `"assignmentFailed"`), which are spelled for each
//!   backend only when a machine that raises them is generated;
//! * the code a template writes beside its event: a compiler accepts
//!   `targetUnreachable` raised as `error.execution`, and the value range says
//!   the record would then name the other event;
//! * the document itself: lowerCamel names (the stdlib's spelling), one value
//!   each, every value in 1 to 63 (`error.execution`) or 64 to 127
//!   (`error.communication`).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use sce_build::forge::enum_naming::variant_ident;
use sce_build::generator::Language;

/// The `name` the vocabulary document declares, which every backend spells its
/// type and members from.
const ENUM_NAME: &str = "error_reason";

/// The first value of the communication half.
const COMMUNICATION_FROM: u32 = 64;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build sits directly under the repository root")
        .to_path_buf()
}

/// The vocabulary, read from the document: each declared name and its value.
fn vocabulary() -> BTreeMap<String, u32> {
    let path = repo_root().join("stdlib/engine/error_reason.scxml");
    let text = std::fs::read_to_string(&path).expect("read the vocabulary document");
    let document = roxmltree::Document::parse(&text).expect("the vocabulary is XML");
    let mut codes = BTreeMap::new();
    for node in document
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "variant")
    {
        let name = node.attribute("name").expect("a variant has a name");
        let value: u32 = node
            .attribute("value")
            .expect("a variant has a value")
            .parse()
            .expect("a value is a whole number");
        assert!(
            codes.insert(name.to_string(), value).is_none(),
            "{name} is declared twice"
        );
    }
    codes
}

/// Every file under `dir` (recursively) whose name ends with one of `suffixes`,
/// leaving out generated trees, which are spelled from the document.
fn files_under(dir: &Path, suffixes: &[&str]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(at) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&at) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if path.is_dir() {
                if name != "generated" && name != "__pycache__" {
                    stack.push(path);
                }
            } else if suffixes.iter().any(|s| name.ends_with(s)) {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

#[test]
fn the_vocabulary_is_lower_camel_and_each_value_names_its_event() {
    let codes = vocabulary();
    assert!(
        codes.len() > 30,
        "expected the engine and Mesh codes, found {}: the document was not read",
        codes.len()
    );
    let lower_camel = regex::Regex::new(r"^[a-z][A-Za-z0-9]*$").unwrap();
    let mut values = BTreeSet::new();
    for (name, value) in &codes {
        assert!(lower_camel.is_match(name), "{name} is not lowerCamel");
        assert!(
            (1..=127).contains(value),
            "{name} = {value} is in neither half (1 to 63, 64 to 127)"
        );
        assert!(values.insert(*value), "{name} reuses the value {value}");
    }
    for (name, half) in [
        ("expressionFailed", false),
        ("payloadRefused", false),
        ("sessionTransportUnavailable", false),
        ("targetUnreachable", true),
        ("transportUnavailable", true),
        ("causalViewMismatch", true),
    ] {
        let value = codes
            .get(name)
            .unwrap_or_else(|| panic!("{name} is not declared"));
        assert_eq!(
            *value >= COMMUNICATION_FROM,
            half,
            "{name} = {value} is raised as the wrong event"
        );
    }
}

/// The names the generator hands `raise_execution_error`, read off its source:
/// the first string literal of each call, since `machine` before it is a name.
#[test]
fn every_code_the_generator_raises_is_declared_and_an_execution_code() {
    let codes = vocabulary();
    let path = repo_root().join("sce-build/src/forge/static_lowering.rs");
    let text = std::fs::read_to_string(&path).expect("read the static lowering");
    let call = regex::Regex::new(r#"\.raise_execution_error\(\s*[^"]*?"([^"]*)""#).unwrap();
    let mut seen = 0;
    for found in call.captures_iter(&text) {
        let name = &found[1];
        seen += 1;
        let value = codes.get(name).unwrap_or_else(|| {
            panic!(
                "static_lowering.rs raises error.execution for `{name}`, which \
                 stdlib/engine/error_reason.scxml does not declare"
            )
        });
        assert!(
            *value < COMMUNICATION_FROM,
            "`{name}` = {value} is a communication code raised as error.execution"
        );
    }
    assert!(
        seen >= 3,
        "expected the generator's three raise sites (two expressionFailed, one \
         assignmentFailed), found {seen}: the scan no longer reaches them"
    );
}

/// One backend's spelling of a code, read where its templates and runtime write it.
struct Spelling {
    language: Language,
    /// Captures the member as the backend spells it.
    member: &'static str,
    /// What marks a line as raising `error.communication` in this backend.
    communication: &'static [&'static str],
    /// What marks a line as raising `error.execution` in this backend.
    execution: &'static [&'static str],
    dirs: &'static [&'static str],
    suffixes: &'static [&'static str],
}

const SPELLINGS: &[Spelling] = &[
    Spelling {
        language: Language::Rust,
        member: r"ErrorReason::([A-Z][A-Za-z0-9]*)",
        communication: &["'error.communication'", "\"error.communication\""],
        execution: &["'error.execution'", "\"error.execution\"", "ErrorExecution"],
        dirs: &[
            "tools/codegen/templates/rust",
            "backends/rust/runtime/src",
            "sce-build/src/forge/native_action.rs",
        ],
        suffixes: &[".jinja2", ".rs"],
    },
    Spelling {
        language: Language::Kotlin,
        member: r"ErrorReason\.([A-Z][A-Z0-9_]*)",
        communication: &["Error.Communication"],
        execution: &["Error.Execution"],
        dirs: &[
            "tools/codegen/templates/kotlin",
            "backends/kotlin/runtime/src/commonMain",
            "sce-build/src/forge/native_action.rs",
        ],
        suffixes: &[".jinja2", ".kt", ".rs"],
    },
    Spelling {
        language: Language::Go,
        member: r"errorreason\.(ErrorReason[A-Z][A-Za-z0-9]*)",
        communication: &["'error.communication'"],
        execution: &["'error.execution'", "EventErrorExecution"],
        dirs: &[
            "tools/codegen/templates/go",
            "backends/go/runtime",
            "sce-build/src/forge/native_action.rs",
        ],
        suffixes: &[".jinja2", ".go", ".rs"],
    },
    Spelling {
        language: Language::Python,
        member: r"ErrorReason\.([A-Z][A-Z0-9_]*)",
        communication: &["\"error.communication\""],
        execution: &["\"error.execution\"", "_raise_error_execution("],
        dirs: &[
            "tools/codegen/templates/python",
            "backends/python/runtime/sce_runtime",
            "sce-build/src/forge/native_action.rs",
        ],
        suffixes: &[".jinja2", ".py", ".rs"],
    },
    Spelling {
        language: Language::Swift,
        // A member is written with the leading dot where the parameter's type
        // names it: `reason: .x`, `raiseExecutionError(.x, …)`, a `send_error`
        // macro's second argument `'.x'`, and `sendArgReason = .x`.
        member: r"(?:reason: |raiseExecutionError\(|', '|sendArgReason = )\.([a-z][A-Za-z0-9]*)",
        communication: &["'error.communication'", "errorCommunication"],
        execution: &["'error.execution'", "raiseExecutionError("],
        dirs: &[
            "tools/codegen/templates/swift",
            "backends/swift/runtime/Sources/SceRuntime",
        ],
        suffixes: &[".jinja2", ".swift"],
    },
    Spelling {
        language: Language::Cpp,
        // `ErrorReason::X` wherever a site writes the member (the AOT
        // templates, the static engine, the Interpreter's `raiseError`), and
        // the member a template hands as text: a `send_arg_error` macro's
        // first argument and the two arms of a `<param>`'s choice.
        member: r"(?:ErrorReason::|send_arg_error\('|\{\{ '|if param\.expr else ')([A-Z][A-Za-z0-9]*)",
        communication: &["Event::Error_communication"],
        execution: &["Event::Error_execution", "send_arg_error("],
        // The C++ templates are the top level of the template tree, its
        // `actions/` and the C++ macro; the other languages' directories
        // beside them are judged in their own spelling.
        dirs: &[
            "tools/codegen/templates/datamodel_macros.jinja2",
            "tools/codegen/templates/entry_exit_actions.jinja2",
            "tools/codegen/templates/invoke_methods.jinja2",
            "tools/codegen/templates/scriptengine_helpers.jinja2",
            "tools/codegen/templates/state_machine.jinja2",
            "tools/codegen/templates/utility_methods.jinja2",
            "tools/codegen/templates/actions",
            "tools/codegen/templates/_macros/cpp_invoke_start.jinja2",
            "sce/include/static/StaticExecutionEngine.h",
            "sce/src/runtime",
            "sce/src/events",
            "sce-build/src/forge/native_action.rs",
        ],
        suffixes: &[".jinja2", ".h", ".cpp"],
    },
    Spelling {
        language: Language::C11,
        // A member is a global constant that carries its type's name, so it is
        // read wherever it is written: a raise site's third argument, the
        // two arms of a `<param>`'s or a delay's choice, the door's rewrite of
        // a refused delivery and the native action's guard.
        member: r"\b(ERROR_REASON_[A-Z0-9_]+)\b",
        communication: &["_EVENT_ERROR_COMMUNICATION"],
        execution: &["_EVENT_ERROR_EXECUTION"],
        dirs: &[
            "tools/codegen/templates/c",
            "backends/c/runtime/include/sce/error_data.h",
            "sce-build/src/forge/native_action.rs",
            "sce-build/src/forge/generator.rs",
        ],
        suffixes: &[".jinja2", ".h", ".rs"],
    },
];

#[test]
fn every_code_a_backend_writes_is_declared_and_raised_as_its_event() {
    let codes = vocabulary();
    let root = repo_root();
    let mut violations = Vec::new();
    for spelling in SPELLINGS {
        let declared: BTreeMap<String, u32> = codes
            .iter()
            .map(|(name, value)| (variant_ident(spelling.language, ENUM_NAME, name), *value))
            .collect();
        let member = regex::Regex::new(spelling.member).unwrap();
        let mut seen = 0;
        for dir in spelling.dirs {
            let at = root.join(dir);
            let files = if at.is_file() {
                vec![at]
            } else {
                files_under(&at, spelling.suffixes)
            };
            for path in files {
                let Ok(text) = std::fs::read_to_string(&path) else {
                    continue;
                };
                let shown = path.strip_prefix(&root).unwrap_or(&path).display();
                for (n, line) in text.lines().enumerate() {
                    for found in member.captures_iter(line) {
                        // Rust's `ErrorReason::X` and Kotlin's `ErrorReason.X`
                        // also match a type of the same name in another
                        // backend's text inside native_action.rs; only the
                        // spelling of this backend is judged.
                        let ident = &found[1];
                        let Some(value) = declared.get(ident) else {
                            if spelling.language == Language::Rust
                                || !shown.to_string().ends_with("native_action.rs")
                            {
                                violations.push(format!(
                                    "{shown}:{}: {:?} spells `{ident}`, which \
                                     stdlib/engine/error_reason.scxml does not declare",
                                    n + 1,
                                    spelling.language
                                ));
                            }
                            continue;
                        };
                        seen += 1;
                        let raised_as_communication =
                            spelling.communication.iter().any(|m| line.contains(m));
                        let raised_as_execution =
                            spelling.execution.iter().any(|m| line.contains(m));
                        if raised_as_communication && *value < COMMUNICATION_FROM {
                            violations.push(format!(
                                "{shown}:{}: `{ident}` = {value} raised as error.communication",
                                n + 1
                            ));
                        }
                        if raised_as_execution
                            && !raised_as_communication
                            && *value >= COMMUNICATION_FROM
                        {
                            violations.push(format!(
                                "{shown}:{}: `{ident}` = {value} raised as error.execution",
                                n + 1
                            ));
                        }
                    }
                }
            }
        }
        assert!(
            seen > 20,
            "{:?}: expected the templates and the runtime to name a code at well \
             over twenty sites, found {seen}: the scan no longer reaches them",
            spelling.language
        );
    }
    assert!(
        violations.is_empty(),
        "codes that are not the vocabulary's, or not their event's:\n  {}",
        violations.join("\n  ")
    );
}
