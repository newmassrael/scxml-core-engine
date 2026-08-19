// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! Every generator template that raises an `error.*` event says what failed.
//!
//! W3C SCXML 3.12.2 closes with "platforms MAY include additional information
//! about the nature of the error in the 'data' field". It is a MAY, so a
//! backend that carries nothing is still conformant — and that is exactly why
//! this drifted rather than failing: nothing was ever going to notice.
//!
//! MEASURED 2026-08-19, before the repair. The C++ Interpreter passed the
//! failing construct at every raise site it had. The C++ AOT backend passed one
//! at 8 sites of 37. The other five generated backends passed one at ZERO of
//! 168. So a document that answered `error.execution` was handed `undefined` on
//! five engines, a real string on one, and the full text on the Interpreter —
//! three different answers to the same question, from one repository.
//!
//! ## Why a gate and not just the repair
//!
//! The repair was 178 hand edits across six template trees. Three defects were
//! introduced during it and caught afterwards — a short-circuit that skipped
//! transition selection, a duplicated raise, and a comment block that ate the
//! declaration after it. A ratio that drifted to 8/205 once will drift again
//! the next time someone adds a raise site, and the next author has no way to
//! know the convention exists.
//!
//! Each backend's entry point now takes the message as a required argument, so
//! most of this is enforced by its own compiler. This catches what a compiler
//! cannot: a call that passes an EMPTY message, and a site that goes back to
//! constructing the carrier by hand instead of calling the entry point.

use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build sits directly under the repository root")
        .to_path_buf()
}

/// Every `.jinja2` under `tools/codegen/templates`, recursively.
fn template_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.join("tools/codegen/templates")];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "jinja2") {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

fn rel(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .into_owned()
}

/// The per-language spelling of "raise a platform error", and the shape that
/// means the message is missing.
///
/// Each backend routes every `error.*` raise through one entry point; the
/// second field is what a raise looks like when it bypasses that entry point
/// and builds the carrier itself, which is how all six backends started.
struct Backend {
    /// Directory under `tools/codegen/templates` this rule applies to. The
    /// empty string is the C++ tree, which lives at the top level.
    dir: &'static str,
    /// What a raise that has ABANDONED the entry point looks like.
    bypass: &'static [&'static str],
    /// Human-readable name of the entry point, for the failure message.
    entry_point: &'static str,
}

const BACKENDS: &[Backend] = &[
    Backend {
        dir: "rust",
        bypass: &[
            "EventWithMetadata::new({{ machine_name }}Event::{{ 'error.execution'",
            "EventWithMetadata::new({{ machine_name }}Event::{{ 'error.communication'",
        ],
        entry_point: "EventWithMetadata::platform_error(event, message)",
    },
    Backend {
        dir: "go",
        bypass: &[
            "NewPlatformEvent({{ machine_name }}Event{{ 'error.execution'",
            "NewPlatformEvent({{ machine_name }}Event{{ 'error.communication'",
        ],
        entry_point: "sce.NewPlatformError(event, message)",
    },
    Backend {
        dir: "kotlin",
        bypass: &["raiseInternal({{ machine_name }}Event.Error."],
        entry_point: "raisePlatformError(event, message)",
    },
    Backend {
        dir: "python",
        bypass: &["_raise_error_execution(engine)"],
        entry_point: "self._raise_error_execution(engine, message)",
    },
    Backend {
        dir: "c",
        bypass: &["_err_evt.event = "],
        entry_point: "<machine>_raise_platform_error(sm, event, message)",
    },
    Backend {
        // The C++ tree is the top level plus `actions/`; `Event::Error_x)`
        // with nothing before the paren is the no-message constructor.
        dir: "",
        bypass: &[
            "EventWithMetadata(Event::Error_execution)",
            "EventWithMetadata(Event::Error_communication)",
        ],
        entry_point: "EventWithMetadata(event, message)",
    },
];

/// Which backend rule owns a template path.
fn backend_for<'a>(rel_path: &str) -> Option<&'a Backend> {
    let inside = rel_path.strip_prefix("tools/codegen/templates/")?;
    // Longest directory prefix wins, so `c/…` does not also match the
    // top-level C++ rule.
    for backend in BACKENDS {
        if !backend.dir.is_empty() && inside.starts_with(&format!("{}/", backend.dir)) {
            return Some(backend);
        }
    }
    BACKENDS.iter().find(|b| b.dir.is_empty())
}

/// A raise site that builds its own carrier is a raise site that can forget
/// the message — and every one of them did, in five backends out of six.
#[test]
fn every_error_raise_goes_through_its_backend_entry_point() {
    let root = repo_root();
    let mut violations: Vec<String> = Vec::new();

    for path in template_files(&root) {
        let rel_path = rel(&root, &path);
        let Some(backend) = backend_for(&rel_path) else {
            continue;
        };
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        for (n, line) in text.lines().enumerate() {
            for needle in backend.bypass {
                if line.contains(needle) {
                    violations.push(format!(
                        "{rel_path}:{}: raises an error.* event without going through \
                         `{}` — so `_event.data` is empty and the document that \
                         answers this error cannot say what broke",
                        n + 1,
                        backend.entry_point
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "W3C SCXML 3.12.2 lets the `data` field carry what failed, and these raise \
         sites carry nothing:\n  {}\n\nUse the backend's entry point, which takes the \
         message as a required argument.",
        violations.join("\n  ")
    );
}

/// The compiler enforces that a message is PASSED; only a reader enforces that
/// it says something. An empty literal satisfies every signature in the repo.
#[test]
fn no_error_raise_passes_an_empty_message() {
    let root = repo_root();
    let mut violations: Vec<String> = Vec::new();

    for path in template_files(&root) {
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        for (n, line) in text.lines().enumerate() {
            let empty_arg = line.contains(", \"\")") || line.contains(", '')");
            let is_error_raise = line.contains("platform_error")
                || line.contains("PlatformError")
                || line.contains("_raise_error_execution");
            // ⚠ The C11 `_with_send_id` form ends in a sendid, and the plain
            // form forwards to it with `""` in THAT position — which is the
            // correct value there and not a missing message. Measured: this
            // check's first run flagged the forwarding line in
            // `state_machine.h.jinja2`, i.e. the repair's own helper.
            let empty_is_the_send_id = line.contains("_with_send_id");
            if empty_arg && is_error_raise && !empty_is_the_send_id {
                violations.push(format!(
                    "{}:{}: raises an error with an empty message",
                    rel(&root, &path),
                    n + 1
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "an empty message satisfies the signature and tells the reader nothing:\n  {}",
        violations.join("\n  ")
    );
}

/// The rules above are only worth their runtime if they still find the sites
/// they were written against. A typo in a needle turns this whole file into a
/// test that passes by matching nothing — the failure mode the repository has
/// hit before, and the reason every scanner here carries a lower bound.
#[test]
fn the_scan_actually_reaches_every_backend() {
    let root = repo_root();
    let files = template_files(&root);
    assert!(
        files.len() > 100,
        "expected the template tree to hold hundreds of files, found {} — the \
         scan is looking in the wrong place",
        files.len()
    );

    for backend in BACKENDS {
        let matched = files
            .iter()
            .filter(|p| backend_for(&rel(&root, p)).is_some_and(|b| b.dir == backend.dir))
            .count();
        assert!(
            matched > 0,
            "no template file was attributed to the `{}` backend rule, so its \
             bypass patterns guard nothing",
            if backend.dir.is_empty() {
                "c++"
            } else {
                backend.dir
            }
        );
    }

    // And the entry points themselves must exist: a rule that forbids the old
    // shape while the new one is absent would leave no legal way to raise.
    let calls: usize = files
        .iter()
        .filter_map(|p| std::fs::read_to_string(p).ok())
        .map(|t| {
            t.matches("platform_error").count()
                + t.matches("PlatformError").count()
                + t.matches("_raise_error_execution").count()
        })
        .sum();
    assert!(
        calls > 100,
        "expected the six entry points to be called at well over a hundred sites \
         (178 when this landed), found {calls} — either the repair was reverted or \
         the needles no longer match"
    );
}
