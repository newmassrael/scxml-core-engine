// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A test lane must be able to fail.
//
// This exists because one could not. The C++ W3C lane ran 404 conformance
// tests, two of them reached their fail state, and the job reported success
// for three weeks — three layers each declined to act on the failure:
//
//   1. `w3c_test_cli` computed its exit status as
//      `errorTests == 0 && passRate > 0`, never reading `failedTests`, so
//      the process exited 0 with two tests failing.
//   2. The step that runs it carried `continue-on-error: true`, so even a
//      non-zero status would not have failed the job.
//   3. The junit reporter carried `fail_on_failure: false`, so the check it
//      published was red while the job stayed green.
//
// Layer 1 is now a pure function with its own cases
// (`tests/common/TestSummaryExitStatusTest.cpp`). Layers 2 and 3 are
// workflow structure, which is what this reads.
//
// The rule is deliberately narrow: it constrains the step that RUNS a test
// suite, not the reporting steps around it. Those legitimately use
// `if: always()` so a failing run still uploads its artifacts and publishes
// its report — that is the arrangement `continue-on-error` was reaching for,
// and it works without disarming the lane.

use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent directory")
        .to_path_buf()
}

fn workflow_files() -> Vec<PathBuf> {
    let dir = repo_root().join(".github/workflows");
    let mut out: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read {}: {}", dir.display(), e))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "yml" || x == "yaml"))
        .collect();
    out.sort();
    assert!(
        out.len() > 5,
        "found {} workflow(s); the directory read is broken, not the tree",
        out.len()
    );
    out
}

/// Steps of one workflow, as (name, body) where body is every line of the
/// step until the next `- name:` at the same indent.
fn steps(text: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("- name:") {
            out.push((rest.trim().to_string(), String::new()));
        } else if let Some(last) = out.last_mut() {
            last.1.push('\n');
            last.1.push_str(line);
        }
    }
    out
}

#[test]
fn a_step_that_runs_a_test_suite_can_fail_its_job() {
    // `continue-on-error` on a *reporting* step is fine and is not what this
    // looks for. The signal is a step whose name says it runs tests.
    let runs_tests = |name: &str| {
        let n = name.to_lowercase();
        (n.contains("test") || n.contains("conformance"))
            && !n.contains("report")
            && !n.contains("summary")
            && !n.contains("upload")
            && !n.contains("publish")
            && !n.contains("analyze")
            && !n.contains("install")
            && !n.contains("setup")
            && !n.contains("cache")
            && !n.contains("build")
    };

    let mut disarmed: Vec<String> = Vec::new();
    for wf in workflow_files() {
        let text =
            fs::read_to_string(&wf).unwrap_or_else(|e| panic!("read {}: {}", wf.display(), e));
        let file = wf
            .file_name()
            .expect("workflow has a name")
            .to_string_lossy()
            .to_string();
        for (name, body) in steps(&text) {
            if !runs_tests(&name) {
                continue;
            }
            if body
                .lines()
                .map(str::trim)
                .any(|l| l.starts_with("continue-on-error:") && l.contains("true"))
            {
                disarmed.push(format!("{file}: step '{name}'"));
            }
        }
    }
    assert!(
        disarmed.is_empty(),
        "test-running step(s) carry `continue-on-error: true`, so their \
         failures cannot fail the job:\n  {}\n\
         Reporting steps around them can use `if: always()` instead, which \
         keeps artifacts and reports on a failing run without disarming the \
         lane.",
        disarmed.join("\n  ")
    );
}

#[test]
fn a_job_publishing_a_check_declares_permission_to_write_it() {
    // `mikepenz/action-junit-report` creates a check run, which needs
    // `checks: write`. Without it the step fails with "Resource not
    // accessible by integration" and that suite's results get no check at
    // all — observed on the Kotlin W3C job while its C++ sibling, which
    // declared the permission, published normally. A missing declaration is
    // silent: the reporter warns and the job still succeeds.
    let mut missing: Vec<String> = Vec::new();
    for wf in workflow_files() {
        let text =
            fs::read_to_string(&wf).unwrap_or_else(|e| panic!("read {}: {}", wf.display(), e));
        let file = wf
            .file_name()
            .expect("workflow has a name")
            .to_string_lossy()
            .to_string();

        // Split into jobs by their two-space-indented `<id>:` header, which
        // is the only place a `permissions:` block can attach.
        let mut job = String::new();
        let mut body = String::new();
        let mut jobs: Vec<(String, String)> = Vec::new();
        for line in text.lines() {
            let indent = line.len() - line.trim_start().len();
            let t = line.trim_end();
            if indent == 2 && t.ends_with(':') && !t.trim_start().starts_with('#') {
                if !job.is_empty() {
                    jobs.push((job.clone(), std::mem::take(&mut body)));
                }
                job = t.trim().trim_end_matches(':').to_string();
            } else if !job.is_empty() {
                body.push('\n');
                body.push_str(line);
            }
        }
        if !job.is_empty() {
            jobs.push((job, body));
        }

        for (id, body) in jobs {
            if !body.contains("action-junit-report") {
                continue;
            }
            let declares = body.lines().map(str::trim).any(|l| l == "checks: write");
            if !declares {
                missing.push(format!("{file}: job '{id}'"));
            }
        }
    }
    assert!(
        missing.is_empty(),
        "job(s) publish a junit check without declaring `checks: write`:\n  {}\n\
         The reporter fails with 'Resource not accessible by integration' and \
         the job still succeeds, so those results get no check.",
        missing.join("\n  ")
    );
}
