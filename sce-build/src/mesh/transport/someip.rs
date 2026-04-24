//! SCE_MESH.md §9.6 L1399 (b) — per-transport scxml-invoke codegen helpers
//! for `someip`. Introduced as part of Session 4's wire-14/20 landing; the
//! sibling `custom_tcp` module ported its `resolve_connect_endpoint` helper
//! in the same commit, and this file fills the third slot called out in
//! §9.6 L1399 (b) so zenoh (Session 5 candidate) plugs in without a
//! four-transport simultaneous split.
//!
//! At Session 4 A4 scope the module body is intentionally empty: the
//! codegen template's someip branch — landed in Session 4 A5 — owns the
//! SCE-reserved `service_id` / `instance_id` / `method_id` constants
//! required for wire-14/20 dispatch and imports them from here at that
//! commit. Richer author-facing fields (custom service names, multi-peer
//! instance carving, collision detection against OEM-owned
//! `vsomeip.json` service tables) land when the first consumer forces
//! them; keeping the module present at A4 documents the boundary
//! without shipping infrastructure ahead of a consumer.
