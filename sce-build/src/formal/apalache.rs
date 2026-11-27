// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The one runner of Apalache (docs/adr/0115, D3; docs/adr/0146, D4): a
//! module, the parameters of a configuration, a predicate to start from, an
//! invariant and a number of steps. Apalache checks symbolically every
//! execution of at most that many steps from every state the predicate allows,
//! with every parameter fixed (Konnov, Kukovec and Tran, OOPSLA 2019). Every
//! level that runs Apalache runs it here, and its answer is read here.
//!
//! Apalache reads no file of its own but the ones a run names: its JSON
//! configuration is an empty one written for the run (`--config-file`), so a
//! file in the home or the working directory changes no verdict, and its
//! outputs go to the run's directory.
//!
//! It runs on the pinned Java runtime (docs/adr/0146, D3). The release's
//! launcher, `bin/apalache-mc`, runs the `java` its search path finds first and
//! reads no `JAVA_HOME`, so a run's search path is led by the pinned runtime's
//! `bin`, and `JAVA_HOME` names the same runtime for anything that looks there.

use std::ffi::{OsStr, OsString};
use std::path::Path;
use std::process::Command;

use super::{stage_modules, FormalError, WorkDir};

/// Apalache's exit status for a run that found no violation.
pub const HOLDS: i32 = 0;
/// Apalache's exit status for a violation of any kind (it reports every one as
/// TLC reports a violated invariant). Any status but these two is a run that
/// did not judge: 75 a specification it could not evaluate, 120 a type error,
/// 150 a parse error, 255 any other.
pub const VIOLATED: i32 = 12;

/// What one run of Apalache answered.
#[derive(Debug, Clone)]
pub struct Answer {
    /// Apalache's exit status.
    pub status: i32,
    /// What Apalache printed, its standard output then its standard error.
    pub output: String,
}

/// One run: the modules of `sources` (every `.tla` there) and `extra` written
/// beside them, `module` read with the CONSTANTS of `constants` (a TLC
/// configuration that holds nothing else), and `invariants` checked in every
/// state of every execution of at most `length` steps of `Next` from a state
/// that satisfies `init`.
pub struct Run<'a> {
    pub tool: &'a Path,
    /// The home of the pinned Java runtime Apalache runs on.
    pub java_home: &'a Path,
    pub sources: &'a Path,
    pub extra: &'a [(String, String)],
    pub module: &'a str,
    pub constants: &'a str,
    pub init: &'a str,
    pub invariants: &'a [String],
    pub length: u32,
}

/// The name of the configuration of a run, in its directory.
const CONFIG_FILE: &str = "run.cfg";

/// The name of the empty JSON configuration a run gives Apalache in place of
/// the ones of the home and of the working directory.
const SETTINGS_FILE: &str = "apalache.json";

/// The TLC configuration of `run`: the predicate it starts from, its step and
/// the CONSTANTS of its configuration. Apalache reads the behaviour from the
/// configuration it is given and refuses one that names none (a status of
/// 255), so the configuration names both, and nothing else names them.
fn config(run: &Run<'_>) -> String {
    format!("INIT {}\nNEXT Next\n{}", run.init, run.constants)
}

/// The argument `--<name>=<value>` for a path.
fn setting(name: &str, value: &Path) -> OsString {
    let mut argument = OsString::from(format!("--{name}="));
    argument.push(value);
    argument
}

/// The search path of a run: the `bin` of the runtime at `java_home` first, so
/// that the launcher's `java` is the pinned one, then `inherited`, for the shell
/// and the tools the launcher runs (`bash`, `dirname`, `readlink`, `mktemp`).
fn search_path(java_home: &Path, inherited: Option<&OsStr>) -> Result<OsString, FormalError> {
    let first = java_home.join("bin");
    let rest = inherited.into_iter().flat_map(std::env::split_paths);
    std::env::join_paths(std::iter::once(first).chain(rest)).map_err(|e| {
        FormalError::ToolUnavailable {
            tool: "Java".to_string(),
            detail: format!(
                "the pinned runtime at {} cannot lead a search path: {e}",
                java_home.display()
            ),
        }
    })
}

/// The command of `run`, in the directory `work` it was staged in, with the
/// search path `search`. `APALACHE_JAR` is taken out of what the run inherits:
/// the launcher runs the jar it names in place of the release's own.
fn command(run: &Run<'_>, work: &Path, search: &OsStr) -> Command {
    let mut command = Command::new(run.tool);
    command
        .current_dir(work)
        .env("TMPDIR", work)
        .env("JAVA_HOME", run.java_home)
        .env("PATH", search)
        .env_remove("APALACHE_JAR")
        .arg("check")
        .arg(setting("config-file", &work.join(SETTINGS_FILE)))
        .arg(setting("config", &work.join(CONFIG_FILE)))
        .arg(format!("--inv={}", run.invariants.join(",")))
        .arg(format!("--length={}", run.length))
        .arg("--discard-disabled=false")
        .arg("--no-deadlock")
        .arg(setting("out-dir", &work.join("out")))
        .arg(format!("{}.tla", run.module));
    command
}

/// Whether what a run printed says the Java runtime could not load Apalache:
/// a class built for a later release than the runtime's. Such a run judged
/// nothing, and is the runtime's failure, not the model's.
fn runtime_refused(output: &str) -> bool {
    output.contains("UnsupportedClassVersionError")
}

/// Run Apalache as `run` says, in a directory of its own.
///
/// Deadlock is not checked (`--discard-disabled=false --no-deadlock`): a state
/// with no step is no counterexample to an invariant, and a state the start
/// predicate allows that no execution reaches may have none.
pub fn run(run: &Run<'_>) -> Result<Answer, FormalError> {
    let work = WorkDir::new("apalache")?;
    let mut staged = run.extra.to_vec();
    staged.push((CONFIG_FILE.to_string(), config(run)));
    staged.push((SETTINGS_FILE.to_string(), "{}\n".to_string()));
    stage_modules(run.sources, work.path(), &staged)?;
    let search = search_path(run.java_home, std::env::var_os("PATH").as_deref())?;
    let ran =
        command(run, work.path(), &search)
            .output()
            .map_err(|e| FormalError::ToolUnavailable {
                tool: "Apalache".to_string(),
                detail: format!("cannot run {}: {e}", run.tool.display()),
            })?;
    let output = format!(
        "{}{}",
        String::from_utf8_lossy(&ran.stdout),
        String::from_utf8_lossy(&ran.stderr)
    );
    if runtime_refused(&output) {
        return Err(FormalError::ToolUnavailable {
            tool: "Java".to_string(),
            detail: format!(
                "the runtime at {} cannot load {}: UnsupportedClassVersionError, \
                 so the run judged nothing",
                run.java_home.display(),
                run.tool.display()
            ),
        });
    }
    Ok(Answer {
        // A run ended by a signal has no status; it judged nothing.
        status: ran.status.code().unwrap_or(-1),
        output,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_runs_configuration_names_its_start_and_step_before_its_parameters() {
        let run = Run {
            tool: Path::new("apalache-mc"),
            java_home: Path::new("/opt/jre"),
            sources: Path::new("."),
            extra: &[],
            module: "M",
            constants: "CONSTANTS\n    N = 2\n",
            init: "Start",
            invariants: &[],
            length: 1,
        };
        assert_eq!(
            config(&run),
            "INIT Start\nNEXT Next\nCONSTANTS\n    N = 2\n"
        );
    }

    #[test]
    fn the_launcher_finds_the_pinned_runtime_first_and_is_told_its_home() {
        let run = Run {
            tool: Path::new("/opt/apalache/bin/apalache"),
            java_home: Path::new("/opt/jre"),
            sources: Path::new("."),
            extra: &[],
            module: "M",
            constants: "",
            init: "Init",
            invariants: &["Safe".to_string()],
            length: 1,
        };
        let inherited = std::env::join_paths(["/usr/bin", "/bin"]).expect("a search path");
        let search =
            search_path(run.java_home, Some(inherited.as_os_str())).expect("a search path");
        let built = command(&run, Path::new("/work"), &search);
        let env = |name: &str| {
            built
                .get_envs()
                .find(|(key, _)| *key == OsStr::new(name))
                .map(|(_, value)| value.map(OsStr::to_os_string))
        };
        assert_eq!(env("JAVA_HOME"), Some(Some(OsString::from("/opt/jre"))));
        let path = env("PATH")
            .flatten()
            .expect("the run is given a search path");
        let found: Vec<_> = std::env::split_paths(&path).collect();
        assert_eq!(
            found,
            ["/opt/jre/bin", "/usr/bin", "/bin"].map(std::path::PathBuf::from),
            "the pinned runtime's bin leads, and the rest is what the judge was given"
        );
        assert_eq!(
            env("APALACHE_JAR"),
            Some(None),
            "a jar named from outside is not run in place of the release's"
        );
        assert_eq!(
            Path::new(built.get_program()),
            Path::new("/opt/apalache/bin/apalache")
        );
    }

    #[test]
    fn a_runtime_too_old_for_the_jar_is_told_from_a_verdict() {
        assert!(runtime_refused(
            "Exception in thread \"main\" java.lang.UnsupportedClassVersionError: \
             at/forsyte/apalache/tla/Tool has been compiled by a more recent version"
        ));
        assert!(!runtime_refused(
            "State 1: Check the trace in: out/violation.tla\nEXITCODE: ERROR (12)"
        ));
    }
}
