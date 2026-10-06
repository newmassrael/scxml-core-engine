// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! `sce-web-shell [--listen ADDR:PORT] [--root DIR] [--settings DIR] [--ui DIR] [--claude-connection]`
//!
//! The token comes from `SCE_WEB_TOKEN` (at least 16 characters) or is generated
//! and printed in the URL to open. The settings folder (`--settings`, else
//! `SCE_SETTINGS_DIR`, else the per-user configuration directory) is read by what a browser
//! asks and never changed by it: changing a person's settings is the desktop application's.
//!
//! `--claude-connection` is the one exception, and it is the operator's: the person who starts
//! this program, with no window to choose in. When nobody has a default connection it saves one to
//! Claude Code (the model `SCE_CLAUDE_MODEL` names, when set) and makes it the default, so that
//! the screen in the browser can ask for a model. A default somebody chose is not replaced.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use sce_app_core::host::{self, HostSettings};
use sce_app_core::{
    default_renderer, default_root, default_settings_root, AdapterKind, AuthSource, Connection,
    ConnectionId, ConnectionStore, Limits as RunLimits, Policy, SceCodegen, WorkStore,
};
use sce_web_shell::address::check_bind;
use sce_web_shell::assets::Assets;
use sce_web_shell::server::{serve, Limits};
use sce_web_shell::{token, Shell};

const DEFAULT_LISTEN: &str = "127.0.0.1:5174";

const USAGE: &str = "usage: sce-web-shell [--listen ADDR:PORT] [--root DIR] [--settings DIR] \
                     [--ui DIR] [--claude-connection]";

struct Options {
    listen: SocketAddr,
    root: Option<PathBuf>,
    settings: Option<PathBuf>,
    ui: Option<PathBuf>,
    claude_connection: bool,
}

fn parse(args: impl Iterator<Item = String>) -> Result<Options, String> {
    let mut options = Options {
        listen: DEFAULT_LISTEN
            .parse()
            .expect("the default address is valid"),
        root: None,
        settings: None,
        ui: None,
        claude_connection: false,
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
            "--settings" => options.settings = Some(value("--settings")?.into()),
            "--ui" => options.ui = Some(value("--ui")?.into()),
            "--claude-connection" => options.claude_connection = true,
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
    // Read for a browser, never changed by one. A shell with no settings folder says so to a screen
    // that asks.
    let settings = options.settings.or_else(default_settings_root);
    if options.claude_connection {
        let dir = settings.as_ref().ok_or(
            "--claude-connection needs a settings folder: pass --settings, or set SCE_SETTINGS_DIR",
        )?;
        let first = Connection {
            id: ConnectionId::parse("claude").expect("`claude` is a connection id"),
            adapter: AdapterKind::ClaudeCode,
            display_name: None,
            executable: None,
            model: std::env::var("SCE_CLAUDE_MODEL")
                .ok()
                .filter(|model| !model.trim().is_empty()),
            auth: AuthSource::OfficialLogin,
            server_url: None,
            limits: RunLimits::default(),
        };
        let given = ConnectionStore::at(dir.clone())
            .ensure_default(&first)
            .map_err(|e| format!("--claude-connection: {e}"))?;
        eprintln!(
            "connection:   claude, {}",
            if given {
                "saved as the default"
            } else {
                "left as it was: a default is already chosen"
            }
        );
    }
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
    match &settings {
        Some(dir) => eprintln!("settings:     {} (read only here)", dir.display()),
        None => eprintln!("settings:     none (pass --settings, or set SCE_SETTINGS_DIR)"),
    }
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
    let _executor = host::start_with(
        Arc::new(WorkStore::at(root.clone())),
        Arc::from(default_renderer()),
        HostSettings::from_environment("web-shell"),
        // It runs the requests made for a connection too, when there is a settings folder to
        // read them from.
        settings
            .clone()
            .map(|dir| (ConnectionStore::at(dir), Policy::shipped())),
    );
    match _executor.not_hosted() {
        None => eprintln!(
            "executor:     Claude Code {}",
            _executor.client_version().unwrap_or_default()
        ),
        Some(why) => eprintln!("executor:     none ({why})"),
    }

    let mut shell = Shell::new(WorkStore::at(root), figures, token, assets);
    if let Some(dir) = settings {
        shell = shell.with_settings(ConnectionStore::at(dir));
    }
    let shell = Arc::new(shell);
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
