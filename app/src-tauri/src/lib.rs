// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The desktop shell.
//!
//! It holds no logic of its own. The window shows `ui/`, and the one command it
//! offers the screen, `sce_call`, hands the command's name and arguments to
//! `sce_app_core::call` and hands the answer back. The browser shell used during
//! development offers the same single entrance, so the screen cannot behave
//! differently in the two for a reason that lives in the shell.
//!
//! Permissions are the least a window needs: no file system, no shell, no
//! network. The works folder is reached only through the store, which is the
//! only code that touches it.

use sce_app_core::{call, default_renderer, default_root, CommandError, Product, WorkStore};
use serde_json::Value;
use tauri::Manager;

/// The works folder this window works on, and the product that draws a model
/// and reads one for it.
struct Works {
    store: WorkStore,
    figures: Box<dyn Product>,
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
    call(
        &works.store,
        works.figures.as_ref(),
        &name,
        args.unwrap_or(Value::Null),
    )
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // `SCE_WORKS_DIR`, then the per-user data directory: the same place
            // the authoring MCP's `sce-work` resolves to, so the two see one folder.
            let root = default_root().unwrap_or_else(|| {
                app.path()
                    .app_data_dir()
                    .expect("the platform has no per-user data directory")
                    .join("works")
            });
            app.manage(Works {
                store: WorkStore::at(root),
                figures: default_renderer(),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![sce_call])
        .run(tauri::generate_context!())
        .expect("the application could not start");
}
