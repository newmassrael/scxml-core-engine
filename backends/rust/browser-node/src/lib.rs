// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The WebAssembly module of a browser node (docs/adr/0117, D2): the HTTPS binding's client
//! core, behind the half of the browser shell that runs in the module
//! (`sce_rust_mesh::https::browser`), exported to the page's script as the class `Client`.
//!
//! Every method of `Client` is one of `BrowserClient`'s, and answers what it does: the effects
//! of the step, as the JSON that module documents. A refusal is thrown as a JavaScript `Error`
//! whose message says why. The page's script that carries the effects out is `js/node.mjs`.
//!
//! Only the wasm32 build exports anything; on any other target this crate is empty, and the
//! module's half is built and tested where it is written, in `sce-rust-mesh` under the feature
//! `browser`.

#[cfg(target_arch = "wasm32")]
mod module;
