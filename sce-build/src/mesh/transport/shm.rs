//! SCE_MESH.md §9.6 L1399 (b) — per-transport scxml-invoke codegen helpers
//! for `shm` (same-device cross-partition implicit fallback).
//!
//! Shared-memory peers consume no deploy.yaml fields beyond the implicit
//! "same-device cross-partition" classifier decision: the template emits
//! two `ShmChannel<>` members per peer (one per direction) driven by
//! `pumpScxmlInvokeRequests` / `pumpScxmlInvokeReplies` polling. There is
//! no per-peer resolver to own here — this module exists to complete the
//! `mesh::transport::{shm,custom_tcp,someip}` boundary called out in
//! §9.6 L1399 (b) so follow-on wire additions have a single place to land
//! shm-specific resolution if any ever arrives.
//!
//! Keeping the file present (rather than collapsing shm work into
//! `mod.rs`) preserves the discoverability promise of L1399 (b): a
//! reader finding `transport/custom_tcp.rs` next to a sibling
//! `transport/someip.rs` will expect a matching `transport/shm.rs`.
