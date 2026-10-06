// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Asking a model server what it is before it is registered.
//!
//! A person types an address, and the screen says whether a server is there, whether it can be
//! talked to as this build talks (an OpenAI-compatible root), and which models it lists to choose
//! among. What is held: each way the answer can go is a state of its own (a screen acts on each
//! differently: a typo, a server that is off, a server that wants a key, a certificate that is not
//! trusted), the server is where the address says and this computer is told from another, an
//! address a connection could not keep is refused as a connection refuses it, and only the desktop
//! window may ask, because asking calls whatever address it is given. The server is a thread.
//!
//! Unix only: the model server stand-in shares a module with the authoring server's shell script.

#![cfg(unix)]

mod common;

use std::net::TcpListener;
use std::sync::Arc;
use std::thread;

use rustls::pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer};
use rustls::{ServerConfig, ServerConnection, StreamOwned};
use sce_app_core::server_status::{read, Reach, ServerState};
use sce_app_core::{call_in, ConnectionStore, Context, Entrance, Policy, WorkStore};
use serde_json::{json, Value};

use common::model_server::{chat_server, Script};
use common::{scratch, FakeRenderer};

fn listing(ids: &[&str]) -> Script {
    let data: Vec<Value> = ids
        .iter()
        .map(|id| json!({"id": id, "object": "model"}))
        .collect();
    Script::Reply(200, json!({"object": "list", "data": data}).to_string())
}

#[test]
fn a_server_that_lists_models_is_listed_with_them_in_its_order_and_is_this_computer() {
    let server = chat_server(vec![listing(&["qwen3-coder:30b", "devstral:24b"])]);

    let status = read(&server.address).unwrap();

    assert_eq!(
        status.server,
        ServerState::Listed {
            models: vec!["qwen3-coder:30b".to_string(), "devstral:24b".to_string()]
        }
    );
    assert_eq!(status.address, server.address);
    assert_eq!(status.reach, Reach::ThisComputer);
    assert!(!status.tls);
    // The request is the listing of the protocol and nothing else: no key, no work.
    let asked = &server.requests()[0];
    assert!(
        asked.head.starts_with("GET /v1/models HTTP/1.1"),
        "{}",
        asked.head
    );
    assert!(
        !asked.head.to_ascii_lowercase().contains("authorization"),
        "{}",
        asked.head
    );
}

#[test]
fn a_server_with_nothing_loaded_is_listed_with_none() {
    let server = chat_server(vec![listing(&[])]);

    let status = read(&server.address).unwrap();

    assert_eq!(status.server, ServerState::Listed { models: Vec::new() });
}

#[test]
fn nobody_there_is_unreachable_and_says_so() {
    let status = read("http://127.0.0.1:1/v1").unwrap();

    let ServerState::Unreachable { reason } = status.server else {
        panic!("expected unreachable, got {:?}", status.server);
    };
    assert!(reason.contains("could not be reached"), "{reason}");
}

#[test]
fn a_server_that_wants_a_key_is_told_apart_from_one_that_is_not_a_model_server() {
    let keyed = chat_server(vec![Script::Reply(
        401,
        r#"{"error":{"message":"missing bearer"}}"#.to_string(),
    )]);
    // A server that says "forbidden" and not "unauthorized" wants a key as much.
    let forbidden = chat_server(vec![Script::Reply(
        403,
        r#"{"error":"invalid api key"}"#.to_string(),
    )]);
    let elsewhere = chat_server(vec![Script::Reply(
        404,
        r#"{"detail":"Not Found"}"#.to_string(),
    )]);
    let page = chat_server(vec![Script::Reply(200, "<html>hello</html>".to_string())]);

    let keyed = read(&keyed.address).unwrap();
    let forbidden = read(&forbidden.address).unwrap();
    let elsewhere = read(&elsewhere.address).unwrap();
    let page = read(&page.address).unwrap();

    assert!(
        matches!(keyed.server, ServerState::NeedsKey { .. }),
        "{:?}",
        keyed.server
    );
    assert!(
        matches!(forbidden.server, ServerState::NeedsKey { .. }),
        "{:?}",
        forbidden.server
    );
    let ServerState::NotAModelList { reason } = &elsewhere.server else {
        panic!("expected not a model list, got {:?}", elsewhere.server);
    };
    assert!(
        reason.contains("404") || reason.contains("Not Found"),
        "{reason}"
    );
    let ServerState::NotAModelList { reason } = &page.server else {
        panic!("expected not a model list, got {:?}", page.server);
    };
    assert!(reason.contains("not a list of models"), "{reason}");
}

#[test]
fn another_computer_is_told_from_this_one_and_https_from_http() {
    // Nothing listens, so the answer is unreachable; what is held is what is said of the address.
    let near = read("http://localhost:1/v1").unwrap();
    let far = read("http://192.0.2.1:1/v1").unwrap();
    let secure = read("https://127.0.0.1:1/v1").unwrap();

    assert_eq!((near.reach, near.tls), (Reach::ThisComputer, false));
    assert_eq!((far.reach, far.tls), (Reach::Network, false));
    assert_eq!((secure.reach, secure.tls), (Reach::ThisComputer, true));
}

#[test]
fn a_server_whose_certificate_is_not_trusted_is_a_state_of_its_own_and_is_sent_nothing() {
    // A certificate made for `localhost` by itself: no authority the build trusts vouches for it.
    let certified = rcgen::generate_simple_self_signed(vec!["localhost".to_string()]).unwrap();
    let config = Arc::new(
        ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_safe_default_protocol_versions()
            .unwrap()
            .with_no_client_auth()
            .with_single_cert(
                vec![certified.cert.der().clone()],
                PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(certified.key_pair.serialize_der())),
            )
            .unwrap(),
    );
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = format!(
        "https://localhost:{}/v1",
        listener.local_addr().unwrap().port()
    );
    let (told, request) = std::sync::mpsc::channel();
    thread::spawn(move || {
        use std::io::Read;
        let (tcp, _) = listener.accept().unwrap();
        let mut stream = StreamOwned::new(ServerConnection::new(config).unwrap(), tcp);
        let mut bytes = [0u8; 16];
        // The client refuses the certificate and leaves: what the server reads is nothing.
        let _ = told.send(stream.read(&mut bytes).unwrap_or(0));
    });

    let status = read(&address).unwrap();

    let ServerState::Certificate { reason } = &status.server else {
        panic!("expected a certificate refused, got {:?}", status.server);
    };
    assert!(
        reason.contains("authority this application does not trust"),
        "{reason}"
    );
    assert!(status.tls);
    assert_eq!(request.recv().unwrap(), 0);
}

#[test]
fn an_address_a_connection_could_not_keep_is_refused_as_a_connection_refuses_it() {
    for address in [
        "ftp://127.0.0.1/v1",
        "127.0.0.1:11434",
        "http://user:key@127.0.0.1/v1",
        "http://127.0.0.1/v1?key=1",
        "http://",
    ] {
        let refused = read(address).unwrap_err();

        assert!(
            refused.to_string().contains("not a server address"),
            "{address}: {refused}"
        );
    }
}

// ---- the command -----------------------------------------------------------------------------

fn ask(entrance: Entrance, args: Value) -> Result<Value, sce_app_core::CommandError> {
    let works = WorkStore::at(scratch("ss-works"));
    let settings = ConnectionStore::at(scratch("ss-settings"));
    let policy = Policy::shipped();
    let context =
        Context::new(&works, &FakeRenderer, &policy, entrance).with_connections(Some(&settings));
    call_in(&context, "read_server_status", args)
}

#[test]
fn the_command_answers_with_the_server_and_what_it_listed() {
    let server = chat_server(vec![listing(&["qwen3-coder:30b"])]);

    let said = ask(Entrance::Desktop, json!({ "server_url": server.address })).unwrap();

    assert_eq!(
        said,
        json!({ "server": {
            "address": server.address,
            "reach": "this-computer",
            "tls": false,
            "state": "listed",
            "models": ["qwen3-coder:30b"],
        } })
    );
}

#[test]
fn only_the_desktop_window_may_call_an_address() {
    let tool = ask(
        Entrance::Tool,
        json!({ "server_url": "http://127.0.0.1:1/v1" }),
    )
    .unwrap_err();
    let browser = ask(
        Entrance::Browser,
        json!({ "server_url": "http://127.0.0.1:1/v1" }),
    )
    .unwrap_err();

    assert_eq!(tool.kind, "not-allowed-here");
    assert_eq!(browser.kind, "not-allowed-here");
}

#[test]
fn a_bad_address_and_an_argument_that_is_not_one_are_refused() {
    let bad = ask(Entrance::Desktop, json!({ "server_url": "ftp://x" })).unwrap_err();
    let extra = ask(
        Entrance::Desktop,
        json!({ "server_url": "http://127.0.0.1:1/v1", "key": "sk-secret" }),
    )
    .unwrap_err();
    let missing = ask(Entrance::Desktop, json!({})).unwrap_err();

    assert_eq!(bad.kind, "bad-connection");
    // A key cannot be given: this build has no place to keep one, and none is sent.
    assert_eq!(extra.kind, "bad-request");
    assert_eq!(missing.kind, "bad-request");
}
