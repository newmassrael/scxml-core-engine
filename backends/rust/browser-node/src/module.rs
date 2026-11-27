// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The class `Client` the page's script holds: [`BrowserClient`] through wasm-bindgen.

use std::sync::Once;

use sce_rust_mesh::https::browser::{BrowserClient, BrowserError};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn error(message: &str);
}

/// A panic on this target prints nowhere unless something writes it, and stops the module: its
/// message is the whole of why, so it goes to the page's console.
fn report_panics() {
    static HOOK: Once = Once::new();
    HOOK.call_once(|| std::panic::set_hook(Box::new(|info| error(&info.to_string()))));
}

fn refused(why: BrowserError) -> JsError {
    JsError::new(&why.to_string())
}

/// The client core of a browser node. Each method gives it one input and answers the effects of
/// the step as JSON (`sce_rust_mesh::https::browser`).
#[wasm_bindgen]
pub struct Client {
    inner: BrowserClient,
}

#[wasm_bindgen]
impl Client {
    /// A client with `settings` (JSON) whose backoffs are spread from `seed` (32 random bytes).
    #[wasm_bindgen(constructor)]
    pub fn new(settings: &str, seed: &[u8]) -> Result<Client, JsError> {
        report_panics();
        BrowserClient::start(settings, seed)
            .map(|inner| Client { inner })
            .map_err(refused)
    }

    /// The `keyid` the node's requests are signed under.
    #[wasm_bindgen(js_name = keyId)]
    pub fn key_id(&self) -> String {
        self.inner.key_id()
    }

    pub fn started(&mut self) -> String {
        self.inner.started()
    }

    pub fn transmitted(&mut self, envelope: &[u8]) -> String {
        self.inner.transmitted(envelope)
    }

    pub fn drawn(&mut self, lane: &str, random: &[u8], moment_ms: f64) -> Result<String, JsError> {
        self.inner.drawn(lane, random, moment_ms).map_err(refused)
    }

    pub fn signed(&mut self, lane: &str, signature: &[u8]) -> Result<String, JsError> {
        self.inner.signed(lane, signature).map_err(refused)
    }

    pub fn answered(
        &mut self,
        lane: &str,
        status: u16,
        headers: &str,
        body: &[u8],
    ) -> Result<String, JsError> {
        self.inner
            .answered(lane, status, headers, body)
            .map_err(refused)
    }

    pub fn failed(&mut self, lane: &str, kind: &str, words: &str) -> Result<String, JsError> {
        self.inner.failed(lane, kind, words).map_err(refused)
    }

    pub fn woken(&mut self, lane: &str) -> Result<String, JsError> {
        self.inner.woken(lane).map_err(refused)
    }

    pub fn received(&self) -> String {
        self.inner.received()
    }

    pub fn held(&mut self, received: &str) -> Result<String, JsError> {
        self.inner.held(received).map_err(refused)
    }

    pub fn stop(&mut self) -> String {
        self.inner.stop()
    }

    pub fn stopped(&self) -> bool {
        self.inner.stopped()
    }
}
