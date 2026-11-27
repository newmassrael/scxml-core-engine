// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A server of the HTTPS binding (SCE_MESH.md section 22) that answers every envelope a client
//! sends with the same envelope, for the browser node's kit (`backends/rust/browser-node/kit`) to
//! hold the browser shell to the binding's own server (docs/adr/0117, D2).
//!
//! It serves any name that is a path segment under the `key` authenticator, pinning each to the
//! key of its first exchange answered `200` (trust on first use, in memory), holds each request to
//! the freshness step of `FRESH_MS` (D4), and signs every answer under `sign:` with a key it draws
//! when it starts, as `kit`. It listens on a loopback port without TLS and prints one line of
//! JSON before it serves: `{"port", "key_id", "public_key", "fresh_ms"}`, the public key in
//! base64url without padding, for a client's `verify:`. It serves until it is stopped.

use std::io::Write;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use sce_rust_crypto::ed25519::KeyPair;
use sce_rust_crypto::random::SystemRandom;
use sce_rust_mesh::endpoint::Transport;
use sce_rust_mesh::https::server::{
    Authenticator, HttpsServer, KeyPins, MailboxTransport, MemoryCredentials, ResponseSigner,
    ServerSettings,
};
use sce_rust_mesh::https::{is_path_segment, LinkEvent};
use tokio::net::TcpListener;

/// The freshness step every request is held to: short, so that a value a node signed goes stale
/// within a case of the kit.
const FRESH_MS: u64 = 2_000;

/// The id a client's trust set knows the server's key by.
const KEY_ID: &str = "kit";

fn main() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("a runtime");
    // The mailboxes the host answers into: the server's own transport, set once it is built.
    let mailboxes: Arc<OnceLock<MailboxTransport>> = Arc::new(OnceLock::new());
    let host = {
        let mailboxes = Arc::clone(&mailboxes);
        move |events: Vec<LinkEvent>| {
            let Some(transport) = mailboxes.get() else {
                return;
            };
            for event in events {
                if let LinkEvent::Received(client, envelope) = event {
                    // A full mailbox refuses the echo, which the kit sees as an envelope that
                    // never comes back.
                    let _ = transport.clone().transmit(&client, &envelope);
                }
            }
        }
    };
    let key = KeyPair::generate(&mut SystemRandom).expect("the random source");
    let public_key = key.public_key();
    let server = HttpsServer::new(
        ServerSettings {
            max_hold: Duration::from_secs(2),
            signer: Some(ResponseSigner::new(KEY_ID, key)),
            fresh: Some(Duration::from_millis(FRESH_MS)),
            ..ServerSettings::default()
        },
        Authenticator::Key(KeyPins::first_use(Arc::new(MemoryCredentials::new()))),
        is_path_segment,
        host,
    )
    .expect("a server");
    let _ = mailboxes.set(server.transport());
    let listener = runtime
        .block_on(TcpListener::bind("127.0.0.1:0"))
        .expect("a loopback port");
    let port = listener.local_addr().expect("its address").port();
    println!(
        "{{\"port\":{port},\"key_id\":\"{KEY_ID}\",\"public_key\":\"{}\",\"fresh_ms\":{FRESH_MS}}}",
        URL_SAFE_NO_PAD.encode(public_key)
    );
    std::io::stdout().flush().expect("stdout is open");
    let stopped = runtime.block_on(server.serve(listener, None));
    eprintln!("the server stopped: {stopped}");
}
