// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Hand-curated integration fixtures (non-W3C-IRP).
//!
//! See `docs/SCE_INTEGRATION_FIXTURE_LAYOUT.md` for the per-backend
//! policy governing this layer. Each sub-module is a state machine
//! generated from a fixture under `backends/rust/tests/fixtures/<stem>.scxml`
//! by a dedicated regen script (e.g.
//! `scripts/regen_donedata_local_invoke.sh`). The files under each
//! sub-directory carry the standard §6.2.6 `SCE-GENERATED` drift
//! header and `SCE-MAP:` markers; only this `mod.rs` is hand-authored,
//! so the W3C regen pipeline (`sce-codegen generate-w3c -l rust`)
//! never touches it and adding or dropping an integration fixture is
//! a single-line edit here.

pub mod autoforward_dequeue_point;
pub mod autoforward_done_invoke;
pub mod autoforward_event_fields;
pub mod autoforward_internal_queue;
pub mod donedata_late_completion;
pub mod donedata_local_invoke;
pub mod event_schema_native;
pub mod invoke_precedes_dequeue_midrun;
pub mod invoke_precedes_external_dequeue;
pub mod native_action;
pub mod nested_final_not_terminal;
pub mod send_param_payload;
