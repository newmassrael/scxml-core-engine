// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Zenoh binding across a real Zenoh router, with upstream's `zenoh` crate as the client of
//! both machines.
//!
//! The binding depends on the protocol, not on a router: any router of the protocol version the
//! implementation pins (§mesh-20.4) carries it. The harness therefore speaks the command line of
//! the reference router, upstream's `zenohd` (`--listen`, `--connect`, `--no-multicast-scouting`),
//! picks each router's port itself and takes a router as ready once it accepts a connection there.
//! `ZENOH_ROUTER_BIN` names the router to run, `zenohd` on the `PATH` when unset; a router that
//! takes `zenohd`'s three options can be named instead. The router needs the liveliness plane the
//! binding's readiness rides on (§mesh-20.3), which `zenohd` has.
//!
//! Opt-in, since it needs a router binary:
//!
//! ```text
//! ZENOH_ROUTER_BIN=/path/to/zenohd cargo test -p sce-rust-mesh --features zenoh,conformance \
//!     --test zenoh_through_a_router -- --ignored --nocapture
//! ```
//!
//! What it holds: two machines that only know each other's names learn each is up through the
//! router (§mesh-20.3), an envelope one sends reaches the other's engine under the sender's name
//! (§mesh-20.1, §mesh-20.2), and a machine that goes away is told to the other as a lost link
//! (§mesh-20.3), all through a real router with upstream's client (§mesh-20.4). A machine that
//! loses its session when the router is killed and started again on the same port comes back
//! knowing only who is still there.
//! Nothing here is a timing claim; every wait has a generous bound and the assertions are on what
//! arrived.
//!
//! The assertions are not in this file. They are the conformance kit
//! (`sce_rust_mesh::conformance`, feature `conformance`), which holds any client of the binding to
//! the same behaviour; this file is the router process and the harness that connects upstream's
//! client to it.

#![cfg(all(feature = "zenoh", feature = "conformance"))]

use std::net::{TcpListener, TcpStream};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use sce_rust_mesh::conformance::{self, Disruptable, Harness};
use sce_rust_mesh::inbox::ClientError;
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

/// The router to run: `ZENOH_ROUTER_BIN`, or upstream's `zenohd` on the `PATH`.
fn router_binary() -> String {
    std::env::var("ZENOH_ROUTER_BIN").unwrap_or_else(|_| "zenohd".to_string())
}

/// A port nothing listens on at this moment, which the router is then told to listen on.
fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .and_then(|listener| listener.local_addr())
        .map(|addr| addr.port())
        .expect("the system hands out a free port")
}

/// Start a router, federated with the router listening on `connect_to` when one is named.
fn start_router(connect_to: Option<u16>) -> RouterProcess {
    start_router_on(connect_to, None)
}

/// [`start_router`], listening on `listen` when a port is named and on a free one when not.
fn start_router_on(connect_to: Option<u16>, listen: Option<u16>) -> RouterProcess {
    try_start_router(connect_to, listen).expect("the router starts and accepts on its port")
}

/// Start a router and wait until it accepts a connection on its port. Fails when the process ends
/// first, which is what a port that is still held gives.
fn try_start_router(connect_to: Option<u16>, listen: Option<u16>) -> Result<RouterProcess, String> {
    let port = listen.unwrap_or_else(free_port);
    let mut args = vec![
        "--no-multicast-scouting".to_string(),
        "--listen".to_string(),
        format!("tcp/127.0.0.1:{port}"),
    ];
    if let Some(peer) = connect_to {
        args.push("--connect".to_string());
        args.push(format!("tcp/127.0.0.1:{peer}"));
    }
    let child = Command::new(router_binary())
        .args(&args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("the router did not start: {e}"))?;
    // The process is owned from here, so a failure below kills it and waits for it.
    let mut process = RouterProcess { child, port };
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        if let Some(status) = process
            .child
            .try_wait()
            .map_err(|e| format!("the router's state cannot be read: {e}"))?
        {
            return Err(format!(
                "the router ended before it accepted on port {port}: {status}"
            ));
        }
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return Ok(process);
        }
        if Instant::now() > deadline {
            return Err(format!(
                "the router did not accept on port {port} within 30 s"
            ));
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// The address a client dials to reach the router on `port`.
fn dial(port: u16) -> Result<UpstreamClient, ClientError> {
    let config = client_config(&[&format!("tcp/127.0.0.1:{port}")]).map_err(ClientError::new)?;
    UpstreamClient::connect(config).map_err(ClientError::new)
}

/// Upstream's client, dialing the router listening on `port`. Every machine gets a session of its
/// own, so each has its own session identity as it had when the binding opened its own.
struct UpstreamHarness {
    port: u16,
}

impl Harness for UpstreamHarness {
    type Client = UpstreamClient;

    fn connect(&self) -> UpstreamClient {
        dial(self.port).expect("a session opens")
    }
}

/// A router that is killed and started again on the same port, and the process that is the
/// router at the moment.
struct RestartableRouter {
    port: u16,
    process: Mutex<Option<RouterProcess>>,
}

/// Upstream's client on a router that can be lost: [`Disruptable::drop_every_session`] kills it and
/// starts it again on the same port. Clones share the router.
#[derive(Clone)]
struct RestartingHarness {
    router: Arc<RestartableRouter>,
}

impl RestartingHarness {
    fn start() -> Self {
        let process = start_router(None);
        RestartingHarness {
            router: Arc::new(RestartableRouter {
                port: process.port,
                process: Mutex::new(Some(process)),
            }),
        }
    }
}

impl Harness for RestartingHarness {
    type Client = UpstreamClient;

    fn connect(&self) -> UpstreamClient {
        dial(self.router.port).expect("a session opens")
    }
}

impl Disruptable for RestartingHarness {
    fn drop_every_session(&self) {
        let mut process = self.router.process.lock().unwrap();
        // Dropping the process kills it and waits for it, which closes every session it carried.
        drop(process.take());
        // The port can be held for a moment after the process is gone: ask again until it is free.
        let mut last = String::new();
        for _ in 0..40 {
            match try_start_router(None, Some(self.router.port)) {
                Ok(restarted) => {
                    assert_eq!(restarted.port, self.router.port, "the same address");
                    *process = Some(restarted);
                    return;
                }
                Err(why) => last = why,
            }
            std::thread::sleep(Duration::from_millis(250));
        }
        panic!(
            "the router did not start again on port {}: {last}",
            self.router.port
        );
    }

    fn try_connect(&self) -> Result<UpstreamClient, ClientError> {
        dial(self.router.port)
    }
}

#[test]
#[ignore = "needs a Zenoh router: upstream's zenohd on the PATH, or one named by ZENOH_ROUTER_BIN"]
fn two_machines_meet_through_a_router_and_one_reaches_the_others_engine() {
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
#[ignore = "needs a Zenoh router: upstream's zenohd on the PATH, or one named by ZENOH_ROUTER_BIN"]
fn two_machines_on_two_federated_routers_meet_as_if_on_one() {
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
#[ignore = "needs a Zenoh router: upstream's zenohd on the PATH, or one named by ZENOH_ROUTER_BIN"]
fn a_device_that_was_away_catches_up_from_a_hub_that_stayed_up() {
    let router = start_router(None);
    conformance::a_device_that_was_away_catches_up_from_a_hub_that_stayed_up(&UpstreamHarness {
        port: router.port,
    });
}

/// The router goes away and comes back on the same address. Machines on `open_reconnecting` are
/// told every peer they had ready is lost, then find each other again from the presence history,
/// and a machine that left during the gap is never believed to be up. Upstream's client tells the
/// binding its session ended by watching the node ids the session is connected to and closes the
/// session itself, so the machine that does not reconnect does not return on its own.
#[test]
#[ignore = "needs a Zenoh router: upstream's zenohd on the PATH, or one named by ZENOH_ROUTER_BIN"]
fn a_machine_that_loses_its_session_comes_back_and_resyncs_through_a_router() {
    let harness = RestartingHarness::start();
    conformance::a_machine_that_loses_its_session_comes_back_and_resyncs(&harness);
}
