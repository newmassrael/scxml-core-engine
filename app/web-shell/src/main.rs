// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! `sce-web-shell [--listen ADDR:PORT] [--root DIR] [--ui DIR]`
//!
//! The token comes from `SCE_WEB_TOKEN` (at least 16 characters) or is generated
//! and printed in the URL to open.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use sce_app_core::host::{self, HostSettings};
use sce_app_core::{default_renderer, default_root, SceCodegen, WorkStore};
use sce_web_shell::address::check_bind;
use sce_web_shell::assets::Assets;
use sce_web_shell::server::{serve, Limits};
use sce_web_shell::{token, Shell};

const DEFAULT_LISTEN: &str = "127.0.0.1:5174";

const USAGE: &str = "usage: sce-web-shell [--listen ADDR:PORT] [--root DIR] [--ui DIR]";

struct Options {
    listen: SocketAddr,
    root: Option<PathBuf>,
    ui: Option<PathBuf>,
}

fn parse(args: impl Iterator<Item = String>) -> Result<Options, String> {
    let mut options = Options {
        listen: DEFAULT_LISTEN
            .parse()
            .expect("the default address is valid"),
        root: None,
        ui: None,
    };
    let mut args = args;
    while let Some(flag) = args.next() {
        let mut value = |name: &str| args.next().ok_or_else(|| format!("{name} needs a value"));
        match flag.as_str() {
            "--listen" => {
                let text = value("--listen")?;
                options.listen = text
                    .parse()
                    .map_err(|e| format!("--listen {text}: {e} (expected ADDR:PORT)"))?;
            }
            "--root" => options.root = Some(value("--root")?.into()),
            "--ui" => options.ui = Some(value("--ui")?.into()),
            "-h" | "--help" => return Err(USAGE.to_string()),
            other => return Err(format!("{other} is not an option\n{USAGE}")),
        }
    }
    Ok(options)
}

fn run() -> Result<(), String> {
    let options = parse(std::env::args().skip(1))?;
    check_bind(options.listen.ip())?;

    let root = options
        .root
        .or_else(default_root)
        .ok_or("no works folder: pass --root or set SCE_WORKS_DIR")?;
    let assets = options
        .ui
        .map(|dir| Assets::new(&dir).map_err(|e| format!("--ui {}: {e}", dir.display())))
        .transpose()?;

    let (token, generated) = match std::env::var("SCE_WEB_TOKEN") {
        Ok(t) if t.len() >= token::MIN_LENGTH => (t, false),
        Ok(_) => {
            return Err(format!(
                "SCE_WEB_TOKEN is shorter than {} characters",
                token::MIN_LENGTH
            ))
        }
        Err(_) => (
            token::generate().map_err(|e| format!("no random source for a token: {e}"))?,
            true,
        ),
    };

    let listener = std::net::TcpListener::bind(options.listen)
        .map_err(|e| format!("cannot listen on {}: {e}", options.listen))?;
    listener
        .set_nonblocking(true)
        .map_err(|e| format!("cannot use the listener: {e}"))?;
    let bound = listener
        .local_addr()
        .map_err(|e| format!("the listener has no address: {e}"))?;

    let figures = default_renderer();
    eprintln!("works folder: {}", root.display());
    eprintln!(
        "SCE generator: {}",
        SceCodegen::discover().map_or_else(
            || "none found (set SCE_CODEGEN); models are stored but not drawn".to_string(),
            |g| g.program().display().to_string()
        )
    );
    if generated {
        eprintln!("open:         http://{bound}/#token={token}");
    } else {
        eprintln!("open:         http://{bound}/#token=<SCE_WEB_TOKEN>");
    }

    // The same executor the desktop application hosts, so that the screen being developed here
    // can be pressed "generate" on and be answered. Held to the end of the program: it stops
    // a client at work when the server stops, and `SCE_EXECUTOR=off` hosts nothing.
    let _executor = host::start(
        Arc::new(WorkStore::at(root.clone())),
        Arc::from(default_renderer()),
        HostSettings::from_environment("web-shell"),
    );
    match _executor.not_hosted() {
        None => eprintln!(
            "executor:     Claude Code {}",
            _executor.client_version().unwrap_or_default()
        ),
        Some(why) => eprintln!("executor:     none ({why})"),
    }

    let shell = Arc::new(Shell::new(WorkStore::at(root), figures, token, assets));
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("cannot start the runtime: {e}"))?;
    runtime.block_on(async {
        let listener = tokio::net::TcpListener::from_std(listener)
            .map_err(|e| format!("cannot use the listener: {e}"))?;
        serve(shell, listener, Limits::default(), async {
            // A stop request is the only thing that ends the server.
            let _ = tokio::signal::ctrl_c().await;
        })
        .await;
        Ok(())
    })
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::from(2)
        }
    }
}
