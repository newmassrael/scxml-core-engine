//! SCE_MESH.md §mesh-9.6 (b) — per-transport scxml-invoke codegen helpers
//! for `custom_tcp`. Lives under `mesh::transport::custom_tcp` alongside the
//! sibling `shm` / `someip` modules; the parent `transport` module stays
//! focused on cross-transport descriptor metadata (capabilities, ordering,
//! dedup, readiness) while this file owns the custom_tcp-specific parts
//! of wire-14/20 per-peer emission context.
//!
//! The public surface is intentionally narrow at this landing — only the
//! helper consumed by [`crate::collect_scxml_remote_peers`] is exposed.
//! Future work (e.g. device-level `listen:` lifting from §mesh-9.6 L1397 if
//! multi-machine-per-device ever lands) continues here so lib.rs does not
//! re-grow inline per-transport branches.

use crate::mesh;

/// Resolve the cross-device connect endpoint for a `custom_tcp` scxml-remote
/// invoke peer. Returns `None` unless all three conditions hold:
///
///   1. `transport == Some("custom_tcp")` — this is the only §mesh-9.6 transport
///      that consumes a peer connect endpoint at template time (shm emits
///      two `ShmChannel<>` members directly from model fields; someip uses
///      SCE-reserved service/instance/method constants baked into the
///      codegen template).
///   2. The peer resolves to a topology device.
///   3. That device declares `transports.custom_tcp.listen:`.
///
/// Other transports keep `None` and the template's `custom_tcp` branch is
/// the only consumer of this field. When all three conditions hold but the
/// listen value is empty, the downstream validator
/// (`validate_scxml_invoke_transport` custom_tcp branch) rejects the
/// deployment — this helper stays narrow and fact-only so the acceptance
/// decision lives in one place.
///
/// The argument order — deploy config before transport string before peer —
/// matches `collect_scxml_remote_peers`'s call site and avoids re-ordering
/// churn at the caller.
pub fn resolve_connect_endpoint(
    deploy_cfg: &mesh::deploy::DeployConfig,
    transport: Option<&str>,
    peer_name: &str,
) -> Option<String> {
    if transport != Some("custom_tcp") {
        return None;
    }
    deploy_cfg
        .device_for_machine(peer_name)
        .and_then(|device| device.transports.custom_tcp.as_ref())
        .and_then(|cfg| cfg.listen.clone())
}
