// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.3: a typed `<data>` whose id is a Rust keyword, or no Rust
// identifier at all, still gets a reader, and it reads its own variable.
// Rust AOT.
//
// `<data id="box">` used to generate `pub fn box(&self)`, which does not
// compile. `sce-build/src/reader_names.rs` now spells it `r#box` — the
// language's own escape — and folds `screen-rules` to `screen_rules`. The ids
// no backend can give a reader (`auto`, `self`, `new`, `start`, `t`, and `a-b`
// beside `a_b`) get none here either; this file compiling is what shows none
// of them was emitted as a duplicate or a keyword.
//
// Fixture: integration_resources/typed_reader_names/typed_reader_names.scxml
// (canonical, shared with the C++ / C11 / Go / Kotlin / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_typed_reader_names.sh

use std::sync::Arc;

use sce_rust_runtime::{Engine, IScriptEngine};
use sce_rust_tests::integration::typed_reader_names::{
    TypedReaderNamesEvent as Event, TypedReaderNamesPolicy as Policy,
};

fn started() -> Engine<Policy> {
    let script_engine: Arc<dyn IScriptEngine> = Arc::new(sce_rust_lua::LuaEngine::new());
    let mut engine = Engine::new(Policy::new(script_engine));
    engine.initialize();
    engine
}

/// Each escaped reader reads the value its own variable was declared with.
#[test]
fn an_escaped_reader_reads_its_own_variable() {
    let engine = started();
    let p = engine.policy();
    assert_eq!(p.r#box(), Some(1));
    assert_eq!(p.object(), Some(2));
    assert_eq!(p.pass(), Some(3));
    assert_eq!(p.screen_rules(), Some(4));
    assert_eq!(p.a_b(), Some(10));
}

/// And the value it holds now: `bump` adds 10 to four of them, and leaves
/// `screen-rules` — which no ECMAScript expression can name — alone.
#[test]
fn an_escaped_reader_reads_the_live_value() {
    let mut engine = started();
    engine.raise_external(Event::Bump, "", "");
    engine.step();
    let p = engine.policy();
    assert_eq!(p.r#box(), Some(11));
    assert_eq!(p.object(), Some(12));
    assert_eq!(p.pass(), Some(13));
    assert_eq!(p.screen_rules(), Some(4));
    assert_eq!(p.a_b(), Some(20));
}
