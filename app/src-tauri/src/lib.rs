// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The desktop shell.
//!
//! It holds no logic of its own. The window shows `ui/`, and the command it
//! offers the screen, `sce_call`, hands the command's name and arguments to
//! `sce_app_core::call` and hands the answer back. The browser shell used during
//! development offers the same single entrance, so the screen cannot behave
//! differently in the two for a reason that lives in the shell.
//!
//! The two others are about the window itself: a browser tab is asked "leave this
//! page?" by the browser, and a window is not, so the screen reports whether it holds
//! unsaved changes (`sce_unsaved`) and says when the person has decided the window
//! may close (`sce_close`); [`close_gate`] is what keeps the window open in between.
//!
//! Permissions are the least a window needs: no file system, no shell, no
//! network. The works folder is reached only through the store, which is the
//! only code that touches it.

mod close_gate;

use std::sync::{Arc, Mutex};
use std::time::Instant;

use close_gate::{CloseGate, Decision};
use sce_app_core::host::{self, ExecutorHost, HostSettings};
use sce_app_core::installed::{self, Installed};
use sce_app_core::{
    call_in, default_root, default_settings_root, renderer_with_bundle, CommandError,
    ConnectionStore, Context, Entrance, Policy, Product, WorkStore,
};
use serde_json::Value;
use tauri::{Manager, RunEvent, WindowEvent};

/// What the shell asks the screen to run when a window that holds unsaved changes is
/// asked to close. A fixed script with nothing of the person's in it.
const ASK_THE_SCREEN: &str = "window.sceCloseRequested && window.sceCloseRequested()";

/// The folder of the application's resources an installer puts the bundle in (`tauri.conf.json`
/// names it, and `scripts/package_app.sh` fills it).
const BUNDLE_FOLDER: &str = "sce-author";

/// The works folder this window works on, the product that draws a model and reads one for it,
/// and the person's own settings, which only this window changes.
struct Works {
    store: WorkStore,
    figures: Box<dyn Product>,
    settings: ConnectionStore,
    policy: Policy,
    /// The program the environment named as Claude Code (`SCE_CLAUDE`); none is the first one on
    /// the search path, which is looked for each time it is asked about, so that installing it
    /// while the window is open is seen by the next check.
    claude: Option<std::path::PathBuf>,
    /// The program the environment named as Codex (`SCE_CODEX`), looked for the same way.
    codex: Option<std::path::PathBuf>,
}

/// The executor this application hosts, when it found what it needs: a runner on a thread of
/// its own that takes the requests the owner makes. Held so that it is stopped when the
/// application ends, and a client at work is killed with it and not left running.
struct Executor(Mutex<Option<ExecutorHost>>);

/// Host the application's own executor, or say why it does not. Not having Claude Code is not
/// an error: the application shows that no AI is connected, and why, and works as it always did.
/// What it could not do is said to the works folder, where the screen reads it, and also here.
fn host_the_executor(
    root: std::path::PathBuf,
    connections: ConnectionStore,
    bundle: &Installed,
) -> Executor {
    let store = Arc::new(WorkStore::at(root));
    let product: Arc<dyn Product> = Arc::from(renderer_with_bundle(bundle.codegen.as_deref()));
    let settings = HostSettings::from_environment("desktop").with_bundle(bundle);
    // The executor runs the requests made for a connection, for the connections it can run: it
    // reads the person's settings, and the build's table of ways of signing in judges them.
    let running = host::start_with(
        store,
        product,
        settings,
        Some((connections, Policy::shipped())),
    );
    if let Some(why) = running.not_hosted() {
        eprintln!("sce-workbench: {why}");
    }
    Executor(Mutex::new(Some(running)))
}

/// Run one of the store's commands. The screen's only way to anything.
///
/// `async` here means "not on the thread that draws the window": drawing a model
/// runs the SCE generator, which may take seconds, and a command on the main
/// thread would freeze the window for all of them.
#[tauri::command(async)]
fn sce_call(
    works: tauri::State<'_, Works>,
    name: String,
    args: Option<Value>,
) -> Result<Value, CommandError> {
    // This is the person at the keyboard: the one entrance that may change their settings.
    let context = Context::new(
        &works.store,
        works.figures.as_ref(),
        &works.policy,
        Entrance::Desktop,
    )
    .with_connections(Some(&works.settings))
    .with_claude(works.claude.as_deref())
    .with_codex(works.codex.as_deref());
    call_in(&context, &name, args.unwrap_or(Value::Null))
}

/// The screen's report of whether it holds changes the core has not been given.
#[tauri::command]
fn sce_unsaved(gate: tauri::State<'_, CloseGate>, unsaved: bool) {
    gate.unsaved(unsaved, Instant::now());
}

/// The person decided the window may close, with or without what was unsaved.
#[tauri::command]
fn sce_close(gate: tauri::State<'_, CloseGate>, window: tauri::WebviewWindow) {
    gate.allow();
    // A window that cannot be closed here is closed by the application ending.
    if window.close().is_err() {
        window.app_handle().exit(0);
    }
}

pub fn run() {
    tauri::Builder::default()
        .manage(CloseGate::new(Instant::now()))
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let gate = window.state::<CloseGate>();
                if gate.on_close_requested(Instant::now()) == Decision::Ask {
                    api.prevent_close();
                    if let Some(webview) = window.app_handle().get_webview_window(window.label()) {
                        // Failing to ask leaves the window open; the next request
                        // after the wait closes it (see `close_gate`).
                        let _ = webview.eval(ASK_THE_SCREEN);
                    }
                }
            }
        })
        .setup(|app| {
            // `SCE_WORKS_DIR`, then the per-user data directory: the same place
            // the authoring MCP's `sce-work` resolves to, so the two see one folder.
            let root = default_root().unwrap_or_else(|| {
                app.path()
                    .app_data_dir()
                    .expect("the platform has no per-user data directory")
                    .join("works")
            });
            // What an installer carried: the generator, `sce-work` and the authoring server, in a
            // folder of the application's own resources. A development build carries none, and
            // finds them as it always did (the environment, beside the program, the search path).
            let bundle = app
                .path()
                .resource_dir()
                .map(|dir| installed::in_bundle(&dir.join(BUNDLE_FOLDER)))
                .unwrap_or_default();
            // The person's settings are kept apart from the works: a works folder is shared and
            // moved, and whose account a person reaches a model with is no part of a work.
            let settings = default_settings_root().unwrap_or_else(|| {
                app.path()
                    .app_config_dir()
                    .expect("the platform has no per-user configuration directory")
                    .join("settings")
            });
            app.manage(host_the_executor(
                root.clone(),
                ConnectionStore::at(settings.clone()),
                &bundle,
            ));
            // The programs the environment named as the clients, which the status asked of each
            // is about.
            let named = HostSettings::from_environment("desktop");
            app.manage(Works {
                store: WorkStore::at(root),
                figures: renderer_with_bundle(bundle.codegen.as_deref()),
                settings: ConnectionStore::at(settings),
                policy: Policy::shipped(),
                claude: named.claude,
                codex: named.codex,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![sce_call, sce_unsaved, sce_close])
        .build(tauri::generate_context!())
        .expect("the application could not start")
        .run(|app, event| {
            if let RunEvent::Exit = event {
                // Stopped here and not left to the end of the process: a client at work is a
                // child process, and one that outlives the window is a run nobody is waiting for.
                if let Some(running) = app
                    .state::<Executor>()
                    .0
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .take()
                {
                    running.stop();
                }
            }
        });
}
