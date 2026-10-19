// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Zenoh binding across a real watching-zenoh router, with upstream's `zenoh` crate as the
//! client of both machines.
//!
//! Opt-in: it needs a built `wz-ap-demo` with `router-hat-router` and `routing-token-tables` (the
//! liveliness plane the binding's readiness rides on), named by `WZ_ROUTER_BIN`:
//!
//! ```text
//! WZ_ROUTER_BIN=/path/to/wz-ap-demo cargo test -p sce-rust-mesh --features zenoh,conformance \
//!     --test zenoh_through_a_wz_router -- --ignored --nocapture
//! ```
//!
//! What it holds: two machines that only know each other's names learn each is up through the
//! router (§mesh-20.3), an envelope one sends reaches the other's engine under the sender's name
//! (§mesh-20.1, §mesh-20.2), and a machine that goes away is told to the other as a lost link
//! (§mesh-20.3), all through a real watching-zenoh router with upstream's client (§mesh-20.4).
//! Nothing here is a timing claim; every wait has a generous bound and the assertions are on what
//! arrived.
//!
//! The assertions are not in this file. They are the conformance kit
//! (`sce_rust_mesh::conformance`, feature `conformance`), which holds any client of the binding to
//! the same behaviour; this file is the router process and the harness that connects upstream's
//! client to it.

#![cfg(all(feature = "zenoh", feature = "conformance"))]

use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use sce_rust_mesh::conformance::{self, Harness};
use sce_rust_mesh::zenoh::{client_config, UpstreamClient};

/// The router process, killed when the test ends however it ends.
struct RouterProcess {
    child: Child,
    port: u16,
}

impl Drop for RouterProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Start a router, federated with the router listening on `connect_to` when one is named.
fn start_router(connect_to: Option<u16>) -> RouterProcess {
    let binary = std::env::var("WZ_ROUTER_BIN").expect(
        "WZ_ROUTER_BIN names a wz-ap-demo built with `router-hat-router,routing-token-tables`",
    );
    let mut args = vec!["--router-hat".to_string(), "127.0.0.1:0".to_string()];
    if let Some(port) = connect_to {
        args.push("--connect".to_string());
        args.push(format!("127.0.0.1:{port}"));
    }
    let mut child = Command::new(binary)
        .args(&args)
        .env("RUST_LOG", "info")
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the router starts");
    let stderr = child.stderr.take().expect("stderr is piped");
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            if let Some(rest) = line.split("listening on 127.0.0.1:").nth(1) {
                let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
                if let Ok(port) = digits.parse::<u16>() {
                    let _ = tx.send(port);
                }
            }
        }
    });
    let port = rx
        .recv_timeout(Duration::from_secs(30))
        .expect("the router said which port it listens on");
    RouterProcess { child, port }
}

/// Upstream's client, dialing the router listening on `port`. Every machine gets a session of its
/// own, so each has its own session identity as it had when the binding opened its own.
struct UpstreamHarness {
    port: u16,
}

impl Harness for UpstreamHarness {
    type Client = UpstreamClient;

    fn connect(&self) -> UpstreamClient {
        let config = client_config(&[&format!("tcp/127.0.0.1:{}", self.port)]).expect("a config");
        UpstreamClient::connect(config).expect("a session opens")
    }
}

#[test]
#[ignore = "needs WZ_ROUTER_BIN: a built wz-ap-demo with router-hat-router and routing-token-tables"]
fn two_machines_meet_through_a_wz_router_and_one_reaches_the_others_engine() {
    // A router for each function: both name their machines "server" and "phone", and a name that
    // a closed session has only just left must not be seen by the next one that takes it.
    let router = start_router(None);
    conformance::two_machines_meet(&UpstreamHarness { port: router.port });
    let router = start_router(None);
    conformance::a_machine_that_goes_away_is_told_as_lost(&UpstreamHarness { port: router.port });
}

/// The scaling claim, as far as one host can hold it: the machines are on DIFFERENT routers, joined
/// by a federation link, and still see each other up, reach each other's engine, and see each other
/// go. A device connects to whichever router it is assigned and the server to its own; nothing in
/// the binding says which.
#[test]
#[ignore = "needs WZ_ROUTER_BIN: a built wz-ap-demo with router-hat-router and routing-token-tables"]
fn two_machines_on_two_federated_wz_routers_meet_as_if_on_one() {
    let first = start_router(None);
    let second = start_router(Some(first.port));
    let server_side = UpstreamHarness { port: first.port };
    let phone_side = UpstreamHarness { port: second.port };
    conformance::two_machines_meet_across(&server_side, &phone_side);

    let first = start_router(None);
    let second = start_router(Some(first.port));
    let server_side = UpstreamHarness { port: first.port };
    let phone_side = UpstreamHarness { port: second.port };
    conformance::a_machine_that_goes_away_is_told_as_lost_across(&server_side, &phone_side);
}

/// A device that was not there when a note was pushed asks the hub, on its own visit, for what it
/// has not seen, and gets it: the shape of a shared list whose devices are on only while their
/// user presses sync. The router holds nothing for an absent machine; what the latecomer is given
/// is the hub's state.
#[test]
#[ignore = "needs WZ_ROUTER_BIN: a built wz-ap-demo with router-hat-router and routing-token-tables"]
fn a_device_that_was_away_catches_up_from_a_hub_that_stayed_up() {
    let router = start_router(None);
    conformance::a_device_that_was_away_catches_up_from_a_hub_that_stayed_up(&UpstreamHarness {
        port: router.port,
    });
}
