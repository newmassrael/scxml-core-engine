// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The C11 ctest fixture's BasicHTTP listener takes a port the kernel assigns
//! and hands it to its runners through a port file
//! (`tests/w3c/http_server_fixture.sh`, `tests/w3c/basic_http_test_endpoint.h`).
//!
//! The fixture used to bind one fixed port, which every C11 BasicHTTP runner
//! also compiled in as a default. On a host where another account's program
//! held that port the fixture died, and every runner that required it was
//! reported Not Run. ctest fixes each test's environment when the tree is
//! configured, before the listener has a port, so the port has to reach the
//! runners through a file the fixture writes once the listener has bound.
//!
//! What this holds: the `start` arm told port 0 writes a nonzero port into the
//! port file; the listener answers a BasicHTTP POST at that port; the `stop`
//! arm removes the file, so a later run never reads the port of a listener
//! that is gone.

use std::ffi::OsString;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::Duration;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build sits inside the repository")
        .to_path_buf()
}

fn fixture(args: &[OsString]) -> Output {
    Command::new("bash")
        .arg(repo_root().join("tests/w3c/http_server_fixture.sh"))
        .args(args)
        .output()
        .expect("run tests/w3c/http_server_fixture.sh")
}

/// Stops the listener however the test ends, so a failing assertion leaves no
/// server running behind it.
struct StopOnDrop {
    pid_file: PathBuf,
    port_file: PathBuf,
}

impl Drop for StopOnDrop {
    fn drop(&mut self) {
        let _ = fixture(&[
            "stop".into(),
            self.pid_file.clone().into(),
            self.port_file.clone().into(),
        ]);
    }
}

/// POST a BasicHTTP form naming `event` to the listener at `port`, and answer
/// the whole response it sent back.
fn post_event(port: u16, event: &str) -> String {
    let body = format!("_scxmleventname={event}");
    let request = format!(
        "POST /test HTTP/1.1\r\nHost: localhost:{port}\r\n\
         Content-Type: application/x-www-form-urlencoded\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let mut stream =
        TcpStream::connect(("localhost", port)).expect("connect to the fixture listener");
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("set a read timeout");
    stream.write_all(request.as_bytes()).expect("send the POST");
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .expect("read the answer");
    response
}

#[test]
fn the_fixture_binds_a_port_the_kernel_assigns_and_writes_it_for_its_runners() {
    let Some(node) =
        sce_build::toolchain::require_or_skip("node", "the C11 BasicHTTP fixture listener")
    else {
        return;
    };
    let dir = tempfile::tempdir().expect("tempdir");
    let pid_file = dir.path().join("w3c_http_server.pid");
    let port_file = dir.path().join("w3c_http_server.port");
    let stop = StopOnDrop {
        pid_file: pid_file.clone(),
        port_file: port_file.clone(),
    };

    let started = fixture(&[
        "start".into(),
        node.into(),
        repo_root()
            .join("tests/w3c/standalone_http_server.js")
            .into(),
        "0".into(),
        "/test".into(),
        pid_file.clone().into(),
        port_file.clone().into(),
    ]);
    assert!(
        started.status.success(),
        "the fixture's start arm failed:\n{}{}",
        String::from_utf8_lossy(&started.stdout),
        String::from_utf8_lossy(&started.stderr)
    );

    let written = std::fs::read_to_string(&port_file)
        .expect("the start arm wrote no port file, so a runner would have no port to post to");
    let port: u16 = written
        .trim_end()
        .parse()
        .unwrap_or_else(|_| panic!("the port file holds {written:?}, which is not a port"));
    assert_ne!(
        port, 0,
        "the port file names port 0, which no listener holds"
    );

    let answer = post_event(port, "probe");
    assert!(
        answer.contains("\"event\":\"probe\""),
        "the listener at the port the fixture wrote did not echo the posted event:\n{answer}"
    );

    drop(stop);
    assert!(
        !port_file.exists(),
        "the stop arm left the port file behind, so a later run could read the port of a \
         listener that is gone"
    );
}
