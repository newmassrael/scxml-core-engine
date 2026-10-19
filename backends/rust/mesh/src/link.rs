// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What every binding that moves bytes for the router core tells its host, and the one place that
//! hands it to the core.
//!
//! A binding keeps no decision. It moves envelope bytes and readiness between a medium and the
//! core: a peer became ready, bytes arrived from it, it was lost, an envelope was queued for it
//! and never written. The host feeds those, as [`LinkEvent`]s, to its [`Endpoint`] with
//! [`deliver`], on the thread that steps the engine. The WebSocket binding (`wss`) and the Zenoh
//! binding (`zenoh_inbox`, SCE_MESH.md §mesh-20) differ in the medium and agree on this, so a host moves
//! from one to the other by changing what produces the events.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::endpoint::{Endpoint, Environment, Transport, TransportFailure};

/// What a link tells its host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkEvent {
    /// `peer`'s binding is ready.
    Ready(String),
    /// A binary message from `peer`: one envelope's bytes.
    Received(String, Vec<u8>),
    /// An envelope queued for `peer` that the link closed before writing.
    Unsent(String, Vec<u8>),
    /// The link to `peer` closed, failed, or stopped answering; the words say which.
    Lost(String, String),
}

/// Hand a link's event to the endpoint it concerns.
pub fn deliver<T: Transport, E: Environment>(endpoint: &mut Endpoint<T, E>, event: LinkEvent) {
    match event {
        LinkEvent::Ready(peer) => endpoint.peer_ready(&peer),
        LinkEvent::Received(peer, bytes) => endpoint.receive(&peer, bytes),
        LinkEvent::Unsent(peer, bytes) => endpoint.transmit_failed(
            &peer,
            bytes,
            TransportFailure {
                retryable: true,
                message: Some("the link closed before the envelope was written".to_string()),
            },
        ),
        LinkEvent::Lost(peer, _) => endpoint.peer_not_ready(&peer),
    }
}

/// Whether `name` is a path segment as it stands: RFC 3986's unreserved characters, so no
/// encoding can make two names one. It is also a key expression chunk as it stands, which is why
/// the Zenoh binding asks the same question of a machine name.
pub fn is_path_segment(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~'))
}
