// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A server machine on the WebSocket binding (SCE_MESH.md §mesh-18), for a
//! peer written in another language to talk to.
//!
//! It listens on a loopback port it prints as `LISTENING <port>`, accepts one
//! client named `client`, and answers every `ping` it is delivered with a
//! `pong` carrying the same `_event.data` and the send id `reply-<n>`. When
//! the link ends it prints `DONE <pings answered>` and exits — 0 when it
//! answered at least one, 1 otherwise.
//!
//! The Kotlin module's `CrossLanguageLoopbackTest` runs it: the Rust core
//! decodes what the Kotlin core encoded and the other way round, over one real
//! socket, which is the claim two cores held to the same bytes have to make
//! good on.

use std::io::Write;

use sce_rust_mesh::endpoint::{Endpoint, Environment, MESH_PROCESSOR_TYPE};
use sce_rust_mesh::inbound::Delivery;
use sce_rust_mesh::router::{PeerConfig, Router};
use sce_rust_mesh::wss::{accept, deliver, LinkEvent, WssTransport, DEFAULT_KEEPALIVE};
use sce_rust_runtime::HostSendRequest;
use tokio::net::TcpListener;
use tokio::sync::mpsc;

/// A counting clock and counting ids: this peer's ids need only differ.
#[derive(Default)]
struct Counting {
    now: i64,
    ids: u8,
}

impl Environment for Counting {
    fn now_ms(&mut self) -> i64 {
        self.now += 1;
        self.now
    }
    fn envelope_id(&mut self) -> [u8; 16] {
        self.ids = self.ids.wrapping_add(1);
        [0xA0 ^ self.ids; 16]
    }
    fn jitter_draw(&mut self) -> i64 {
        0
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("a loopback port");
    println!(
        "LISTENING {}",
        listener.local_addr().expect("a bound address").port()
    );
    std::io::stdout().flush().expect("stdout is open");

    let mut router = Router::new("server", 64, 1000).expect("a valid configuration");
    router.add_peer(
        "client",
        PeerConfig {
            transport: "wss",
            max_pending: 64,
            max_age_ms: 0,
            retry: None,
            stamp_sequence: false,
            delivery: Delivery {
                dedup: true,
                ordered: false,
            },
        },
    );
    let transport = WssTransport::new();
    let mut endpoint = Endpoint::new(router, transport.clone(), Counting::default());

    let (tx, mut events) = mpsc::unbounded_channel();
    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("one client");
        let _ = accept(
            stream,
            |name| name == "client",
            transport,
            tx,
            DEFAULT_KEEPALIVE,
        )
        .await;
    });

    let mut answered = 0u32;
    while let Some(event) = events.recv().await {
        let lost = matches!(event, LinkEvent::Lost(..));
        deliver(&mut endpoint, event);
        for delivered in endpoint.take_events() {
            if delivered.name != "ping" {
                continue;
            }
            answered += 1;
            endpoint.send(&HostSendRequest {
                processor_type: MESH_PROCESSOR_TYPE.to_string(),
                event_name: "pong".to_string(),
                target: "#client".to_string(),
                send_id: format!("reply-{answered}"),
                event_data: delivered.metadata.data.clone(),
                ..HostSendRequest::default()
            });
        }
        for error in endpoint.take_host_errors() {
            eprintln!("host error: {error:?}");
        }
        if lost {
            break;
        }
    }
    println!("DONE {answered}");
    std::process::exit(if answered > 0 { 0 } else { 1 });
}
