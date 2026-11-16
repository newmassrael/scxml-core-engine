// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Hand-curated integration fixtures (non-W3C-IRP).
//!
//! See `docs/SCE_INTEGRATION_FIXTURE_LAYOUT.md` for the per-backend
//! policy governing this layer. Each sub-module is a state machine
//! generated from the canonical fixture at
//! `integration_resources/<stem>/<stem>.scxml` by the actions declared in
//! `tools/codegen/trees/rust-integration.toml`, and `static_datamodel` by
//! those of `rust-static-datamodel.toml`. The crate's build script writes
//! both trees into `OUT_DIR` and declares a module for each directory they
//! placed there (docs/adr/0097), so adding or dropping an integration fixture
//! is an edit of the tree manifest alone. The files under each sub-directory,
//! their `mod.rs` included, carry a generated header and `SCE-MAP:` markers;
//! only this `mod.rs` is hand-authored.
//!
//! Two entries do not come from there. `ai_loop` is generated from the
//! worked example at `examples/ai_loop/ai_loop.scxml`, so that a document
//! shipped as an example is asserted by two engines rather than one: the
//! C++ driver next to it and `tests/ai_loop.rs` here.
//! `integration_stem_registration` enumerates `integration_resources/` and
//! so does not reach it — the example is not on the seven-channel contract,
//! and its generation in the tree manifest says why.
//!
//! `parallel_region_root_external_domain` is the other, and it is outside for
//! the opposite reason: it is a document the seven-channel contract would be
//! premature for. It pins the Appendix D rule that a `<parallel>` is never a
//! transition domain, and the Go, Python and C11 engines still resolve a
//! region root's external transition to the enclosing `<parallel>`. Promoting
//! the stem would register coverage this repository does not have; it moves
//! under `integration_resources/` when those engines are repaired.
//!
//! `a_child_timer_is_a_deadline_of_its_parent` is the third, outside for a
//! reason of its own: it asks a host's question — when does this machine next
//! need a tick? — and C11 and the C++ Interpreter have no such query, so there
//! is no driver for them to carry and no harness to wait for. It lives at
//! `tests/integration/a_child_timer_is_a_deadline_of_its_parent.scxml`, beside
//! its C++ AOT driver.
//!
//! `external_chain_is_bounded` is the fourth, outside for the arity reason: it
//! holds an engine to ARCHITECTURE.md "External-Event Budget", engine by engine,
//! and a stem under `integration_resources/` is a seven-channel contract. It
//! lives at `tests/integration/external_chain_is_bounded.scxml`, beside its
//! drivers.
//!
//! `a_static_child_declares_its_data_with_a_handed_value` is the fifth: it
//! holds docs/adr/0098 for a `datamodel="sce-static"` child, whose handed
//! values are typed, which the script-engine fixture of that ADR
//! (`a_child_takes_a_handed_value_in_place_of_its_data`, a stem of every
//! channel) does not reach. It lives at `tests/integration/<stem>.scxml`.

// One module per directory the trees placed under src/integration, each read
// from its `mod.rs` in OUT_DIR: `#[path]` cannot take `env!("OUT_DIR")`, so
// the build script writes the declarations.
include!(concat!(env!("OUT_DIR"), "/integration.rs"));
