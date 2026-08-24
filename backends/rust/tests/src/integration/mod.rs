// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Hand-curated integration fixtures (non-W3C-IRP).
//!
//! See `docs/SCE_INTEGRATION_FIXTURE_LAYOUT.md` for the per-backend
//! policy governing this layer. Each sub-module is a state machine
//! generated from the canonical fixture at
//! `integration_resources/<stem>/<stem>.scxml` by a dedicated regen script
//! (e.g. `scripts/regen_donedata_local_invoke.sh`). The files under each
//! sub-directory carry the standard §6.2.6 `SCE-GENERATED` drift
//! header and `SCE-MAP:` markers; only this `mod.rs` is hand-authored,
//! so the W3C regen pipeline (`sce-codegen generate-w3c -l rust`)
//! never touches it and adding or dropping an integration fixture is
//! a single-line edit here.
//!
//! Two entries do not come from there. `ai_loop` is generated from the
//! worked example at `examples/ai_loop/ai_loop.scxml`, so that a document
//! shipped as an example is asserted by two engines rather than one: the
//! C++ driver next to it and `tests/ai_loop.rs` here.
//! `integration_stem_registration` enumerates `integration_resources/` and
//! so does not reach it — the example is not on the seven-channel contract,
//! and its regen script says why.
//!
//! `parallel_region_root_external_domain` is the other, and it is outside for
//! the opposite reason: it is a document the seven-channel contract would be
//! premature for. It pins the Appendix D rule that a `<parallel>` is never a
//! transition domain, and the Go, Python and C11 engines still resolve a
//! region root's external transition to the enclosing `<parallel>`. Promoting
//! the stem would register coverage this repository does not have; it moves
//! under `integration_resources/` when those engines are repaired.

pub mod ai_loop;
pub mod ancestor_entry_is_not_default_entry;
pub mod autoforward_dequeue_point;
pub mod autoforward_done_invoke;
pub mod autoforward_event_fields;
pub mod autoforward_internal_queue;
pub mod discarded_event_is_observable;
pub mod donedata_late_completion;
pub mod donedata_local_invoke;
pub mod empty_finalize_updates_the_location;
pub mod error_cascade_is_bounded;
pub mod event_data_arrives_as_sent;
pub mod event_origin_is_a_location;
pub mod event_schema_native;
pub mod eventless_macrostep_is_bounded;
pub mod host_event_reaches_the_child;
pub mod host_processor;
pub mod internal_chain_is_bounded;
pub mod invoke_param_error_starts_the_child;
pub mod invoke_param_seeds_declared_child_data;
pub mod invoke_precedes_dequeue_midrun;
pub mod invoke_precedes_external_dequeue;
pub mod invoke_unsupported_type;
pub mod late_tick_honours_cancel;
pub mod native_action;
pub mod nested_final_not_terminal;
pub mod parallel_completion_raises_done_state;
pub mod parallel_done_state_is_delivered;
pub mod parallel_region_root_external_domain;
pub mod parallel_regions_take_own_transitions;
pub mod parallel_self_transition_keeps_its_leaf;
pub mod send_namelist_over_http;
pub mod send_param_payload;
pub mod session_ids_are_distinct;
pub mod targetless_transition_completes_macrostep;
pub mod undecodable_payload_is_reported;
pub mod unhandled_error_is_observable;
pub mod unseen_event_is_reported;
pub mod xml_data_is_a_dom_tree;
