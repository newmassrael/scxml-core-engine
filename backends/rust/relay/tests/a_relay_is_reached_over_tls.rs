// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
//
// The relay over `wss://`. The relay's server terminates TLS with a certificate made for the test,
// and a link that believes the authority behind it exchanges texts with the relay, its token judged
// at the upgrade as over `ws://`; a link that believes the public authorities refuses that
// certificate and says so as the failure of its link, never as a panic of the library; and an
// authority that is not a certificate is refused when the link is made. Both ends run on the one
// provider the bindings name (`ring`).

use std::sync::{Arc, Mutex};

use sce_rust_relay::serve::{serve_tls, WssLink};
use sce_rust_relay::tls::{self, Roots};
use sce_rust_relay::{
    Authority, Limits, Link, MemoryStore, Notice, Relay, Request, Response, Signal, Transport,
    TransportError,
};
use tokio::net::TcpListener;

const LIST: &str = "home";

struct Running {
    runtime: tokio::runtime::Runtime,
    authority: Authority,
    base: String,
    /// The certificate the relay presents, which is its own authority.
    certificate: Vec<u8>,
}

/// A relay behind TLS on a port of this host, presenting a certificate for `127.0.0.1` that no
/// public authority issued.
fn start() -> Running {
    let issued = rcgen::generate_simple_self_signed(vec!["127.0.0.1".to_string()]).unwrap();
    let certificate = issued.cert.der().to_vec();
    let config =
        tls::server_config(vec![certificate.clone()], issued.key_pair.serialize_der()).unwrap();

    let authority = Authority::new([4; 32], "relay-over-tls");
    let relay = Arc::new(Mutex::new(
        Relay::open(MemoryStore::new(), authority.clone(), Limits::default()).unwrap(),
    ));
    let clock: Arc<dyn Fn() -> u64 + Send + Sync> = Arc::new(|| 0);
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let listener = runtime.block_on(TcpListener::bind("127.0.0.1:0")).unwrap();
    let address = listener.local_addr().unwrap();
    runtime.spawn(serve_tls(listener, config, relay, clock, 64 * 1024));
    Running {
        runtime,
        authority,
        base: format!("wss://{address}"),
        certificate,
    }
}

#[test]
fn a_link_that_believes_the_relays_authority_exchanges_over_tls() {
    let running = start();
    let _keep = &running.runtime;
    let roots = Roots::Only(vec![running.certificate.clone()]);
    let none = Signal::new();

    let token = running.authority.issue(LIST, 1, u64::MAX);
    let mut link = WssLink::believing(&running.base, &token, &roots).unwrap();
    let answer = link
        .exchange(&Request::Entries { from: 0 }.to_json(), None, &none)
        .unwrap();
    assert_eq!(
        Response::from_json(&answer),
        Ok(Response::Entries {
            entries: Vec::new(),
            len: 0
        }),
        "the response text, as the relay wrote it, through TLS"
    );
    assert_eq!(link.take_notices(), vec![Notice::Connected]);
    assert_eq!(
        link.call(&Request::Entries { from: 0 }),
        Ok(Response::Entries {
            entries: Vec::new(),
            len: 0
        }),
        "the connection was kept"
    );
    assert!(link.take_notices().is_empty());

    // The upgrade inside TLS judges the token as the upgrade over `ws://` does.
    let mut stranger = WssLink::believing(&running.base, "nonsense", &roots).unwrap();
    assert!(
        matches!(
            stranger.exchange(&Request::Heads.to_json(), None, &none),
            Err(TransportError::Unauthorized(_))
        ),
        "the token was refused at the upgrade"
    );
}

#[test]
fn a_link_that_believes_the_public_authorities_refuses_a_certificate_they_did_not_issue() {
    let running = start();
    let _keep = &running.runtime;
    let token = running.authority.issue(LIST, 1, u64::MAX);

    let mut link = WssLink::new(&running.base, &token).unwrap();
    let answer = link.exchange(&Request::Heads.to_json(), None, &Signal::new());
    let Err(TransportError::Link(why)) = answer else {
        panic!("expected the link to fail its TLS handshake, got {answer:?}");
    };
    assert!(
        why.contains("invalid peer certificate: UnknownIssuer"),
        "{why}"
    );
    assert!(link.take_notices().is_empty(), "it never connected");
}

#[test]
fn an_authority_that_is_not_a_certificate_is_refused_when_the_link_is_made() {
    let made = WssLink::believing(
        "wss://127.0.0.1:1",
        "token",
        &Roots::Only(vec![b"not a certificate".to_vec()]),
    );
    let Err(error) = made else {
        panic!("expected the link to be refused");
    };
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
}
