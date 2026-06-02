// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// NL→IR Item C1 Path A regression gate for the cross-state native-guard
// collision.
//
// `fixtures/event_schema/statechart_cross_state_guard.scxml` puts a native
// typed `_event.data` guard on the FIRST transition of three different source
// states (waiting / running / escalated), so all three share local
// `transition_index` 0. The per-language payload builders used to key a
// machine-global guard map by that per-state index, so the alphabetically-last
// state's guard (waiting → `elapsed_ms`) overwrote the other two and every
// state matched on `elapsed_ms` — a silent miscompilation. The guard now
// rides home on its owning `Transition::native_payload_guard`, where a
// per-state index cannot collide it.
//
// Each state's guard reads a DISTINCT field, so a collision is detectable by
// token presence alone: every backend must emit the exact field accessor for
// EACH state's guard. With the bug, the `retried` and `job_id` accessors are
// absent (replaced by a third `elapsed_ms` comparison). The accessor prefixes
// asserted below appear only in lowered guards, never in the payload struct
// definitions or the inject seams, so a match is unambiguous.
//
// Pure text assertions on generator output — no toolchain needed, so this
// runs unconditionally (compilation of the multi-state case is covered for
// all six backends by `forge_codegen_event_schema_smoke.rs`).

use std::path::PathBuf;
use std::process::Command;

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/event_schema/statechart_cross_state_guard.scxml")
}

/// Generate the fixture for `lang` and return every emitted file concatenated.
fn generate(lang: &str) -> String {
    let out_dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("event_schema_cross_state_guard")
        .join(lang);
    std::fs::create_dir_all(&out_dir).expect("create scratch dir");

    let status = Command::new(sce_codegen_bin())
        .arg("generate")
        .arg(fixture())
        .arg("-o")
        .arg(&out_dir)
        .arg("-l")
        .arg(lang)
        .status()
        .expect("run sce-codegen");
    assert!(status.success(), "sce-codegen generate failed for {lang}");

    let mut combined = String::new();
    for entry in std::fs::read_dir(&out_dir).expect("read scratch dir") {
        let path = entry.expect("dir entry").path();
        if path.is_file() {
            combined.push_str(&std::fs::read_to_string(&path).expect("read generated file"));
            combined.push('\n');
        }
    }
    combined
}

/// `(language, [guard accessor for waiting, running, escalated])`. The three
/// markers are the distinct-field guard expressions the three source states
/// must each lower to; each accessor prefix is guard-only, so presence proves
/// the matching state's guard survived rather than being overwritten by
/// another state's local-index-0 guard.
const EXPECTED: &[(&str, [&str; 3])] = &[
    (
        "rust",
        [
            "ev.elapsed_ms == 0",
            "ev.retried == true",
            "ev.job_id == \"sentinel\"",
        ],
    ),
    (
        "cpp",
        [
            "pendingJobCompletedPayload_.elapsed_ms == 0",
            "pendingJobCompletedPayload_.retried == true",
            "pendingJobCompletedPayload_.job_id == \"sentinel\"",
        ],
    ),
    (
        "c11",
        [
            "sm->pending_payload.as.job_completed.elapsed_ms == 0",
            "sm->pending_payload.as.job_completed.retried == true",
            "strcmp(sm->pending_payload.as.job_completed.job_id, \"sentinel\") == 0",
        ],
    ),
    (
        "go",
        [
            "p.pendingJobCompletedPayload.elapsed_ms == 0",
            "p.pendingJobCompletedPayload.retried == true",
            "p.pendingJobCompletedPayload.job_id == \"sentinel\"",
        ],
    ),
    (
        "kotlin",
        [
            "pendingJobCompletedPayload!!.elapsed_ms == 0.toUInt()",
            "pendingJobCompletedPayload!!.retried == true",
            "pendingJobCompletedPayload!!.job_id == \"sentinel\"",
        ],
    ),
    (
        "python",
        [
            "self._pending_job_completed_payload.elapsed_ms == 0",
            "self._pending_job_completed_payload.retried == True",
            "self._pending_job_completed_payload.job_id == 'sentinel'",
        ],
    ),
];

#[test]
fn every_backend_lowers_each_states_guard_to_its_own_field() {
    for (lang, markers) in EXPECTED {
        let code = generate(lang);
        for marker in markers {
            assert!(
                code.contains(marker),
                "{lang}: missing guard `{marker}` — a cross-state \
                 transition_index collision overwrote it (every state would \
                 share the alphabetically-last guard `elapsed_ms`)",
            );
        }
    }
}
