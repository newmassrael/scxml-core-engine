// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The standard Mesh documents (`sce:std/mesh`), generated.
//!
//! GENERATED -- DO NOT EDIT (backends/rust/mesh/generate.sh).
//! Each module is one document; they name one another as siblings
//! (`super::envelope_id`), which is why they share this parent.

pub mod dedup_admit;
pub mod dedup_holds;
pub mod envelope;
pub mod envelope_id;
pub mod ordering_drain;
pub mod ordering_gap_end;
pub mod ordering_hold;
pub mod ordering_prune;
pub mod ordering_slot;
pub mod outbound_overflows;
pub mod outbound_sends_now;
pub mod outbound_stale;
pub mod pattern_kind;
pub mod payload_codec;
pub mod retry_exhausted;
pub mod retry_jittered;
pub mod retry_next_backoff;
pub mod rpc_status;
