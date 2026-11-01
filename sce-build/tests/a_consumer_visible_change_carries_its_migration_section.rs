// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A push that changes what a downstream consumer compiles against, or what a
//! decode refuses, says how to follow it (docs/adr/0078 §2).
//!
//! `tools/git-hooks/consumer-guide-gate.sh` is sourced by `pre-push` and judges
//! the range a push publishes. A range that removes or rewrites a public
//! declaration of an existing generated golden, of a codec runtime's source or
//! of a mesh library's source, or the expected error of an oracle reject, must
//! also change
//! `docs/SCE_CODEC_CONSUMER_MIGRATION.md`, or carry a bullet
//! `- No consumer-visible change: <why>` in one of its messages.
//!
//! Each case builds the range in a sandbox repository and runs the gate there
//! in a strict shell, as `pre-push` does. The first group holds what refuses,
//! the second what passes, so a gate that refused everything, or nothing,
//! fails one group or the other.

#![cfg(unix)]

mod common;

use std::path::PathBuf;
use std::process::{Command, Output};

struct Sandbox {
    dir: PathBuf,
}

impl Sandbox {
    fn new(label: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("sce-consumer-guide-{label}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create sandbox repository");
        let sandbox = Sandbox { dir };
        sandbox.git(&["init", "-q"]);
        sandbox
    }

    fn git(&self, args: &[&str]) -> String {
        let out = Command::new("git")
            .arg("-C")
            .arg(&self.dir)
            .args([
                "-c",
                "user.name=sandbox",
                "-c",
                "user.email=sandbox@example.invalid",
            ])
            .args(args)
            .output()
            .expect("git runs");
        assert!(
            out.status.success(),
            "git {args:?} failed:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }

    fn write(&self, rel: &str, text: &str) {
        let path = self.dir.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).expect("create parent directory");
        std::fs::write(&path, text).expect("write sandbox file");
    }

    /// Commit `files` with `message` and answer the new commit.
    fn commit(&self, files: &[(&str, &str)], message: &str) -> String {
        for (rel, text) in files {
            self.write(rel, text);
            self.git(&["add", "--", rel]);
        }
        self.git(&["commit", "-q", "--allow-empty", "-m", message]);
        self.git(&["rev-parse", "HEAD"])
    }

    /// The gate over `base..head`, sourced in a strict shell as `pre-push` does.
    fn gate(&self, base: &str, head: &str) -> Output {
        let gate = common::repository::root().join("tools/git-hooks/consumer-guide-gate.sh");
        let script = format!(
            "set -euo pipefail\nsource '{}'\nconsumer_guide_gate_range '{base}..{head}'\n",
            gate.display()
        );
        Command::new("bash")
            .arg("-c")
            .arg(script)
            .current_dir(&self.dir)
            .output()
            .expect("bash runs the gate")
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

const GOLDEN: &str = "tests/forge/expected/codec_probe.rs";
const RUNTIME: &str = "backends/rust/forge-runtime/src/codec.rs";
const ORACLE: &str = "tests/forge/conformance/numerical_reference.json";
const GUIDE: &str = "docs/SCE_CODEC_CONSUMER_MIGRATION.md";

const GOLDEN_BEFORE: &str = "/// A probe codec.\npub struct Probe {\n    pub len: u8,\n}\n";
const ORACLE_BEFORE: &str =
    "{\n  \"rejects\": [\n    {\n      \"why\": \"too long\",\n      \"error\": \"need-more-bytes\"\n    }\n  ]\n}\n";

/// A sandbox holding a golden, a runtime file, an oracle and the guide, and the
/// commit they make.
fn base(label: &str) -> (Sandbox, String) {
    let sb = Sandbox::new(label);
    let head = sb.commit(
        &[
            (GOLDEN, GOLDEN_BEFORE),
            (RUNTIME, "pub enum CodecError {\n    NeedMoreBytes,\n}\n"),
            (ORACLE, ORACLE_BEFORE),
            (GUIDE, "# Codec API changes\n"),
        ],
        "base",
    );
    (sb, head)
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

#[test]
fn a_public_field_removed_from_a_golden_without_its_section_is_refused() {
    let (sb, start) = base("golden-removed");
    let head = sb.commit(
        &[(
            GOLDEN,
            "/// A probe codec.\npub struct Probe {\n    pub length: u8,\n}\n",
        )],
        "feat: Rename a probe field",
    );
    let out = sb.gate(&start, &head);
    assert!(
        !out.status.success(),
        "the rename must be refused:\n{}",
        stderr(&out)
    );
    assert!(
        stderr(&out).contains(GOLDEN),
        "the refusal names the golden:\n{}",
        stderr(&out)
    );
}

#[test]
fn a_runtime_item_removed_without_its_section_is_refused() {
    let (sb, start) = base("runtime-removed");
    let head = sb.commit(
        &[(RUNTIME, "pub enum CodecError {}\n")],
        "feat: Drop an error",
    );
    let out = sb.gate(&start, &head);
    assert!(
        !out.status.success(),
        "the removal must be refused:\n{}",
        stderr(&out)
    );
    assert!(stderr(&out).contains(RUNTIME), "{}", stderr(&out));
}

/// A consumer implements the mesh client trait and runs the mesh conformance kit,
/// so a public item of the mesh library is as much its API as the codec runtime.
#[test]
fn a_mesh_library_item_removed_without_its_section_is_refused() {
    let (sb, _) = base("mesh-removed");
    let mesh = "backends/rust/mesh/src/inbox.rs";
    sb.commit(
        &[(mesh, "pub trait ZenohClient {\n    fn publish(&self);\n}\n")],
        "feat: Add a client trait",
    );
    let before = sb.git(&["rev-parse", "HEAD"]);
    let head = sb.commit(
        &[(mesh, "pub trait MeshClient {\n    fn publish(&self);\n}\n")],
        "feat: Rename the client trait",
    );
    let out = sb.gate(&before, &head);
    assert!(
        !out.status.success(),
        "the rename must be refused:\n{}",
        stderr(&out)
    );
    assert!(stderr(&out).contains(mesh), "{}", stderr(&out));
}

#[test]
fn a_reject_refused_differently_without_its_section_is_refused() {
    let (sb, start) = base("oracle-changed");
    let head = sb.commit(
        &[(
            ORACLE,
            &ORACLE_BEFORE.replace("need-more-bytes", "length-out-of-range"),
        )],
        "feat: Refuse a long frame for its length",
    );
    let out = sb.gate(&start, &head);
    assert!(
        !out.status.success(),
        "the new refusal must be refused:\n{}",
        stderr(&out)
    );
    assert!(stderr(&out).contains(ORACLE), "{}", stderr(&out));
}

#[test]
fn the_section_in_the_landings_feat_commit_covers_its_regenerated_goldens() {
    let (sb, start) = base("landing");
    sb.commit(
        &[(
            GUIDE,
            "# Codec API changes\n\n## A probe field is renamed\n",
        )],
        "feat: Rename a probe field",
    );
    let head = sb.commit(
        &[(
            GOLDEN,
            "/// A probe codec.\npub struct Probe {\n    pub length: u8,\n}\n",
        )],
        "chore: Regenerate the goldens",
    );
    let out = sb.gate(&start, &head);
    assert!(
        out.status.success(),
        "the landing has its section:\n{}",
        stderr(&out)
    );
}

#[test]
fn a_written_waiver_passes() {
    let (sb, start) = base("waiver");
    let head = sb.commit(
        &[(GOLDEN, "/// A probe codec.\npub struct Probe {\n    pub(crate) len: u8,\n}\n")],
        "refactor: Hide a probe field\n\n- No consumer-visible change: the probe is SCE's own fixture",
    );
    let out = sb.gate(&start, &head);
    assert!(
        out.status.success(),
        "the waiver is written:\n{}",
        stderr(&out)
    );
}

#[test]
fn a_comment_an_addition_or_a_new_golden_passes() {
    let (sb, start) = base("additions");
    let head = sb.commit(
        &[
            (
                GOLDEN,
                "/// A probe codec, documented.\npub struct Probe {\n    pub len: u8,\n    pub extra: u8,\n}\n",
            ),
            ("tests/forge/expected/codec_new.rs", "pub struct New;\n"),
            (RUNTIME, "pub enum CodecError {\n    NeedMoreBytes,\n    LengthOutOfRange,\n}\n"),
        ],
        "feat: Add what no consumer used before",
    );
    let out = sb.gate(&start, &head);
    assert!(
        out.status.success(),
        "nothing a consumer used changed:\n{}",
        stderr(&out)
    );
}

#[test]
fn a_fixtures_golden_removed_whole_passes() {
    let (sb, start) = base("golden-deleted");
    sb.git(&["rm", "-q", "--", GOLDEN]);
    sb.git(&["commit", "-q", "-m", "test: Retire the probe fixture"]);
    let head = sb.git(&["rev-parse", "HEAD"]);
    let out = sb.gate(&start, &head);
    assert!(
        out.status.success(),
        "a fixture leaving is no consumer API:\n{}",
        stderr(&out)
    );
}

#[test]
fn a_runtime_tests_change_passes() {
    let (sb, start) = base("runtime-tests");
    sb.commit(
        &[(
            "backends/rust/forge-runtime/tests/probe.rs",
            "pub fn probe() {}\n",
        )],
        "test: Add a probe",
    );
    let head = sb.commit(
        &[(
            "backends/rust/forge-runtime/tests/probe.rs",
            "fn probe() {}\n",
        )],
        "test: Narrow the probe",
    );
    let out = sb.gate(&start, &head);
    assert!(
        out.status.success(),
        "a test is no consumer API:\n{}",
        stderr(&out)
    );
}
