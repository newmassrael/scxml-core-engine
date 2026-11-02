// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A device logs in at a relay over the WebSocket binding (ADR 0080 section 1). What is asked: that a
// relay that offers login takes an upgrade with no token, a device logs in on it and is issued its
// token, and the link given the token gives the anonymous connection up and dials with it; that an
// upgrade with a token the relay issued that expired is taken as anonymous, a request of a session on
// it is refused as the token's, and the device logs in on it; that a connection with no token is
// closed once it made as many requests as it may; and that the source a connection with no token is
// held to is its address, or the address a proxy the server trusts says it forwarded, and never one
// a client says of itself.

use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, Mutex};

use sce_rust_relay::serve::{Bounds, Server, WssLink};
use sce_rust_relay::wss::{endpoint, upgrade_request};
use sce_rust_relay::{
    drive, Authority, Code, Doorway, Limits, Link, LoginMachine, LoginPolicy, MemoryStore, Notice,
    Relay, Request, Response, Signal, Transport, TransportError,
};
use sce_rust_trust::membership::{Body, Entry, Log, Profile};
use sce_rust_trust::{Identity, Trust};
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::{http, Error as WsError};

const LIST: &str = "home";
const AUDIENCE: &str = "relay-over-a-socket";
const NOW: u64 = 5_000_000;

fn person(node: u64) -> Identity {
    Identity::from_seed([node as u8; 32])
}

fn admin() -> Identity {
    Identity::from_seed([9; 32])
}

/// The log: alice (1) and bob (2) write.
fn entries() -> Vec<Entry> {
    let trust = Trust::open(LIST);
    let genesis = Log::genesis_draft(
        LIST,
        Body::Genesis {
            administrators: vec![admin().public()],
            quorum: 1,
            profile: Profile::Untrusted,
        },
    )
    .sign(&admin())
    .finish();
    trust.append(&genesis).unwrap();
    for member in [1u64, 2] {
        let entry = trust
            .draft(Body::Grant {
                node: member,
                role: 2,
                key: person(member).public(),
                final_seq: None,
            })
            .sign(&admin())
            .finish();
        trust.append(&entry).unwrap();
    }
    trust.entries()
}

/// The device of `node`, holding the log and its person's key.
fn device(node: u64) -> Trust {
    let trust = Trust::open(LIST);
    for entry in entries() {
        trust.append(&entry).unwrap();
    }
    trust.set_identity(node, person(node));
    trust
}

struct Running {
    runtime: tokio::runtime::Runtime,
    authority: Authority,
    base: String,
}

/// A relay that offers login under `policy`, holding the log, served by a server bounded by `bounds`
/// and trusting the proxies at `proxies`.
fn start(policy: LoginPolicy, bounds: Bounds, proxies: Vec<IpAddr>) -> Running {
    let authority = Authority::new([4; 32], AUDIENCE);
    let mut relay = Relay::open(MemoryStore::new(), authority.clone(), Limits::default())
        .unwrap()
        .with_login(Doorway::with_key(policy, [5; 32]));
    let operator = authority.issue(LIST, 1, u64::MAX);
    for entry in entries() {
        let answer = relay.handle(
            &operator,
            &Request::Append {
                entry: entry.to_json(),
            }
            .to_json(),
            NOW,
        );
        assert!(answer.contains("\"appended\""), "{answer}");
    }
    let clock: Arc<dyn Fn() -> u64 + Send + Sync> = Arc::new(|| NOW);
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let listener = runtime.block_on(TcpListener::bind("127.0.0.1:0")).unwrap();
    let address: SocketAddr = listener.local_addr().unwrap();
    let server = Server::trusting(Arc::new(Mutex::new(relay)), clock, bounds, proxies);
    runtime.spawn(async move { server.listen(listener, None).await });
    Running {
        runtime,
        authority,
        base: format!("ws://{address}"),
    }
}

/// The token the device of `node` earns by logging in over `link`.
fn log_in(link: &mut WssLink, node: u64) -> String {
    let mut login = LoginMachine::start(device(node), AUDIENCE);
    drive(&mut login, link, None, &Signal::new());
    login
        .outcome()
        .cloned()
        .expect("the login is over")
        .expect("the login came to a token")
        .token
}

#[test]
fn a_device_logs_in_on_a_connection_with_no_token_and_dials_again_with_its_token() {
    let running = start(LoginPolicy::default(), Bounds::default(), Vec::new());
    let _keep = &running.runtime;
    let mut link = WssLink::new(&running.base, "").unwrap();
    // With no token, the connection answers nothing of a session's.
    let refused = link.call(&Request::Heads).unwrap();
    assert!(
        matches!(
            refused,
            Response::Refused {
                code: Code::Unauthorized,
                ..
            }
        ),
        "{refused:?}"
    );
    let token = log_in(&mut link, 2);
    assert_eq!(
        running
            .authority
            .verify(&token, NOW)
            .map(|session| session.node),
        Ok(2)
    );
    assert!(link.set_token(&token));
    let heads = link.call(&Request::Heads).unwrap();
    assert!(
        matches!(heads, Response::Heads { log_len: 3, .. }),
        "{heads:?}"
    );
    assert_eq!(
        link.take_notices(),
        vec![
            Notice::Connected,
            Notice::Lost("the token changed".to_string()),
            Notice::Connected
        ],
        "the anonymous connection was given up for one with the token"
    );
}

#[test]
fn a_token_that_expired_opens_a_connection_a_device_logs_in_on() {
    let running = start(LoginPolicy::default(), Bounds::default(), Vec::new());
    let _keep = &running.runtime;
    let expired = running.authority.issue(LIST, 2, NOW - 1);
    let mut link = WssLink::new(&running.base, &expired).unwrap();
    let refused = link.call(&Request::Heads).unwrap();
    match refused {
        Response::Refused {
            code: Code::Unauthorized,
            reason,
        } => assert!(reason.contains("expired"), "{reason}"),
        other => panic!("{other:?}"),
    }
    let token = log_in(&mut link, 2);
    assert!(running.authority.verify(&token, NOW).is_ok());
    // A token the relay never issued is refused at the upgrade still.
    let mut stranger = WssLink::new(&running.base, "nonsense").unwrap();
    assert!(matches!(
        stranger.exchange(&Request::Heads.to_json(), None, &Signal::new()),
        Err(TransportError::Unauthorized(_))
    ));
}

#[test]
fn a_connection_with_no_token_makes_so_many_requests_and_is_closed() {
    let bounds = Bounds {
        anonymous_requests: 2,
        ..Bounds::default()
    };
    let running = start(LoginPolicy::default(), bounds, Vec::new());
    let _keep = &running.runtime;
    let mut link = WssLink::new(&running.base, "").unwrap();
    let none = Signal::new();
    let heads = Request::Heads.to_json();
    for _ in 0..2 {
        assert!(link.exchange(&heads, None, &none).is_ok());
    }
    assert!(
        matches!(
            link.exchange(&heads, None, &none),
            Err(TransportError::Link(_))
        ),
        "the relay closed it"
    );
}

/// The status a dial with no token answers with, when it says it was forwarded for `forwarded`.
fn dial(running: &Running, forwarded: &str) -> u16 {
    let mut request = upgrade_request(&endpoint(&running.base).unwrap(), "").unwrap();
    request.headers_mut().insert(
        "x-forwarded-for",
        http::HeaderValue::from_str(forwarded).unwrap(),
    );
    match running
        .runtime
        .block_on(tokio_tungstenite::connect_async(request))
    {
        Ok(_) => 101,
        Err(WsError::Http(response)) => response.status().as_u16(),
        Err(other) => panic!("{other}"),
    }
}

#[test]
fn a_source_is_the_address_a_trusted_proxy_forwarded_for_and_no_other() {
    let one_a_minute = LoginPolicy {
        connections_per_minute: 1,
        ..LoginPolicy::default()
    };
    // Behind a proxy the server trusts (here, the test's own address): each forwarded address is a
    // source of its own.
    let behind = start(
        one_a_minute,
        Bounds::default(),
        vec!["127.0.0.1".parse().unwrap()],
    );
    assert_eq!(dial(&behind, "198.51.100.1"), 101);
    assert_eq!(dial(&behind, "198.51.100.2"), 101);
    assert_eq!(dial(&behind, "198.51.100.1"), 429, "one a minute");
    // Trusting no proxy: what a client says it was forwarded for is not believed.
    let direct = start(one_a_minute, Bounds::default(), Vec::new());
    assert_eq!(dial(&direct, "198.51.100.1"), 101);
    assert_eq!(dial(&direct, "198.51.100.2"), 429, "the same address");
}
