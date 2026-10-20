// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A host that only simulates a machine's duties is refused where the machine is
//! built, in all six backends (docs/adr/0038, decision 3) — and a machine of
//! notices is built as it always was.
//!
//! Each backend's own suite drives a real host and shows the refusal happen. This
//! file asks the same questions of all six emitters from one place, for the reason
//! `native_action_backend_parity.rs` gives: a backend that stops emitting one of
//! them fails HERE, by name, rather than in whichever language's suite runs first.
//!
//! The two documents are the shared fixture `static_host_duty` (a duty released on
//! exit, and a notice) and `static_host_call` (notices only). The first must carry
//! the realization member, the refusal and the way to accept a simulated host on
//! purpose; the second must carry none of the three, which is also what keeps every
//! machine of notices byte-identical to what it was.

use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::generator::Language;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

const DUTY: &str = "sce-build/tests/fixtures/static_datamodel/static_host_duty.scxml";
const NOTICES: &str = "sce-build/tests/fixtures/static_datamodel/static_host_call.scxml";

/// Everything `sce-codegen generate` wrote for `lang`, concatenated: the split of a
/// machine into header and body is a per-language layout detail, and the question
/// asked here is whether the backend emitted the construct at all.
///
/// `tag` is unique per `#[test]`: the tests of this binary run as concurrent
/// threads, and a shared scratch directory is emptied under another's reads.
fn generate(fixture: &str, lang: &str, tag: &str) -> String {
    let out = repo_root().join("target").join(tag).join(lang);
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).expect("scratch dir");
    let result = Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
        .args([
            "generate",
            fixture,
            "-l",
            lang,
            "-o",
            out.to_str().expect("utf-8 path"),
            "--no-format",
        ])
        .current_dir(repo_root())
        .output()
        .expect("sce-codegen runs");
    assert!(
        result.status.success(),
        "`sce-codegen generate -l {lang}` refused {fixture}\nstderr:\n{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let mut text = String::new();
    let mut names: Vec<PathBuf> = std::fs::read_dir(&out)
        .expect("the output directory")
        .map(|entry| entry.expect("a directory entry").path())
        .filter(|path| path.is_file())
        .collect();
    names.sort();
    for path in names {
        text.push_str(&std::fs::read_to_string(&path).unwrap_or_default());
        text.push('\n');
    }
    text
}

/// What one backend must show for a machine that states a duty.
struct Duty {
    lang: &'static str,
    /// The realization member of the host interface, as that language spells it.
    member: &'static str,
    /// The name the machine is built through for a host that only simulates, on
    /// purpose.
    opt_in: &'static str,
    /// Where the generated recorder says it simulates, when the backend has one.
    recorder_simulates: Option<&'static str>,
}

fn duty_backends() -> Vec<Duty> {
    vec![
        Duty {
            lang: "rust",
            member: "const REALIZATION: ::sce_rust_runtime::ActionRealization;",
            opt_in: "pub fn new_simulated(",
            recorder_simulates: Some("::sce_rust_runtime::ActionRealization::Simulates"),
        },
        Duty {
            lang: "go",
            member: "Realization() sce.ActionRealization",
            opt_in: "NewSimulatedStaticHostDutyPolicy",
            recorder_simulates: None,
        },
        Duty {
            lang: "kotlin",
            member: "val realization: ActionRealization",
            opt_in: "acceptSimulatedHost",
            recorder_simulates: Some(
                "override val realization: ActionRealization = ActionRealization.SIMULATES",
            ),
        },
        Duty {
            lang: "python",
            member: "realization: \"ActionRealization\"",
            opt_in: "accept_simulated_host",
            recorder_simulates: None,
        },
        Duty {
            lang: "cpp",
            member: "virtual SCE::ActionRealization realization() const = 0;",
            opt_in: "imulated",
            recorder_simulates: None,
        },
        Duty {
            lang: "c11",
            member: "sce_action_realization_t realization;",
            opt_in: "init_with_simulated_actions",
            recorder_simulates: None,
        },
    ]
}

#[test]
fn every_backend_declares_what_a_host_does_with_a_duty() {
    for d in duty_backends() {
        let text = generate(DUTY, d.lang, "host_duty_member");
        assert!(
            text.contains(d.member),
            "{}: the host interface of a machine that states a duty carries no \
             realization member. Without it a host cannot say beside its code whether \
             it performs the duty, and the machine has nothing to refuse a simulating \
             host by. Expected to find:\n  {}",
            d.lang,
            d.member
        );
    }
}

#[test]
fn every_backend_gives_a_way_to_accept_a_simulated_host_on_purpose() {
    for d in duty_backends() {
        let text = generate(DUTY, d.lang, "host_duty_opt_in");
        assert!(
            text.contains(d.opt_in),
            "{}: there is no named way to build the machine on a host that only \
             simulates its duties. A harness that drives the machine with a recorder \
             has nothing to say so with, and the refusal could not be lifted on \
             purpose. Expected to find:\n  {}",
            d.lang,
            d.opt_in
        );
    }
}

#[test]
fn a_generated_recorder_declares_that_it_simulates() {
    for d in duty_backends() {
        let Some(spelling) = d.recorder_simulates else {
            continue;
        };
        let text = generate(DUTY, d.lang, "host_duty_recorder");
        assert!(
            text.contains(spelling),
            "{}: the generated recorder does not declare that it simulates, so a \
             machine with a duty would take it for a host that performs. Expected to \
             find:\n  {}",
            d.lang,
            spelling
        );
    }
}

#[test]
fn rust_refuses_a_simulating_host_while_it_compiles() {
    let text = generate(DUTY, "rust", "host_duty_rust_refusal");
    assert!(
        text.contains("A::REALIZATION.is_performing()"),
        "the Rust machine does not test the host's realization:\n{text}"
    );
    assert!(
        text.contains("let () = Self::HOST_PERFORMS_DUTIES;"),
        "`new` does not evaluate the refusal, so a host that simulates would build \
         the machine without a word:\n{text}"
    );
    let simulated = text
        .split("pub fn new_simulated(")
        .nth(1)
        .and_then(|rest| rest.split("pub fn ").next())
        .expect("new_simulated is emitted");
    assert!(
        !simulated.contains("HOST_PERFORMS_DUTIES"),
        "`new_simulated` is the caller's statement that it means to, and must not \
         refuse:\n{simulated}"
    );
}

#[test]
fn a_machine_of_notices_carries_no_realization_refusal_or_opt_in() {
    for lang in Language::ALL
        .iter()
        .map(|language| language.canonical_name())
    {
        let text = generate(NOTICES, lang, "host_duty_notices_only");
        for word in ["ealization", "imulated", "REALIZATION"] {
            assert!(
                !text.contains(word),
                "{lang}: a machine that states no duty mentions `{word}`. Its host \
                 interface and its construction are what they were before duties \
                 existed, so that every host written for it still compiles."
            );
        }
    }
}

#[test]
fn the_manifest_names_each_duty_site_and_leaves_the_notice_out() {
    let result = Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
        .args(["check", DUTY, "-l", "rust"])
        .current_dir(repo_root())
        .output()
        .expect("sce-codegen runs");
    assert!(
        result.status.success(),
        "check refused the duty fixture:\n{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let manifest: serde_json::Value =
        serde_json::from_slice(&result.stdout).expect("the manifest is one JSON line");
    let duties = manifest["host_duties"]
        .as_array()
        .expect("a machine that states a duty publishes host_duties");
    let sites: Vec<(&str, &str, &str)> = duties
        .iter()
        .map(|d| {
            (
                d["action"].as_str().expect("action"),
                d["state"].as_str().expect("state"),
                d["on"].as_str().expect("on"),
            )
        })
        .collect();
    assert_eq!(
        sites,
        vec![
            ("start_keepalive", "engaged", "entry"),
            ("stop_keepalive", "engaged", "exit")
        ],
        "the notice `note_engaged` is not a duty, and each duty is at its state"
    );
    assert_eq!(duties[0]["released_by"], "stop_keepalive");
    assert!(duties[1].get("released_by").is_none());
}

#[test]
fn a_machine_of_notices_publishes_no_host_duties() {
    let result = Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
        .args(["check", NOTICES, "-l", "rust"])
        .current_dir(repo_root())
        .output()
        .expect("sce-codegen runs");
    assert!(result.status.success());
    let manifest: serde_json::Value =
        serde_json::from_slice(&result.stdout).expect("the manifest is one JSON line");
    assert!(
        manifest.get("host_duties").is_none(),
        "host_duties is omitted, not empty, when there is no duty"
    );
}
