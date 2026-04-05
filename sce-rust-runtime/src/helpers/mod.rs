// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! W3C SCXML algorithm helpers.
//!
//! Ports the C++ helper headers at `sce/include/core/` (25 files) and
//! `sce/include/common/` (26 files). All helpers here are pure functions over
//! `P: StatePolicy`, matching the C++ pattern of template specialization
//! (e.g., `HierarchicalStateHelper<StatePolicy>::findLCA(a, b)`).
//!
//! ## Phase 1 scope
//!
//! Phase 1 ships only the helpers required to boot the engine loop and run
//! hand-crafted tests:
//!
//! - [`hierarchy`]: LCA, entry/exit chain construction (`HierarchicalStateHelper`)
//! - [`event_queue`]: FIFO internal/external queues (`EventQueueManager`)
//! - [`logger`]: thin `log` crate re-exports (`SCE_LOG_*` macros)
//! - [`scxml_constants`]: W3C URIs and string literals
//! - [`state_policy_concepts`]: Rust trait bounds replacing C++20 concepts
//!
//! Phases 2–4 add the remaining helpers (conflict resolution, parallel states,
//! history, send, invoke, finalize, foreach, guard, datamodel init, etc.).

pub mod event_queue;
pub mod hierarchy;
pub mod logger;
pub mod scxml_constants;
pub mod state_policy_concepts;
