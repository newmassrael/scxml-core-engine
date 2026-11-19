// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// docs/adr/0109, D11 (docs/SCE_ACCEPTED_SUBSET.md §2.15, "A declared migration"):
// a document declares the step a state an earlier version saved is carried by, the
// build holds the step to its two shape files where the machine is generated, and
// writes the plan into the machine a restore applies.
//
// Run through `sce-codegen` end to end, as an author meets it: `shape` writes the
// document's own shape file, the earlier version's is the text that version saved
// under, and `generate` plans the step for Rust and C++, refuses it on a backend
// that applies no migration by name (`generate/unsupported-feature`, exit 7), and
// refuses a document whose step does not hold as the document's refusal
// (`validation/migration-*`, exit 3), whichever backend was asked. The unit tests
// beside the judge (`forge::saved_migration`) hold each refusal of each step; this
// holds that the routes an author runs carry them.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/saved_migration")
}

/// The shape the earlier version of `every_step.scxml` saved under: the states it
/// stood in, the invocation it ran, and its variables before the step.
const EARLIER: &str = "sce-saved-state-shape 1\n\
                       state idle atomic -\n\
                       state legacy atomic -\n\
                       state waiting atomic -\n\
                       state done final -\n\
                       invoke poller waiting\n\
                       variable count uint32 -\n\
                       variable scratch uint8 -\n\
                       variable total int32 -\n\
                       variable active uint8 -\n";

static SCRATCH_ID: AtomicU64 = AtomicU64::new(0);

/// A directory holding the fixtures, with `document` (the text of
/// `every_step.scxml`, perhaps edited) in place of the committed one.
fn scratch(document: &str) -> PathBuf {
    let id = SCRATCH_ID.fetch_add(1, Ordering::SeqCst);
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("declared-migration-{}-{id}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("shapes")).expect("create scratch dir");
    std::fs::write(dir.join("every_step.scxml"), document).expect("write the document");
    std::fs::copy(
        fixtures().join("algorithm_flag_of.scxml"),
        dir.join("algorithm_flag_of.scxml"),
    )
    .expect("copy the algorithm");
    dir
}

fn committed() -> String {
    std::fs::read_to_string(fixtures().join("every_step.scxml")).expect("the fixture")
}

fn codegen(dir: &Path, args: &[&str]) -> Output {
    Command::new(sce_codegen_bin())
        .args(args)
        .current_dir(dir)
        .output()
        .expect("spawn sce-codegen")
}

/// `sce-codegen shape` of the document in `dir`, written to its `shapes/2.shape`.
fn write_own_shape(dir: &Path) {
    let run = codegen(
        dir,
        &[
            "shape",
            "every_step.scxml",
            "-o",
            "shapes/2.shape",
            "--error-format=json",
        ],
    );
    assert!(
        run.status.success(),
        "shape: {}",
        String::from_utf8_lossy(&run.stderr)
    );
}

/// What `generate -l <language>` wrote for the document in `dir`, joined, its exit
/// status and its diagnostics.
fn generate(dir: &Path, language: &str) -> (Option<i32>, String, String) {
    let out = dir.join(format!("out-{language}"));
    let mut args = vec![
        "generate",
        "every_step.scxml",
        "-l",
        language,
        "-o",
        out.to_str().unwrap(),
        "--error-format=json",
    ];
    // The C++ text as the templates write it, which the assertions quote.
    if language == "cpp" {
        args.push("--no-format");
    }
    let run = codegen(dir, &args);
    let mut written = String::new();
    if let Ok(entries) = std::fs::read_dir(&out) {
        for entry in entries {
            let path = entry.expect("an entry").path();
            if let Ok(text) = std::fs::read_to_string(&path) {
                written.push_str(&text);
                written.push('\n');
            }
        }
    }
    (
        run.status.code(),
        written,
        String::from_utf8_lossy(&run.stderr).into_owned(),
    )
}

#[test]
fn the_shape_subcommand_writes_the_text_a_saved_state_names() {
    let dir = scratch(&committed());
    write_own_shape(&dir);
    let text = std::fs::read_to_string(dir.join("shapes/2.shape")).expect("written");
    assert_eq!(
        text,
        "sce-saved-state-shape 1\n\
         state idle atomic -\n\
         state pending atomic -\n\
         state done final -\n\
         variable attempts uint32 -\n\
         variable total int64 -\n\
         variable active bool -\n\
         variable ceiling uint32 -\n",
        "every state in document order, then every variable in declaration order"
    );
}

#[test]
fn a_step_of_every_kind_is_planned_into_the_rust_and_cpp_machines() {
    let dir = scratch(&committed());
    std::fs::write(dir.join("shapes/1.shape"), EARLIER).expect("write the earlier shape");
    write_own_shape(&dir);

    let (status, rust, diagnostics) = generate(&dir, "rust");
    assert_eq!(status, Some(0), "{diagnostics}");
    for planned in [
        r#"from: "waiting", to: "pending""#,
        r#"from: "legacy", to: "idle""#,
        r#"from: "count", to: "attempts""#,
        r#"from: "tock", to: "tick""#,
        r#"dropped_invokes: &["poller", ]"#,
        r#"dropped_data: &["scratch", ]"#,
        r#"widened_to_text: &["total", ]"#,
        r#"id: "ceiling", value: "16""#,
        r#"data: "active", convert: sce_migration_convert_0_0"#,
        "fn sce_migration_convert_0_0(",
    ] {
        assert!(
            rust.contains(planned),
            "the Rust plan holds {planned}:\n{rust}"
        );
    }

    let (status, cpp, diagnostics) = generate(&dir, "cpp");
    assert_eq!(status, Some(0), "{diagnostics}");
    for planned in [
        r#"step.states.push_back(::SCE::Saved::Renamed{"waiting", "pending"});"#,
        r#"step.droppedInvokes.push_back("poller");"#,
        r#"step.widenedToText.push_back("total");"#,
        r#"step.addedData.push_back(::SCE::Saved::AddedData{"ceiling", "16"});"#,
        "::SCE::Saved::savedWriteBool(",
        "::SCE::Saved::savedReadUint8(old, what)",
    ] {
        assert!(
            cpp.contains(planned),
            "the C++ plan holds {planned}:\n{cpp}"
        );
    }
}

#[test]
fn a_backend_that_applies_no_migration_refuses_the_document_by_name() {
    let dir = scratch(&committed());
    std::fs::write(dir.join("shapes/1.shape"), EARLIER).expect("write the earlier shape");
    write_own_shape(&dir);
    for language in ["kotlin", "python"] {
        let (status, _, diagnostics) = generate(&dir, language);
        assert_eq!(status, Some(7), "{language}: {diagnostics}");
        assert!(
            diagnostics.contains("generate/unsupported-feature")
                && diagnostics.contains("sce:migration"),
            "{language}: {diagnostics}"
        );
    }
}

#[test]
fn a_step_that_does_not_hold_is_the_documents_refusal_on_every_route() {
    // No shape file at all.
    let dir = scratch(&committed());
    let (status, _, diagnostics) = generate(&dir, "rust");
    assert_eq!(status, Some(3), "{diagnostics}");
    assert!(
        diagnostics.contains("validation/migration-shape-unreadable"),
        "{diagnostics}"
    );

    // The earlier shape where the document's own belongs: the chain does not end
    // at this document.
    let dir = scratch(&committed());
    std::fs::write(dir.join("shapes/1.shape"), EARLIER).expect("write");
    std::fs::write(
        dir.join("shapes/2.shape"),
        EARLIER.replace("legacy", "older"),
    )
    .expect("write");
    let (status, _, diagnostics) = generate(&dir, "cpp");
    assert_eq!(status, Some(3), "{diagnostics}");
    assert!(
        diagnostics.contains("validation/migration-chain-broken"),
        "{diagnostics}"
    );

    // The step without its map: `legacy` is accounted for by nothing.
    let dir = scratch(&committed().replace(r#"<sce:map-state from="legacy" to="idle"/>"#, ""));
    std::fs::write(dir.join("shapes/1.shape"), EARLIER).expect("write");
    write_own_shape(&dir);
    let (status, _, diagnostics) = generate(&dir, "rust");
    assert_eq!(status, Some(3), "{diagnostics}");
    assert!(
        diagnostics.contains("validation/migration-not-total") && diagnostics.contains("legacy"),
        "{diagnostics}"
    );

    // `total` narrowed in place of widened, with nothing to convert it.
    let narrowed = committed().replace(
        r#"<data id="total" sce:type="int64""#,
        r#"<data id="total" sce:type="int16""#,
    );
    let dir = scratch(&narrowed);
    std::fs::write(dir.join("shapes/1.shape"), EARLIER).expect("write");
    write_own_shape(&dir);
    let (status, _, diagnostics) = generate(&dir, "rust");
    assert_eq!(status, Some(3), "{diagnostics}");
    assert!(
        diagnostics.contains("validation/migration-type-change"),
        "{diagnostics}"
    );
}
