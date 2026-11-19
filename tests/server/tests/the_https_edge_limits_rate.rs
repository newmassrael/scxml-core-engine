// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The HTTPS binding's edge limits rate (docs/adr/0109, decision D2): an address that guesses a
//! client's secret stops reaching the authenticator once its failures are past their rate, an
//! address that pins name after name is refused past its rate of new names, and through both an
//! established client is served as before.
//!
//! The server is the binding's server (`HttpsServer`) with the installation authenticator, behind
//! a proxy it trusts: every connection comes from the loopback address, which the server's
//! `trusted_proxies` names, and each request says the client's address in `Forwarded` (RFC 7239),
//! so the guesser, the flooder and the established client are three addresses to the edge, as
//! they are behind a deployment's load balancer. The client is the reference adapter
//! (`HyperTransport`), carrying each exchange as the binding's client would.

use std::net::SocketAddr;
use std::time::Duration;

use sce_rust_mesh::https::adapter::{AdapterSettings, HyperTransport, ProxyPolicy};
use sce_rust_mesh::https::rate::Cidr;
use sce_rust_mesh::https::server::{Authenticator, HttpsServer, MemoryPins, ServerSettings};
use sce_rust_mesh::https::{
    draw_install_secret, Credential, HttpsRequest, HttpsResponse, HttpsTransport, ACK_HEADER,
    CONTENT_TYPE, HOLD_HEADER,
};
use sce_rust_mesh::link::LinkEvent;
use sce_rust_mesh::peers::{EdgeRate, EdgeRates};
use tokio::net::TcpListener;
use tokio::runtime::Runtime;

/// The established client's address, as the proxy names it.
const ESTABLISHED: &str = "203.0.113.9";
/// The address that guesses another client's secret.
const GUESSER: &str = "198.51.100.7";
/// The address that pins name after name.
const FLOODER: &str = "198.51.100.8";
/// The address the client whose secret is guessed pinned its name from.
const VICTIM_HOME: &str = "203.0.113.10";

/// How many failed authentications, and how many new names, an address or a name may spend at
/// once before its rate holds it back.
const BURST: u32 = 3;

/// The server on a loopback port and the adapter that reaches it.
struct Edge {
    // Held so that the server runs for as long as the scenario does.
    _runtime: Runtime,
    address: SocketAddr,
    port: HyperTransport,
}

impl Edge {
    /// A server whose edge limits failed authentications to `BURST` a minute and new names to
    /// `BURST` in three minutes, by name and by address, and trusts the loopback address as its
    /// proxy; the other limiters are off, so what is seen is these two.
    fn start() -> Edge {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("the scenario's runtime");
        let server = HttpsServer::new(
            ServerSettings {
                rates: EdgeRates {
                    failed_auth: Some(EdgeRate {
                        interval_ms: 60_000,
                        burst: BURST,
                    }),
                    new_names: Some(EdgeRate {
                        interval_ms: 180_000,
                        burst: BURST,
                    }),
                    ..EdgeRates::NONE
                },
                trusted_proxies: vec![Cidr::parse("127.0.0.1/32").expect("a block")],
                ..ServerSettings::default()
            },
            Authenticator::Install(std::sync::Arc::new(MemoryPins::default())),
            |_: &str| true,
            |_: Vec<LinkEvent>| {},
        )
        .expect("the server");
        let listener = runtime
            .block_on(TcpListener::bind("127.0.0.1:0"))
            .expect("a loopback port");
        let address = listener.local_addr().expect("its address");
        runtime.spawn(async move {
            let _ = server.serve(listener, None).await;
        });
        let port = HyperTransport::new(AdapterSettings {
            proxy: ProxyPolicy::Direct,
            ..AdapterSettings::default()
        })
        .expect("the adapter");
        Edge {
            _runtime: runtime,
            address,
            port,
        }
    }

    /// An exchange of `name`, proving it with `secret`, from `from` as the proxy names it.
    fn exchange(&self, name: &str, secret: [u8; 32], from: &str) -> HttpsResponse {
        let authorization = Credential::Install(secret)
            .authorization()
            .expect("a secret is sent in a header");
        self.port
            .exchange(HttpsRequest {
                url: format!("http://{}/sce-mesh/1/{name}", self.address),
                headers: vec![
                    ("Content-Type".to_string(), CONTENT_TYPE.to_string()),
                    (ACK_HEADER.to_string(), "0".to_string()),
                    (HOLD_HEADER.to_string(), "0".to_string()),
                    ("Authorization".to_string(), authorization),
                    ("Forwarded".to_string(), format!("for={from};proto=https")),
                ],
                body: Vec::new(),
                timeout: Duration::from_secs(10),
            })
            .expect("a response")
    }
}

/// The wait a `429` asks for, in seconds.
fn wait_of(response: &HttpsResponse) -> u64 {
    assert_eq!(response.status, 429, "{response:?}");
    response
        .header("Retry-After")
        .and_then(|value| value.parse().ok())
        .expect("a 429 names its wait in seconds")
}

fn secret() -> [u8; 32] {
    draw_install_secret().expect("the random source")
}

/// A storm of wrong secrets for a pinned name, from one address, is answered `401` until the
/// failures are past their rate and `429` after, asking for the wait of the failure limiter; the
/// client whose name is guessed and the established client, from their own addresses, are told
/// apart from it: the established one is served throughout. Red if the edge reads the address
/// of the connection rather than the one its trusted proxy names (every client is then one
/// address, and the established client is refused with the guesser), or if the authenticator is
/// asked before the failure limiters.
#[test]
fn a_storm_of_wrong_secrets_is_held_back_while_an_established_client_is_served() {
    let edge = Edge::start();
    let (phone, victim) = (secret(), secret());
    assert_eq!(edge.exchange("phone", phone, ESTABLISHED).status, 200);
    assert_eq!(edge.exchange("victim", victim, VICTIM_HOME).status, 200);

    for _ in 0..BURST {
        assert_eq!(edge.exchange("victim", secret(), GUESSER).status, 401);
    }
    for _ in 0..50 {
        let wait = wait_of(&edge.exchange("victim", secret(), GUESSER));
        assert!((1..=60).contains(&wait), "{wait}");
        assert_eq!(
            edge.exchange("phone", phone, ESTABLISHED).status,
            200,
            "the established client is served through the storm"
        );
    }
    // The guesser's address is past its rate for any name it tries.
    assert_eq!(edge.exchange("other", secret(), GUESSER).status, 429);
}

/// An address that pins name after name is refused past its rate of new names, asking for the
/// wait until one regenerates, while a name it pinned within the rate and the established client
/// elsewhere are served. Red if a first pin spends nothing of its address's rate: every name of
/// the flood is then pinned.
#[test]
fn a_flood_of_new_names_from_one_address_is_held_back() {
    let edge = Edge::start();
    let phone = secret();
    assert_eq!(edge.exchange("phone", phone, ESTABLISHED).status, 200);

    let mut pinned = Vec::new();
    let mut refused = 0;
    for n in 0..20 {
        let name = format!("device{n}");
        let held = secret();
        let answered = edge.exchange(&name, held, FLOODER);
        match answered.status {
            200 => pinned.push((name, held)),
            _ => {
                let wait = wait_of(&answered);
                assert!((1..=180).contains(&wait), "{wait}");
                refused += 1;
            }
        }
        assert_eq!(edge.exchange("phone", phone, ESTABLISHED).status, 200);
    }
    assert_eq!(pinned.len(), BURST as usize, "the burst, and no more");
    assert_eq!(refused, 20 - BURST as usize);
    for (name, held) in &pinned {
        assert_eq!(
            edge.exchange(name, *held, FLOODER).status,
            200,
            "a name pinned within the rate is not new again"
        );
    }
}
