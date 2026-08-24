// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The shape of a GitHub Actions workflow, parsed once for every suite that
//! reads one.
//!
//! Two suites ask different questions of the same structure, and both
//! questions are per-JOB rather than per-file: a job runs on its own runner,
//! so what a neighbouring job declares — a `defaults.run.shell`, an install
//! step — is not something this one has. `test_result_gating` reads it to
//! find a test step that cannot fail; `gate_registry_contract` reads it to
//! find a job that runs a gate whose tooling it never installed. A second
//! parser written for the second question would be a second answer to "where
//! does this job end", and the two would drift.
//!
//! Kept deliberately small: this is the split into jobs and steps, not a YAML
//! reader. Every predicate over the text stays with the suite that asks it.

// Each consumer reads a different part — one wants the steps, one wants whole
// jobs, and the probe-crate suites that pull `common` in for its other module
// read none of it. An unused item here is therefore the module serving a
// caller that is not this binary, not a leftover.
#![allow(dead_code)]

/// One `- name:` step, with the absolute line number of every body line so a
/// violation can be reported as `file:line`.
pub struct Step {
    pub name: String,
    pub body: Vec<(usize, String)>,
}

/// One job, split into the text that precedes its first step — where a
/// job-level `defaults.run.shell` lives — and the steps themselves.
pub struct Job {
    pub id: String,
    pub prelude: String,
    pub steps: Vec<Step>,
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
pub fn split_workflow(text: &str) -> (String, Vec<Job>) {
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
pub fn job_text(job: &Job) -> String {
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
