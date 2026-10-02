// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! `sce-work` — the works folder's commands, for a process that is not Rust.
//!
//! The authoring MCP is Python. It could read and write a works folder by
//! itself, and then there would be two implementations of what a save is and of
//! what makes two saves conflict, one of them without the other's lock. So it
//! runs this instead, the way it already runs `sce-codegen`, and the desktop app
//! and the MCP cannot disagree about the folder because only one thing writes it.
//!
//! ```text
//! sce-work [--root DIR] call <command> [--args JSON | --args-stdin]
//! sce-work [--root DIR] root
//! ```
//!
//! The answer to a command is one line of JSON on stdout. A refusal is one line
//! of JSON on stderr, `{"v":1,"error":{"kind":…,"message":…,"detail":…}}`, and
//! the exit status says which kind of refusal without parsing it:
//!
//! | status | meaning |
//! |---|---|
//! | 0 | done |
//! | 1 | refused: not found, invalid, too large, corrupt, io, bad request |
//! | 2 | the command line itself was wrong |
//! | 3 | `conflict`: the text was saved from a revision that is no longer current |
//! | 4 | `busy`: another save held the work's lock for the whole wait |
//!
//! A text to save travels on stdin (`--args-stdin`), not in `--args`: an
//! operating system limits a command line, and a specification is larger.

use std::io::Read;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use serde_json::Value;

use sce_app_core::{call, default_root, CommandError, WorkStore};

#[derive(Parser)]
#[command(name = "sce-work", about = "Commands on a specification works folder")]
struct Cli {
    /// The works folder. Default: `SCE_WORKS_DIR`, then the per-user data directory.
    #[arg(long, global = true)]
    root: Option<PathBuf>,

    #[command(subcommand)]
    command: Action,
}

#[derive(Subcommand)]
enum Action {
    /// Run a command (`describe` lists them).
    Call {
        /// The command's name.
        name: String,
        /// Its arguments as a JSON object.
        #[arg(long, conflicts_with = "args_stdin")]
        args: Option<String>,
        /// Read its arguments, a JSON object, from standard input.
        #[arg(long)]
        args_stdin: bool,
    },
    /// Print the works folder this command line resolves to.
    Root,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let Some(root) = cli.root.or_else(default_root) else {
        return refuse_plainly(
            "no works folder: pass --root, or set SCE_WORKS_DIR, or have a home directory",
        );
    };
    match cli.command {
        Action::Root => {
            println!("{}", root.display());
            ExitCode::SUCCESS
        }
        Action::Call {
            name,
            args,
            args_stdin,
        } => {
            let arguments = match read_arguments(args, args_stdin) {
                Ok(value) => value,
                Err(message) => return refuse_plainly(&message),
            };
            match call(&WorkStore::at(root), &name, arguments) {
                Ok(value) => {
                    println!("{value}");
                    ExitCode::SUCCESS
                }
                Err(error) => refuse(&error),
            }
        }
    }
}

fn read_arguments(args: Option<String>, from_stdin: bool) -> Result<Value, String> {
    let text = if from_stdin {
        let mut text = String::new();
        std::io::stdin()
            .read_to_string(&mut text)
            .map_err(|e| format!("could not read standard input: {e}"))?;
        text
    } else {
        args.unwrap_or_default()
    };
    if text.trim().is_empty() {
        return Ok(Value::Null);
    }
    serde_json::from_str(&text).map_err(|e| format!("the arguments are not JSON: {e}"))
}

fn refuse(error: &CommandError) -> ExitCode {
    eprintln!("{}", serde_json::json!({ "v": 1, "error": error }));
    ExitCode::from(match error.kind.as_str() {
        "conflict" => 3,
        "busy" => 4,
        _ => 1,
    })
}

fn refuse_plainly(message: &str) -> ExitCode {
    eprintln!(
        "{}",
        serde_json::json!({ "v": 1, "error": { "kind": "bad-request", "message": message } })
    );
    ExitCode::from(1)
}
