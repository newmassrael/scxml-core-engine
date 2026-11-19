// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The network transport of section 5 (docs/SCE_HOST_FEATURES.md) behind its
// destination policy (docs/adr/0109, D4), held against a server on a loopback
// port of this process. The names are answered by a resolver the test scripts,
// so nothing reaches another machine: a name that resolves to loopback is
// refused before a byte is sent, one the allow-list lets reach loopback is
// served, a name is resolved once per exchange and the connection is made to
// the address checked, a redirect is answered as it came, and the bounds of
// time and size hold.

use std::io::{self, Read, Write};
use std::net::{IpAddr, TcpListener};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use sce_rust_host_features::destination::{
    AllowedDestination, DestinationPolicy, IpNetwork, Resolve, Scheme,
};
use sce_rust_host_features::{
    HttpRequest, Invocation, Outcome, Params, ReferenceHost, ScriptedClock, Transport,
    TransportFailure,
};
use sce_rust_host_http::{NetworkSettings, NetworkTransport, Timeouts};

/// A server on a loopback port, and the head of every request it read.
struct Server {
    port: u16,
    heads: Arc<Mutex<Vec<String>>>,
}

impl Server {
    /// Answers each request by its path: `/ok` is served, `/moved` redirects to
    /// a private address, `/big` sends a body of 65 bytes, `/wide` a head of
    /// about 10000 bytes, and `/slow` never answers.
    fn start() -> Server {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a loopback port");
        let port = listener.local_addr().expect("its address").port();
        let heads = Arc::new(Mutex::new(Vec::new()));
        let seen = Arc::clone(&heads);
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { return };
                let seen = Arc::clone(&seen);
                std::thread::spawn(move || {
                    let mut head = Vec::new();
                    let mut byte = [0u8; 1];
                    while !head.ends_with(b"\r\n\r\n") && stream.read(&mut byte).unwrap_or(0) == 1 {
                        head.push(byte[0]);
                    }
                    let head = String::from_utf8_lossy(&head).into_owned();
                    let path = head.split(' ').nth(1).unwrap_or_default().to_string();
                    seen.lock().expect("not mid-update").push(head);
                    let answer: Vec<u8> = match path.as_str() {
                        "/ok" => b"HTTP/1.1 200 OK\r\nX-Reply: yes\r\nContent-Length: 6\r\n\
                                   Connection: close\r\n\r\nserved"
                            .to_vec(),
                        "/moved" => b"HTTP/1.1 302 Found\r\nLocation: http://10.0.0.1/inside\r\n\
                                      Content-Length: 0\r\nConnection: close\r\n\r\n"
                            .to_vec(),
                        "/big" => {
                            let mut big = b"HTTP/1.1 200 OK\r\nContent-Length: 65\r\n\
                                            Connection: close\r\n\r\n"
                                .to_vec();
                            big.extend_from_slice(&[b'x'; 65]);
                            big
                        }
                        "/wide" => {
                            let mut wide = b"HTTP/1.1 200 OK\r\nX-Wide: ".to_vec();
                            wide.extend_from_slice(&[b'w'; 10000]);
                            wide.extend_from_slice(
                                b"\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                            );
                            wide
                        }
                        _ => {
                            // `/slow`: holds the connection and answers nothing.
                            std::thread::sleep(Duration::from_secs(5));
                            return;
                        }
                    };
                    let _ = stream.write_all(&answer);
                });
            }
        });
        Server { port, heads }
    }

    fn heads(&self) -> Vec<String> {
        self.heads.lock().expect("not mid-update").clone()
    }
}

/// A resolver that answers each name with the next of its answers (the last
/// one again once they run out), and counts the names it was asked.
struct Scripted {
    answers: Vec<(&'static str, Vec<Vec<IpAddr>>)>,
    asked: Mutex<Vec<String>>,
}

impl Resolve for Scripted {
    fn resolve(&self, host: &str) -> io::Result<Vec<IpAddr>> {
        let mut asked = self.asked.lock().expect("not mid-update");
        let before = asked.iter().filter(|name| *name == host).count();
        asked.push(host.to_string());
        let (_, answers) = self
            .answers
            .iter()
            .find(|(name, _)| *name == host)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no such name"))?;
        Ok(answers[before.min(answers.len() - 1)].clone())
    }
}

fn scripted(answers: &[(&'static str, &[&[&str]])]) -> Arc<Scripted> {
    Arc::new(Scripted {
        answers: answers
            .iter()
            .map(|(name, turns)| {
                let turns: Vec<Vec<IpAddr>> = turns
                    .iter()
                    .map(|addresses| {
                        addresses
                            .iter()
                            .map(|text| text.parse::<IpAddr>().expect("an address literal"))
                            .collect()
                    })
                    .collect();
                (*name, turns)
            })
            .collect(),
        asked: Mutex::new(Vec::new()),
    })
}

fn asked(resolver: &Scripted) -> Vec<String> {
    resolver.asked.lock().expect("not mid-update").clone()
}

fn network(address: &str, prefix: u8) -> IpNetwork {
    IpNetwork::new(address.parse().expect("an address literal"), prefix).expect("a prefix")
}

fn get(url: String) -> HttpRequest {
    HttpRequest {
        method: "GET".to_string(),
        url,
        headers: Vec::new(),
        body: String::new(),
    }
}

fn transport(policy: DestinationPolicy, settings: NetworkSettings) -> NetworkTransport {
    NetworkTransport::new(policy, settings).expect("a transport")
}

#[test]
fn a_name_that_resolves_to_loopback_is_refused_and_an_allow_listed_one_is_served() {
    let server = Server::start();
    let port = server.port;
    let resolver = scripted(&[
        ("loopback.test", &[&["127.0.0.1"]]),
        ("served.test", &[&["127.0.0.1"]]),
    ]);
    let policy = DestinationPolicy::allow_list(vec![
        AllowedDestination::new(Scheme::Http, "loopback.test", port),
        AllowedDestination::new(Scheme::Http, "served.test", port)
            .reaching(network("127.0.0.0", 8)),
    ])
    .resolving_with(resolver);
    let mut transport = transport(policy, NetworkSettings::default());

    let refused = transport.exchange(&get(format!("http://loopback.test:{port}/ok")));
    assert_eq!(refused, Err(TransportFailure::DestinationRefused));
    assert!(
        server.heads().is_empty(),
        "nothing was sent: {:?}",
        server.heads()
    );

    let served = transport
        .exchange(&get(format!("http://served.test:{port}/ok")))
        .expect("an answer")
        .expect("not held");
    assert_eq!(served.status, 200);
    assert_eq!(served.body, b"served");
    assert!(served
        .headers
        .contains(&("x-reply".to_string(), "yes".to_string())));
    let heads = server.heads();
    assert_eq!(heads.len(), 1);
    assert!(heads[0].starts_with("GET /ok HTTP/1.1\r\n"), "{}", heads[0]);
    assert!(
        heads[0]
            .to_ascii_lowercase()
            .contains(&format!("host: served.test:{port}\r\n")),
        "{}",
        heads[0]
    );
}

#[test]
fn a_target_off_the_allow_list_is_refused_before_its_name_is_resolved() {
    let server = Server::start();
    let port = server.port;
    let resolver = scripted(&[("other.test", &[&["127.0.0.1"]])]);
    let policy = DestinationPolicy::allow_list(vec![AllowedDestination::new(
        Scheme::Http,
        "served.test",
        port,
    )
    .reaching(network("127.0.0.0", 8))])
    .resolving_with(resolver.clone());
    let mut transport = transport(policy, NetworkSettings::default());
    assert_eq!(
        transport.exchange(&get(format!("http://other.test:{port}/ok"))),
        Err(TransportFailure::DestinationRefused)
    );
    assert_eq!(
        transport.exchange(&get(format!("https://served.test:{port}/ok"))),
        Err(TransportFailure::DestinationRefused)
    );
    assert!(asked(&resolver).is_empty());
    assert!(server.heads().is_empty());
}

/// A name that answers the address checked and then another is resolved once
/// for the exchange, and the connection is made to the first: were it resolved
/// again to connect, it would reach `127.0.0.2`, where nothing listens.
#[test]
fn the_connection_is_made_to_the_address_checked_and_each_exchange_resolves_anew() {
    let server = Server::start();
    let port = server.port;
    let resolver = scripted(&[("rebinding.test", &[&["127.0.0.1"], &["127.0.0.2"]])]);
    let policy = DestinationPolicy::allow_list(vec![AllowedDestination::new(
        Scheme::Http,
        "rebinding.test",
        port,
    )
    .reaching(network("127.0.0.1", 32))])
    .resolving_with(resolver.clone());
    let mut transport = transport(policy, NetworkSettings::default());

    let served = transport
        .exchange(&get(format!("http://rebinding.test:{port}/ok")))
        .expect("an answer")
        .expect("not held");
    assert_eq!(served.status, 200);
    assert_eq!(asked(&resolver), ["rebinding.test"]);

    // The next exchange asks again, and `127.0.0.2` is no address it may reach.
    assert_eq!(
        transport.exchange(&get(format!("http://rebinding.test:{port}/ok"))),
        Err(TransportFailure::DestinationRefused)
    );
    assert_eq!(asked(&resolver), ["rebinding.test", "rebinding.test"]);
    assert_eq!(server.heads().len(), 1);
}

#[test]
fn a_redirect_to_a_private_address_is_answered_as_the_3xx_it_is() {
    let server = Server::start();
    let port = server.port;
    let policy = DestinationPolicy::allow_list(vec![AllowedDestination::new(
        Scheme::Http,
        "127.0.0.1",
        port,
    )
    .reaching(network("127.0.0.1", 32))]);
    let mut transport = transport(policy, NetworkSettings::default());
    let moved = transport
        .exchange(&get(format!("http://127.0.0.1:{port}/moved")))
        .expect("an answer")
        .expect("not held");
    assert_eq!(moved.status, 302);
    assert!(moved
        .headers
        .contains(&("location".to_string(), "http://10.0.0.1/inside".to_string())));
    assert_eq!(server.heads().len(), 1);
}

#[test]
fn a_body_or_a_head_past_its_bound_is_too_large() {
    let server = Server::start();
    let port = server.port;
    let policy = DestinationPolicy::allow_list(vec![AllowedDestination::new(
        Scheme::Http,
        "127.0.0.1",
        port,
    )
    .reaching(network("127.0.0.1", 32))]);
    let mut transport = transport(
        policy,
        NetworkSettings {
            max_response_bytes: 64,
            max_header_bytes: 8192,
            ..NetworkSettings::default()
        },
    );
    assert_eq!(
        transport.exchange(&get(format!("http://127.0.0.1:{port}/big"))),
        Err(TransportFailure::ResponseTooLarge)
    );
    assert_eq!(
        transport.exchange(&get(format!("http://127.0.0.1:{port}/wide"))),
        Err(TransportFailure::ResponseTooLarge)
    );
    let fits = transport
        .exchange(&get(format!("http://127.0.0.1:{port}/ok")))
        .expect("an answer")
        .expect("not held");
    assert_eq!(fits.body, b"served");
}

#[test]
fn a_response_past_the_header_timeout_fails_network() {
    let server = Server::start();
    let port = server.port;
    let policy = DestinationPolicy::allow_list(vec![AllowedDestination::new(
        Scheme::Http,
        "127.0.0.1",
        port,
    )
    .reaching(network("127.0.0.1", 32))]);
    let mut transport = transport(
        policy,
        NetworkSettings {
            timeouts: Timeouts {
                header: Duration::from_millis(200),
                ..Timeouts::default()
            },
            ..NetworkSettings::default()
        },
    );
    let began = Instant::now();
    assert_eq!(
        transport.exchange(&get(format!("http://127.0.0.1:{port}/slow"))),
        Err(TransportFailure::Network)
    );
    assert!(began.elapsed() < Duration::from_secs(3));
}

#[test]
fn a_request_that_names_a_field_the_transport_writes_is_not_sent() {
    let server = Server::start();
    let port = server.port;
    let policy = DestinationPolicy::allow_list(vec![AllowedDestination::new(
        Scheme::Http,
        "127.0.0.1",
        port,
    )
    .reaching(network("127.0.0.1", 32))]);
    let mut transport = transport(policy, NetworkSettings::default());
    for field in ["Host", "Content-Length", "Transfer-Encoding"] {
        let mut request = get(format!("http://127.0.0.1:{port}/ok"));
        request.headers = vec![(field.to_string(), "1".to_string())];
        assert_eq!(
            transport.exchange(&request),
            Err(TransportFailure::Network),
            "{field}"
        );
    }
    assert!(server.heads().is_empty());
}

/// The public internet is `https` at a public address: a name that resolves
/// to loopback is refused before a connection, and a plain `http` URL is
/// refused before its name is resolved.
#[test]
fn the_public_internet_reaches_no_loopback_name() {
    let resolver = scripted(&[("loopback.test", &[&["::ffff:127.0.0.1"]])]);
    let mut transport = transport(
        DestinationPolicy::public_internet().resolving_with(resolver.clone()),
        NetworkSettings::default(),
    );
    assert_eq!(
        transport.exchange(&get("http://loopback.test/ok".to_string())),
        Err(TransportFailure::DestinationRefused)
    );
    assert!(asked(&resolver).is_empty());
    assert_eq!(
        transport.exchange(&get("https://loopback.test/ok".to_string())),
        Err(TransportFailure::DestinationRefused)
    );
    assert_eq!(asked(&resolver), ["loopback.test"]);
}

/// The reference host over this transport answers a refused destination with
/// the operation's own failure code.
#[test]
fn the_reference_host_answers_a_refused_destination_with_its_code() {
    let resolver = scripted(&[("loopback.test", &[&["127.0.0.1"]])]);
    let transport = transport(
        DestinationPolicy::public_internet().resolving_with(resolver),
        NetworkSettings::default(),
    );
    let mut host = ReferenceHost::new(Box::new(ScriptedClock::new()), transport);
    let request: Params = [
        ("method".to_string(), "GET".to_string()),
        ("url".to_string(), "https://loopback.test/items".to_string()),
        ("headers".to_string(), String::new()),
        ("body".to_string(), String::new()),
    ]
    .into();
    let outcome = host.start(
        "sce-host-http-fetch",
        &Invocation::new("session", "fetch", 1),
        &request,
    );
    let data = match outcome {
        Some(Outcome::Done(data)) => data,
        other => panic!("the host did not complete: {other:?}"),
    };
    assert!(data.contains(r#""error":"destination_refused""#), "{data}");
}
