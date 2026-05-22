// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Hand-curated integration fixtures (non-W3C-IRP).
//!
//! See `docs/SCE_INTEGRATION_FIXTURE_LAYOUT.md` for the per-backend
//! policy governing this layer. Each sub-module is a state machine
//! generated from a fixture under `sce-rust-tests/fixtures/<stem>.scxml`
//! by a dedicated regen script (e.g.
//! `scripts/regen_donedata_local_invoke.sh`). The files under each
//! sub-directory carry the standard §6.2.6 `SCE-GENERATED` drift
//! header and `SCE-MAP:` markers; only this `mod.rs` is hand-authored,
//! so the W3C regen pipeline (`sce-codegen generate-w3c -l rust`)
//! never touches it and adding or dropping an integration fixture is
//! a single-line edit here.

pub mod donedata_local_invoke;
