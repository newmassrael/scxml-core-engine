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
//
// A fourth layer sits below those three and is not about test steps at all:
// the runner's default shell is `bash -e {0}`, so ANY top-level pipeline in
// a `run:` script reports only its rightmost command's status. That one is
// counted over every workflow rather than over test-named steps, because
// the shape does not know what it is carrying — see
// `every_pipeline_in_a_run_script_reports_its_own_failure`.

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
        let (_, jobs) = split_workflow(&text);
        for step in jobs.iter().flat_map(|j| &j.steps) {
            if !runs_tests(&step.name) {
                continue;
            }
            if step
                .body
                .iter()
                .map(|(_, l)| l.trim())
                .any(|l| l.starts_with("continue-on-error:") && l.contains("true"))
            {
                disarmed.push(format!("{file}: step '{}'", step.name));
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

/// One `- name:` step, with the absolute line number of every body line so a
/// violation can be reported as `file:line`.
struct Step {
    name: String,
    body: Vec<(usize, String)>,
}

/// One job, split into the text that precedes its first step — where a
/// job-level `defaults.run.shell` lives — and the steps themselves.
struct Job {
    id: String,
    prelude: String,
    steps: Vec<Step>,
    /// Indent of this job's step items, learned from the first one. A step
    /// boundary is a `- ` at exactly this column; a `- ` deeper than it is
    /// an element of some step's own list (`with.args`, `path`), not a step.
    steps_indent: Option<usize>,
    seen_steps_key: bool,
}

/// Split a workflow into (text before `jobs:`, jobs).
///
/// The workflow-level half matters because `defaults.run.shell` may be
/// declared there and applies to every step in the file.
///
/// Steps are cut at every list item rather than at `- name:` alone: a step
/// need not be named, and folding an unnamed one into its predecessor would
/// let it inherit a `set -o pipefail` it never declared.
fn split_workflow(text: &str) -> (String, Vec<Job>) {
    let mut header = String::new();
    let mut jobs: Vec<Job> = Vec::new();
    let mut in_jobs = false;

    for (idx, line) in text.lines().enumerate() {
        let no = idx + 1;
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();

        if !in_jobs {
            if indent == 0 && trimmed.trim_end() == "jobs:" {
                in_jobs = true;
            } else {
                header.push_str(line);
                header.push('\n');
            }
            continue;
        }

        // A job header is the only thing at indent 2 that ends in `:`.
        if indent == 2 && !trimmed.starts_with('#') && trimmed.trim_end().ends_with(':') {
            jobs.push(Job {
                id: trimmed.trim_end().trim_end_matches(':').to_string(),
                prelude: String::new(),
                steps: Vec::new(),
                steps_indent: None,
                seen_steps_key: false,
            });
            continue;
        }
        let Some(job) = jobs.last_mut() else {
            continue;
        };
        if trimmed.trim_end() == "steps:" {
            job.seen_steps_key = true;
        }
        let is_item = trimmed.starts_with("- ");
        if is_item && job.seen_steps_key && job.steps_indent.is_none() {
            job.steps_indent = Some(indent);
        }

        if is_item && job.steps_indent == Some(indent) {
            let item = trimmed.strip_prefix("- ").expect("checked above");
            let name = item
                .strip_prefix("name:")
                .map(|n| n.trim().to_string())
                .unwrap_or_else(|| format!("(unnamed) {}", item.trim()));
            let mut step = Step {
                name,
                body: Vec::new(),
            };
            if !item.trim_start().starts_with("name:") {
                // `- run: …` carries the step's only key on the item line.
                // Re-indent so the key sits where a named step's would.
                step.body
                    .push((no, format!("{}  {item}", " ".repeat(indent))));
            }
            job.steps.push(step);
        } else if let Some(step) = job.steps.last_mut() {
            step.body.push((no, line.to_string()));
        } else {
            job.prelude.push_str(line);
            job.prelude.push('\n');
        }
    }
    (header, jobs)
}

/// A job's whole text: the keys declared before its steps, then the steps.
fn job_text(job: &Job) -> String {
    let mut out = job.prelude.clone();
    for step in &job.steps {
        out.push_str("- name: ");
        out.push_str(&step.name);
        out.push('\n');
        for (_, line) in &step.body {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

/// True when `text` declares a `shell:` that turns `pipefail` on.
///
/// Naming `defaults.run.shell` REPLACES the runner's default rather than
/// adding to it, which is why the declaration has to be read rather than
/// assumed — `bash {0}` would be a shell declaration that silently drops
/// `-e` as well.
fn declares_pipefail_shell(text: &str) -> bool {
    text.lines()
        .map(str::trim)
        .any(|l| l.starts_with("shell:") && l.contains("pipefail"))
}

/// The part of a shell line that is code the step's status depends on:
/// quoted spans and `$( )` / `` ` ` `` substitutions are blanked, and an
/// unquoted `#` ends the line.
///
/// A pipeline inside a substitution decides an assignment's status, not the
/// step's; `awk -F'|'` and `echo "a | b"` are not pipelines at all; and a
/// comment is not a declaration. That last one is not hypothetical — the
/// first draft of this test read `set -o pipefail` out of the prose comment
/// explaining why a nearby line does NOT want pipefail, and passed a
/// mutation that put the defect back.
fn shell_code(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();
    let mut depth = 0usize;
    let mut quote: Option<char> = None;

    while let Some(c) = chars.next() {
        if let Some(q) = quote {
            if c == q {
                quote = None;
            }
            out.push(' ');
            continue;
        }
        match c {
            '#' if depth == 0 => break,
            '\'' | '"' | '`' => {
                quote = Some(c);
                out.push(' ');
            }
            '$' if chars.peek() == Some(&'(') => {
                chars.next();
                depth += 1;
                out.push_str("  ");
            }
            '(' if depth > 0 => {
                depth += 1;
                out.push(' ');
            }
            ')' if depth > 0 => {
                depth -= 1;
                out.push(' ');
            }
            _ => out.push(if depth > 0 { ' ' } else { c }),
        }
    }
    out
}

/// True when the line runs a real pipeline whose status the step inherits.
fn has_top_level_pipe(line: &str) -> bool {
    shell_code(line).replace("||", "  ").contains('|')
}

/// True when the line is a `set` that turns `pipefail` on — the command,
/// not the word.
fn sets_pipefail(line: &str) -> bool {
    let code = shell_code(line);
    let code = code.trim();
    code.starts_with("set ") && code.contains("pipefail")
}

/// The lines of a step's `run:` script, with their absolute line numbers.
///
/// Only these lines are scanned: a `with:` value may legitimately be a `|`
/// block scalar, and that pipe is YAML syntax rather than a shell pipeline.
fn run_script(step: &Step) -> Vec<(usize, String)> {
    let mut out: Vec<(usize, String)> = Vec::new();
    let mut block_indent: Option<usize> = None;

    for (no, line) in &step.body {
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();

        if let Some(ri) = block_indent {
            if trimmed.is_empty() {
                continue;
            }
            if indent > ri {
                out.push((*no, line.clone()));
                continue;
            }
            block_indent = None;
        }
        if let Some(rest) = trimmed.strip_prefix("run:") {
            let rest = rest.trim();
            if rest.is_empty() || rest == "|" || rest == "|-" || rest == ">" || rest == ">-" {
                block_indent = Some(indent);
            } else {
                out.push((*no, rest.to_string()));
            }
        }
    }
    out
}

/// A pipeline in a CI step reports only its rightmost command's status.
///
/// The runner's default shell is `bash -e {0}`: `-e`, but not `-o pipefail`.
/// So `run-the-suite | tee log` exits with tee's status, and a suite that
/// failed every assertion reports green. This was found on the forge
/// conformance lanes, fixed there with a workflow-level `defaults.run.shell`,
/// and the fix was pinned by a test scoped to that ONE file — whose own
/// doc comment recorded that `w3c-tests.yml` carried the same shape at more
/// sites and left them for later. Two of them were still live when this was
/// written, three weeks on. A rule that names one file cannot count the
/// sites it does not name, so this counts the shape across every workflow.
///
/// It is deliberately broader than "steps that run tests": the pipe does not
/// know what it is carrying. A `Debug Environment` step here piped `ls` into
/// `head` with `|| echo "ERROR: resources directory not found"` — the error
/// branch could never be taken, because head's status is what `||` tested.
///
/// A pipeline whose left side really is meant to be ignored stays legal;
/// it just has to say so, by handling the status rather than by relying on
/// a shell option nobody declared.
#[test]
fn every_pipeline_in_a_run_script_reports_its_own_failure() {
    let mut swallowed: Vec<String> = Vec::new();
    let mut examined = 0usize;

    for wf in workflow_files() {
        let text =
            fs::read_to_string(&wf).unwrap_or_else(|e| panic!("read {}: {}", wf.display(), e));
        let file = wf
            .file_name()
            .expect("workflow has a name")
            .to_string_lossy()
            .to_string();

        let (header, jobs) = split_workflow(&text);
        let workflow_wide = declares_pipefail_shell(&header);

        for job in &jobs {
            let job_wide = workflow_wide || declares_pipefail_shell(&job.prelude);
            for step in &job.steps {
                let script = run_script(step);
                if script.is_empty() {
                    continue;
                }
                let step_wide = job_wide
                    || script.iter().any(|(_, l)| sets_pipefail(l))
                    || step
                        .body
                        .iter()
                        .any(|(_, l)| declares_pipefail_shell(l.trim()));

                for (no, line) in &script {
                    if !has_top_level_pipe(line) {
                        continue;
                    }
                    examined += 1;
                    if !step_wide {
                        swallowed.push(format!(
                            "{file}:{no}  job '{}' step '{}'\n      {}",
                            job.id,
                            step.name,
                            line.trim()
                        ));
                    }
                }
            }
        }
    }

    // A scanner that finds nothing must not pass. The floor is well under
    // today's count so that removing a pipeline is not a failure, but a
    // parser that stops seeing `run:` blocks is.
    assert!(
        examined >= 8,
        "found only {examined} top-level pipeline(s) across the workflows; \
         every lane pipes its run into `tee` for the summary step, so this \
         means the scan broke, not the tree",
    );

    assert!(
        swallowed.is_empty(),
        "{} of {} pipeline(s) in workflow `run:` scripts run under the \
         runner's `bash -e {{0}}` default, which does not set pipefail — the \
         step takes the RIGHTMOST command's exit status, so a failure on the \
         left is reported green:\n  {}\n\n\
         Fix by declaring `set -o pipefail` in the step, or \
         `defaults.run.shell: bash -eo pipefail {{0}}` for the whole \
         workflow. If the left side's status is genuinely not the verdict, \
         handle it explicitly instead of leaving it to an undeclared shell \
         option.",
        swallowed.len(),
        examined,
        swallowed.join("\n  "),
    );
}

/// The scanner above decides a contract from shell text, so the two ways it
/// can be wrong are pinned directly: reading a pipe that is not one, and
/// reading a declaration out of prose that says the opposite.
#[test]
fn the_pipeline_scanner_reads_code_not_prose() {
    assert!(has_top_level_pipe("cargo test 2>&1 | tee log"));
    assert!(has_top_level_pipe("a | b || c"));
    assert!(!has_top_level_pipe("a || b"));
    assert!(!has_top_level_pipe("echo \"a | b\""));
    assert!(!has_top_level_pipe("awk -F'|' '{print $1}' f"));
    assert!(!has_top_level_pipe("passed=$(grep x log | wc -l)"));
    assert!(!has_top_level_pipe("# ls -la resources | head -n 10"));

    assert!(sets_pipefail("set -o pipefail"));
    assert!(sets_pipefail("        set -euo pipefail"));
    assert!(!sets_pipefail("# Nor is pipefail the fix here"));
    assert!(!sets_pipefail("echo 'set -o pipefail'"));

    assert!(declares_pipefail_shell("shell: bash -eo pipefail {0}"));
    assert!(!declares_pipefail_shell("shell: bash -e {0}"));
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

        // `permissions:` attaches to the job, so the whole job — prelude and
        // steps alike — is what has to be read.
        let (_, jobs) = split_workflow(&text);
        for job in &jobs {
            let body = job_text(job);
            if !body.contains("action-junit-report") {
                continue;
            }
            let declares = body.lines().map(str::trim).any(|l| l == "checks: write");
            if !declares {
                missing.push(format!("{file}: job '{}'", job.id));
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
