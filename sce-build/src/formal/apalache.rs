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
    pub sources: &'a Path,
    pub extra: &'a [(String, String)],
    pub module: &'a str,
    pub constants: &'a str,
    pub init: &'a str,
    pub invariants: &'a [String],
    pub length: u32,
}

/// The name of the configuration of a run's parameters, in its directory.
const CONSTANTS_FILE: &str = "constants.cfg";

/// The name of the empty JSON configuration a run gives Apalache in place of
/// the ones of the home and of the working directory.
const SETTINGS_FILE: &str = "apalache.json";

/// The argument `--<name>=<value>` for a path.
fn setting(name: &str, value: &Path) -> std::ffi::OsString {
    let mut argument = std::ffi::OsString::from(format!("--{name}="));
    argument.push(value);
    argument
}

/// Run Apalache as `run` says, in a directory of its own.
///
/// Deadlock is not checked (`--discard-disabled=false --no-deadlock`): a state
/// with no step is no counterexample to an invariant, and a state the start
/// predicate allows that no execution reaches may have none.
pub fn run(run: &Run<'_>) -> Result<Answer, FormalError> {
    let work = WorkDir::new("apalache")?;
    let mut staged = run.extra.to_vec();
    staged.push((CONSTANTS_FILE.to_string(), run.constants.to_string()));
    staged.push((SETTINGS_FILE.to_string(), "{}\n".to_string()));
    stage_modules(run.sources, work.path(), &staged)?;
    let settings = setting("config-file", &work.path().join(SETTINGS_FILE));
    let constants = setting("config", &work.path().join(CONSTANTS_FILE));
    let out_dir = setting("out-dir", &work.path().join("out"));
    let ran = Command::new(run.tool)
        .current_dir(work.path())
        .env("TMPDIR", work.path())
        .arg("check")
        .arg(settings)
        .arg(constants)
        .arg(format!("--init={}", run.init))
        .arg("--next=Next")
        .arg(format!("--inv={}", run.invariants.join(",")))
        .arg(format!("--length={}", run.length))
        .arg("--discard-disabled=false")
        .arg("--no-deadlock")
        .arg(out_dir)
        .arg(format!("{}.tla", run.module))
        .output()
        .map_err(|e| FormalError::ToolUnavailable {
            tool: "Apalache".to_string(),
            detail: format!(
                "cannot run {} (Java 17 or later on PATH runs it): {e}",
                run.tool.display()
            ),
        })?;
    Ok(Answer {
        // A run ended by a signal has no status; it judged nothing.
        status: ran.status.code().unwrap_or(-1),
        output: format!(
            "{}{}",
            String::from_utf8_lossy(&ran.stdout),
            String::from_utf8_lossy(&ran.stderr)
        ),
    })
}
