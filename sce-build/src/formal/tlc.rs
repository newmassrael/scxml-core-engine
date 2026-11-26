// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The one runner of TLC (docs/adr/0144, D4): a module, a configuration, the
//! workers and, when asked, the state graph TLC writes. Every level that runs
//! TLC runs it here, and its answer is read here: the exit status, the count of
//! distinct states, the invariant it names violated, and the graph.

use std::path::Path;
use std::process::Command;

use super::{FormalError, WorkDir};

/// TLC's exit status for a run that found no violation.
pub const HOLDS: i32 = 0;
/// TLC's exit status for a violated invariant or assertion.
pub const SAFETY: i32 = 12;
/// TLC's exit status for a violated temporal property. Any status but these
/// three is a run that did not judge.
pub const LIVENESS: i32 = 13;

/// What one run of TLC answered.
#[derive(Debug, Clone)]
pub struct Answer {
    /// TLC's exit status.
    pub status: i32,
    /// What TLC printed, its standard output then its standard error.
    pub output: String,
    /// The graph TLC wrote in the dot form, when it was asked for one.
    pub graph: Option<String>,
}

impl Answer {
    /// The last count of distinct states TLC reported.
    pub fn distinct_states(&self) -> Option<u64> {
        distinct_states(&self.output)
    }
}

/// The last count of distinct states in a TLC report: the digits before each
/// " distinct states found", which TLC prints alone and inside a longer line
/// ("12 states generated, 9 distinct states found, 0 states left on queue.").
pub fn distinct_states(output: &str) -> Option<u64> {
    output
        .match_indices(" distinct states found")
        .filter_map(|(at, _)| {
            let before = &output[..at];
            let digits = before.len() - before.trim_end_matches(|c: char| c.is_ascii_digit()).len();
            before[at - digits..].parse().ok()
        })
        .last()
}

/// The invariants a TLC report names violated, in order.
pub fn violated_invariants(output: &str) -> Vec<String> {
    let mut found = Vec::new();
    for (at, _) in output.match_indices("Invariant ") {
        let rest = &output[at + "Invariant ".len()..];
        let name: String = rest
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        if !name.is_empty() && rest[name.len()..].starts_with(" is violated") {
            found.push(name);
        }
    }
    found
}

/// One run: the modules of `sources` (every `.tla` there), the configuration
/// `config_text` written as `config_name`, `extra` modules written beside them,
/// and `module` checked under it.
pub struct Run<'a> {
    pub jar: &'a Path,
    pub sources: &'a Path,
    pub module: &'a str,
    pub config_name: &'a str,
    pub config_text: &'a str,
    pub extra: &'a [(String, String)],
    pub workers: &'a str,
    pub write_graph: bool,
}

/// Run TLC as `run` says, in a directory of its own.
pub fn run(run: &Run<'_>) -> Result<Answer, FormalError> {
    let work = WorkDir::new("tlc")?;
    let copy_failed = |e: std::io::Error| FormalError::ToolUnavailable {
        tool: "a working directory".to_string(),
        detail: format!(
            "cannot copy the modules of {} into {}: {e}",
            run.sources.display(),
            work.path().display()
        ),
    };
    for entry in std::fs::read_dir(run.sources).map_err(copy_failed)? {
        let path = entry.map_err(copy_failed)?.path();
        if path.extension().is_some_and(|ext| ext == "tla") {
            if let Some(name) = path.file_name() {
                std::fs::copy(&path, work.path().join(name)).map_err(copy_failed)?;
            }
        }
    }
    std::fs::write(work.path().join(run.config_name), run.config_text).map_err(copy_failed)?;
    for (name, text) in run.extra {
        std::fs::write(work.path().join(name), text).map_err(copy_failed)?;
    }
    let mut command = Command::new("java");
    command
        .current_dir(work.path())
        .arg("-XX:+UseParallelGC")
        .arg("-cp")
        .arg(run.jar)
        .arg("tlc2.TLC")
        .args(["-workers", run.workers]);
    if run.write_graph {
        command.args(["-dump", "dot", "graph.dot"]);
    }
    command
        .args(["-config", run.config_name])
        .arg(format!("{}.tla", run.module));
    let ran = command.output().map_err(|e| FormalError::ToolUnavailable {
        tool: "Java".to_string(),
        detail: format!("cannot run java (Java 11 or later on PATH runs TLC): {e}"),
    })?;
    let graph = if run.write_graph {
        std::fs::read_to_string(work.path().join("graph.dot")).ok()
    } else {
        None
    };
    Ok(Answer {
        // A run ended by a signal has no status; it judged nothing, as any
        // status but TLC's three does.
        status: ran.status.code().unwrap_or(-1),
        output: format!(
            "{}{}",
            String::from_utf8_lossy(&ran.stdout),
            String::from_utf8_lossy(&ran.stderr)
        ),
        graph,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_last_count_of_distinct_states_is_read() {
        assert_eq!(distinct_states("7 distinct states found"), Some(7));
        assert_eq!(
            distinct_states("Progress: 3 distinct states found\n12 states generated, 9 distinct states found, 0 states left on queue."),
            Some(9)
        );
        assert_eq!(distinct_states("nothing"), None);
    }

    #[test]
    fn the_violated_invariants_are_read_in_order() {
        assert_eq!(
            violated_invariants("Error: Invariant Safe is violated."),
            vec!["Safe".to_string()]
        );
        assert_eq!(
            violated_invariants("Invariant Other is violated. Invariant Safe is violated."),
            vec!["Other", "Safe"]
        );
        assert!(violated_invariants("Invariant Safe holds").is_empty());
    }
}
