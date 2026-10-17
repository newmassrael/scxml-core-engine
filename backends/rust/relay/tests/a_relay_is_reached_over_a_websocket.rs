// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
//
// The relay over a socket. A device connects with the token its login earned and exchanges request
// and response texts; a connection with no token, or one the relay did not issue, is refused at the
// upgrade; a token that expires while the connection stays open stops being answered; and two hosts
// that never meet carry a write from one to the other through the relay by their couriers.

use std::sync::{Arc, Mutex};

use sce_rust_mesh::endpoint::Sealer;
use sce_rust_relay::serve::{serve, WssLink};
use sce_rust_relay::{
    Authority, Code, Courier, Limits, Link, MemoryStore, Relay, Request, Response,
};
use sce_rust_trust::membership::{Body, Log, Profile};
use sce_rust_trust::{Identity, Trust};
use serde_json::json;
use tokio::net::TcpListener;

const LIST: &str = "home";

fn person(node: u64) -> Identity {
    Identity::from_seed([node as u8; 32])
}

fn admin() -> Identity {
    Identity::from_seed([9; 32])
}

/// A host of the list for `node`, holding the whole log (alice and bob write).
fn host(node: u64) -> Trust {
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
    trust.set_identity(node, person(node));
    trust
}

struct Running {
    runtime: tokio::runtime::Runtime,
    authority: Authority,
    base: String,
    now: Arc<Mutex<u64>>,
}

fn start() -> Running {
    let authority = Authority::new([4; 32], "relay-over-a-socket");
    let relay = Arc::new(Mutex::new(
        Relay::open(MemoryStore::new(), authority.clone(), Limits::default()).unwrap(),
    ));
    let now = Arc::new(Mutex::new(0u64));
    let clock: Arc<dyn Fn() -> u64 + Send + Sync> = {
        let now = now.clone();
        Arc::new(move || *now.lock().unwrap())
    };
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let listener = runtime.block_on(TcpListener::bind("127.0.0.1:0")).unwrap();
    let address = listener.local_addr().unwrap();
    runtime.spawn(serve(listener, relay, clock, 64 * 1024));
    Running {
        runtime,
        authority,
        base: format!("ws://{address}"),
        now,
    }
}

#[test]
fn a_connection_needs_a_token_the_relay_issued_and_asks_again_when_it_expires() {
    let running = start();
    let _keep = &running.runtime;

    for bad in ["", "nonsense"] {
        let mut link = WssLink::new(&running.base, bad).unwrap();
        assert!(link.call(&Request::Heads).is_err(), "{bad:?}");
    }
    let other = Authority::new([4; 32], "another-relay").issue(LIST, 1, u64::MAX);
    assert!(WssLink::new(&running.base, &other)
        .unwrap()
        .call(&Request::Heads)
        .is_err());

    // A token that is good is answered, and the relay says what it is not told yet.
    let token = running.authority.issue(LIST, 1, 1_000);
    let mut link = WssLink::new(&running.base, &token).unwrap();
    let response = link.call(&Request::Entries { from: 0 }).unwrap();
    assert_eq!(
        response,
        Response::Entries {
            entries: Vec::new(),
            len: 0
        }
    );
    // The connection stays open, and the clock passes the token's expiry: it is not answered.
    *running.now.lock().unwrap() = 1_000;
    let response = link.call(&Request::Entries { from: 0 }).unwrap();
    assert!(
        matches!(
            response,
            Response::Refused {
                code: Code::Unauthorized,
                ..
            }
        ),
        "{response:?}"
    );
}

#[test]
fn two_hosts_that_never_meet_carry_a_write_through_the_relay_over_a_socket() {
    let running = start();
    let _keep = &running.runtime;

    let alice = host(1);
    let mut sealer = alice.clone();
    sealer
        .seal(
            "#server",
            "todo.write",
            &json!({"id":"1","field":"3","value":"7","text":"","wallTime":"1000",
                    "counter":"0","nodeId":"1"})
            .to_string(),
        )
        .unwrap();
    let mut alice_courier = Courier::new(
        alice,
        WssLink::new(&running.base, &running.authority.issue(LIST, 1, u64::MAX)).unwrap(),
    );
    let round = alice_courier.sync().unwrap();
    assert_eq!(round.pushed, 3 + 1, "the log and the write");
    assert!(round.refused.is_empty(), "{:?}", round.refused);

    let bob = Trust::open(LIST);
    bob.set_identity(2, person(2));
    bob.append(&host(2).entries()[0]).unwrap();
    let mut bob_courier = Courier::new(
        bob,
        WssLink::new(&running.base, &running.authority.issue(LIST, 2, u64::MAX)).unwrap(),
    );
    let round = bob_courier.sync().unwrap();
    assert!(round.refused.is_empty(), "{:?}", round.refused);
    assert_eq!(
        round
            .events
            .iter()
            .filter(|(event, _)| event == "todo.write")
            .count(),
        1
    );
}
