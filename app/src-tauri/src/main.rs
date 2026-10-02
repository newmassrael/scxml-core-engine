// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// A release build on Windows is a window, not a console program.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    sce_workbench::run();
}
