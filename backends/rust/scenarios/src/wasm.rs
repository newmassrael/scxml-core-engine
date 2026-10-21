// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The wasm32 entry of the scenario table: what a host script calls to learn the
//! scenarios and to replay one. A host runs each scenario in an instance of its
//! own (`wasm/run.mjs`): a panic on `wasm32-unknown-unknown` stops the module and
//! cannot be caught inside it, so one failing scenario must not take the others
//! down with it.

use crate::machines;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn error(message: &str);
}

/// A panic on this target prints nowhere unless something writes it: its message
/// is the whole of why a scenario failed, so it goes to the host's console.
fn report_panics() {
    std::panic::set_hook(Box::new(|info| error(&info.to_string())));
}

/// The scenarios the table holds, separated by commas.
#[wasm_bindgen]
pub fn scenario_names() -> String {
    machines::NAMES.join(",")
}

/// The scenarios of the directory the table has no row for yet
/// ([`machines::NOT_YET_LOWERED`]), separated by commas: a host holds the module
/// to every scenario of the directory, and these are accounted for without it.
#[wasm_bindgen]
pub fn scenarios_not_yet_lowered() -> String {
    machines::NOT_YET_LOWERED
        .iter()
        .map(|(name, _)| *name)
        .collect::<Vec<_>>()
        .join(",")
}

/// Replay the scenario `name`. Panics, with the step and what differed, where the
/// machine does not do what the scenario holds it to.
#[wasm_bindgen]
pub fn run_scenario(name: &str) {
    report_panics();
    machines::run(name);
}
