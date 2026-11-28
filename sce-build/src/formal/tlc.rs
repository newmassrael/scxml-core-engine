// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The one runner of TLC (docs/adr/0146, D4): a module, a configuration, the
//! workers and, when asked, the state graph TLC writes. Every level that runs
//! TLC runs it here, and its answer is read here: the exit status, the count of
//! distinct states, the invariant it names violated, and the graph.
//!
//! The workers are a count, never TLC's `auto`: a judge is given a share of a
//! machine (a build machine's grant, a hosted runner's processors), and TLC's
//! `auto` is every processor the machine has, whatever share the judge was
//! given. On 2026-11-27 a unit granted one core ran TLC with `auto` on a
//! machine of many and did not end within the hour. The Java virtual machine
//! TLC runs on is told the same count, so its own threads (the collector's
//! among them) stay within it too.
//!
//! That virtual machine is the pinned runtime's (docs/adr/0146, D3), run by its
//! path: the `java` a search path finds first is the machine's, whatever its
//! release.
//!
//! TLC's two random choices are fixed: the irreducible polynomial its
//! fingerprints are taken by (`-fp`) and its seed (`-seed`). Unfixed, TLC
//! draws both afresh on every run, and the run it reports of a violated
//! temporal property is the one its search of the state graph met first,
//! which the fingerprints order. Measured 2026-11-28 on one machine, one
//! worker and one tree: four runs of the turnstile's broken properties gave
//! three runs of its `<sce:recurs>` violation and three of its
//! `<sce:leads-to>`, while each invariant's violation was the same four
//! times. A scenario written of a violation is committed and replayed on
//! every lane (docs/adr/0115, D15), so the run that gives it must be the
//! same run wherever it is judged.

use std::num::NonZeroUsize;
use std::path::Path;
use std::process::Command;

use super::{stage_modules, FormalError, WorkDir};

/// TLC's exit status for a run that found no violation.
pub const HOLDS: i32 = 0;
/// TLC's exit status for a violated invariant or assertion.
pub const SAFETY: i32 = 12;
/// TLC's exit status for a violated temporal property. Any status but these
/// three is a run that did not judge.
pub const LIVENESS: i32 = 13;

/// The index of the irreducible polynomial TLC takes its fingerprints by
/// (`-fp`, one of TLC's 131), the same on every run.
pub const FINGERPRINT_POLYNOMIAL: u8 = 0;
/// The seed of TLC's random choices (`-seed`), the same on every run.
pub const SEED: u64 = 0;

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

/// Read the workers a judge is given: a count of at least one, or `auto`,
/// every processor this process may run on
/// ([`std::thread::available_parallelism`], which reads the processors the
/// process is confined to). Either way the answer is a count, so TLC is never
/// handed its own `auto`.
pub fn workers_spec(text: &str) -> Result<NonZeroUsize, String> {
    if text == "auto" {
        return Ok(available_workers());
    }
    text.parse::<NonZeroUsize>()
        .map_err(|_| format!("workers is a count of at least 1, or auto, got {text:?}"))
}

/// Every processor this process may run on, and one when that cannot be read.
pub fn available_workers() -> NonZeroUsize {
    std::thread::available_parallelism().unwrap_or(NonZeroUsize::MIN)
}

/// What every run of a judge is given: `java`, the program of the pinned Java
/// runtime, `jar`, the pinned TLC, and `library`, when given, a directory of
/// standard modules TLC reads beside its own (SANY's `TLA-Library`): the
/// pinned Apalache's, for a model that extends the module `Apalache`.
#[derive(Debug, Clone, Copy)]
pub struct Tools<'a> {
    pub java: &'a Path,
    pub jar: &'a Path,
    pub library: Option<&'a Path>,
}

/// One run: the modules of `sources` (every `.tla` there), the configuration
/// `config_text` written as `config_name`, `extra` modules written beside them,
/// and `module` checked under it, by `tools`.
pub struct Run<'a> {
    pub tools: Tools<'a>,
    pub sources: &'a Path,
    pub module: &'a str,
    pub config_name: &'a str,
    pub config_text: &'a str,
    pub extra: &'a [(String, String)],
    pub workers: NonZeroUsize,
    pub write_graph: bool,
}

/// Run TLC as `run` says, in a directory of its own.
pub fn run(run: &Run<'_>) -> Result<Answer, FormalError> {
    let work = WorkDir::new("tlc")?;
    let mut staged = run.extra.to_vec();
    staged.push((run.config_name.to_string(), run.config_text.to_string()));
    stage_modules(run.sources, work.path(), &staged)?;
    let ran =
        java_command(run, work.path())
            .output()
            .map_err(|e| FormalError::ToolUnavailable {
                tool: "Java".to_string(),
                detail: format!(
                    "cannot run the pinned runtime's {}: {e}",
                    run.tools.java.display()
                ),
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

/// The command of `run`, in the directory `work` it was staged in.
///
/// `-fp` and `-seed` are [`FINGERPRINT_POLYNOMIAL`] and [`SEED`], so a run
/// on any machine searches its state graph in one order.
/// `-workers` is the count the judge was given, and
/// `-XX:ActiveProcessorCount` makes it the processors the virtual machine
/// sees, which sizes the collector's threads and any pool TLC sizes by them.
/// The program is the pinned runtime's `java`, never one a search path finds.
fn java_command(run: &Run<'_>, work: &Path) -> Command {
    let workers = run.workers.to_string();
    let mut command = Command::new(run.tools.java);
    command
        .current_dir(work)
        .arg("-XX:+UseParallelGC")
        .arg(format!("-XX:ActiveProcessorCount={workers}"));
    if let Some(library) = run.tools.library {
        let mut property = std::ffi::OsString::from("-DTLA-Library=");
        property.push(library);
        command.arg(property);
    }
    command
        .arg("-cp")
        .arg(run.tools.jar)
        .arg("tlc2.TLC")
        .args(["-workers", workers.as_str()])
        .args(["-fp", FINGERPRINT_POLYNOMIAL.to_string().as_str()])
        .args(["-seed", SEED.to_string().as_str()]);
    if run.write_graph {
        command.args(["-dump", "dot", "graph.dot"]);
    }
    command
        .args(["-config", run.config_name])
        .arg(format!("{}.tla", run.module));
    command
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_workers_are_a_count_and_never_tlcs_auto() {
        assert_eq!(
            workers_spec("3"),
            Ok(NonZeroUsize::new(3).expect("3 is not 0"))
        );
        assert_eq!(workers_spec("auto"), Ok(available_workers()));
        for refused in ["0", "-1", "", "two", "1.5"] {
            assert!(workers_spec(refused).is_err(), "{refused:?}");
        }
    }

    #[test]
    fn tlc_and_its_virtual_machine_are_given_the_workers_the_judge_was() {
        let run = Run {
            tools: Tools {
                java: Path::new("/opt/jre/bin/java"),
                jar: Path::new("tla2tools.jar"),
                library: None,
            },
            sources: Path::new("."),
            module: "M",
            config_name: "M.cfg",
            config_text: "",
            extra: &[],
            workers: NonZeroUsize::new(2).expect("2 is not 0"),
            write_graph: false,
        };
        let built = java_command(&run, Path::new("."));
        let args: Vec<String> = built
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        let after = |flag: &str| {
            args.iter()
                .position(|arg| arg == flag)
                .and_then(|at| args.get(at + 1))
                .cloned()
        };
        assert_eq!(after("-workers").as_deref(), Some("2"), "{args:?}");
        assert!(
            args.iter().any(|arg| arg == "-XX:ActiveProcessorCount=2"),
            "{args:?}"
        );
        assert!(!args.iter().any(|arg| arg == "auto"), "{args:?}");
    }

    /// Turns red if a run leaves TLC to draw its fingerprint polynomial or
    /// its seed, and so the run it reports of a temporal violation.
    #[test]
    fn tlc_is_given_one_fingerprint_polynomial_and_one_seed() {
        let run = Run {
            tools: Tools {
                java: Path::new("/opt/jre/bin/java"),
                jar: Path::new("tla2tools.jar"),
                library: None,
            },
            sources: Path::new("."),
            module: "M",
            config_name: "M.cfg",
            config_text: "",
            extra: &[],
            workers: NonZeroUsize::MIN,
            write_graph: false,
        };
        let built = java_command(&run, Path::new("."));
        let args: Vec<String> = built
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        let after = |flag: &str| {
            args.iter()
                .position(|arg| arg == flag)
                .and_then(|at| args.get(at + 1))
                .cloned()
        };
        assert_eq!(after("-fp").as_deref(), Some("0"), "{args:?}");
        assert_eq!(after("-seed").as_deref(), Some("0"), "{args:?}");
    }

    #[test]
    fn tlc_runs_on_the_pinned_runtime_and_never_a_java_the_search_path_finds() {
        let run = Run {
            tools: Tools {
                java: Path::new("/opt/jre/bin/java"),
                jar: Path::new("tla2tools.jar"),
                library: Some(Path::new("/opt/apalache/modules")),
            },
            sources: Path::new("."),
            module: "M",
            config_name: "M.cfg",
            config_text: "",
            extra: &[],
            workers: NonZeroUsize::MIN,
            write_graph: false,
        };
        let built = java_command(&run, Path::new("."));
        assert_eq!(
            Path::new(built.get_program()),
            Path::new("/opt/jre/bin/java")
        );
        let args: Vec<String> = built
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        assert!(
            args.iter()
                .any(|arg| arg == "-DTLA-Library=/opt/apalache/modules"),
            "{args:?}"
        );
        let jar = args.iter().position(|arg| arg == "-cp").map(|at| at + 1);
        assert_eq!(
            jar.and_then(|at| args.get(at)).map(String::as_str),
            Some("tla2tools.jar"),
            "{args:?}"
        );
    }

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
