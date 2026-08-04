// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// custom_tcp's socket layer belongs to the deployment, not to the test
// harness that first justified its values.
//
// Every socket-layer constant in `CustomTcpTransport.h` carried a comment
// naming the harness as its reason: `SO_REUSEADDR` "keeps tests
// deterministic across rapid teardowns", `TCP_NODELAY` "keep harness
// latencies deterministic ... tests measure end-to-end ordering, not
// throughput", the dial retry "covers ctest startup jitter". They are
// good defaults and they remain the defaults — but they were also the
// only values obtainable, and custom_tcp is SCE's own implementation, so
// there is no `zenoh.json5` / `vsomeip.json` / `cyclonedds.xml`
// equivalent a deployer could reach past the schema with. An option
// absent from this schema is an option that does not exist.
//
// Cyclone DDS, the transport this project measures itself against on
// this axis, exposes `SocketReceiveBufferSize` / `SocketSendBufferSize`
// among others; a reference transport letting a deployment size its
// socket buffers while SCE's own did not was the clearest form of the
// gap.
//
// This file covers the declaration end: a declared value must reach the
// emitted `SocketOptions`, and an absent one must reach it as the
// historical literal. The behavioural end — that the runtime applies
// what it is handed — is `tests/mesh/CustomTcpSocketOptionsTest.cpp`.

use std::fs;
use std::io::Write;
use std::path::PathBuf;

use sce_build::generator::Language;

const BRAKE_SCXML: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="brake" initial="idle">
  <state id="idle">
    <transition event="start" target="active">
      <send target="#motor" event="service.fire_forget.activate"/>
    </transition>
  </state>
  <state id="active"/>
</scxml>
"##;

const MOTOR_SCXML: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="motor" initial="ready">
  <state id="ready">
    <transition event="service.fire_forget.activate" target="ready"/>
  </state>
</scxml>
"##;

struct Fixture {
    dir: PathBuf,
}

impl Fixture {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("sce_mesh_tcp_socket_{tag}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("mkdir");
        Self { dir }
    }

    fn write(&self, name: &str, content: &str) -> PathBuf {
        let path = self.dir.join(name);
        let mut f = fs::File::create(&path).expect("create");
        f.write_all(content.as_bytes()).expect("write");
        path
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn compile(tag: &str, tcp_keys: &str) -> Result<String, sce_build::mesh::error::MeshError> {
    let fx = Fixture::new(tag);
    fx.write("brake.scxml", BRAKE_SCXML);
    fx.write("motor.scxml", MOTOR_SCXML);
    let deploy_path = fx.write(
        "deploy.yaml",
        &format!(
            r##"
version: "1.0"
topology:
  ecu1:
    transports:
      custom_tcp:
{tcp_keys}    machines:
      brake:
        source: brake.scxml
        bindings:
          "#motor":
            transport: custom_tcp
            connect: "127.0.0.1:9101"
      motor:
        source: motor.scxml
"##
        ),
    );

    let mut parser = sce_build::parser::SCXMLParser::new();
    let mut model = parser.parse_string(BRAKE_SCXML, "brake").expect("parse");
    sce_build::compile_mesh_transport(&mut model, &deploy_path, Language::Cpp)
        .map(|r| r.output.files[0].1.clone())
}

#[test]
fn declared_socket_options_reach_the_emitted_struct() {
    // Values chosen so none can be confused with a default or with each
    // other — a template that crossed two assignments would still pass a
    // test written with repeated numbers.
    let code = compile(
        "declared",
        "        listen: \"127.0.0.1:9100\"\n\
         \x20       backlog: 512\n\
         \x20       reuse_addr: false\n\
         \x20       nodelay: false\n\
         \x20       connect_max_attempts: 7\n\
         \x20       connect_retry_interval_ms: 125\n\
         \x20       recv_buffer_bytes: 262144\n\
         \x20       send_buffer_bytes: 131072\n\
         \x20       keepalive: true\n\
         \x20       keepalive_idle_s: 17\n\
         \x20       keepalive_interval_s: 3\n\
         \x20       keepalive_count: 9\n",
    )
    .expect("compile");

    for expected in [
        "o.backlog = 512",
        "o.reuse_addr = false",
        "o.nodelay = false",
        "o.connect_max_attempts = 7",
        "o.connect_retry_interval_ms = 125",
        "o.recv_buffer_bytes = 262144",
        "o.send_buffer_bytes = 131072",
        "o.keepalive = true",
        "o.keepalive_idle_s = 17",
        "o.keepalive_interval_s = 3",
        "o.keepalive_count = 9",
    ] {
        assert!(
            code.contains(expected),
            "declared socket option must reach the emitted SocketOptions: \
             expected `{expected}`"
        );
    }
}

#[test]
fn absent_socket_options_emit_the_historical_literals() {
    // The defaults are a contract, not an accident: they are the values
    // `CustomTcpTransport.h` carried before the fields existed, so every
    // deployment that declares nothing keeps byte-identical behaviour.
    let code = compile("absent", "        listen: \"127.0.0.1:9100\"\n").expect("compile");

    for expected in [
        "o.backlog = 16",
        "o.reuse_addr = true",
        "o.nodelay = true",
        "o.connect_max_attempts = 20",
        "o.connect_retry_interval_ms = 50",
        "o.recv_buffer_bytes = 0",
        "o.send_buffer_bytes = 0",
        // Off by default: enabling keepalive changes when an existing
        // deployment observes a peer disappear, so it is opt-in.
        "o.keepalive = false",
        "o.keepalive_idle_s = 60",
        "o.keepalive_interval_s = 10",
        "o.keepalive_count = 6",
    ] {
        assert!(
            code.contains(expected),
            "an undeclared socket option must emit its historical literal: \
             expected `{expected}`"
        );
    }
}

#[test]
fn socket_options_reach_a_pure_client_device() {
    // The axis a naive wiring would miss: a device with no `listen:` is a
    // pure TCP client, and `nodelay` / the dial retry / the buffer sizes
    // are exactly what such a device wants to tune. Hanging the socket
    // options off the same "is there a listen endpoint" gate that guards
    // the server would silently drop them here.
    let code = compile(
        "client_only",
        "        nodelay: false\n        connect_max_attempts: 3\n",
    )
    .expect("compile");

    assert!(
        code.contains("o.nodelay = false"),
        "a client-only device must still carry its socket options"
    );
    assert!(code.contains("o.connect_max_attempts = 3"));
}

#[test]
fn every_custom_tcp_client_reports_peer_loss() {
    // §16.7 row 8. custom_tcp was the only transport that could not raise
    // PEER_PARTITIONED, even though a connection-oriented transport holds
    // the liveness evidence more directly than the token / SD-flag / lease
    // the other three infer it from. The raise is unconditional — a clean
    // FIN reports without any configuration; `keepalive:` only extends it
    // to the crash-and-partition case.
    let code = compile("peer_loss", "        listen: \"127.0.0.1:9100\"\n").expect("compile");

    assert!(
        code.contains("setPeerLossHandler"),
        "every custom_tcp client must install the row 8 peer-loss handler"
    );
    assert!(
        code.contains("ReasonCode::PeerPartitioned"),
        "the peer-loss handler must raise PEER_PARTITIONED"
    );
    assert!(
        code.contains("err.target = \"#motor\""),
        "the raise must name the peer, which is what row 8 carries"
    );
}

#[test]
fn zero_keepalive_idle_is_rejected() {
    // A zero idle would ask the kernel to probe continuously. Rejected for
    // the same reason as the other socket values: omitting the field is
    // how an author takes the default.
    let err = match compile(
        "zero_keepalive",
        "        listen: \"127.0.0.1:9100\"\n        keepalive_idle_s: 0\n",
    ) {
        Err(e) => e,
        Ok(_) => panic!("keepalive_idle_s: 0 must not compile"),
    };
    assert!(
        err.to_string().contains("keepalive_idle_s"),
        "the diagnostic must name the offending field: {err}"
    );
}

#[test]
fn zero_backlog_is_rejected_at_parse_time() {
    // `listen(fd, 0)` refuses every peer connection, and its only symptom
    // is that nothing ever connects — indistinguishable at runtime from a
    // routing mistake. Omitting the field is how an author takes the
    // default; declaring 0 can only be an error.
    let err = match compile(
        "zero_backlog",
        "        listen: \"127.0.0.1:9100\"\n        backlog: 0\n",
    ) {
        Err(e) => e,
        Ok(_) => panic!("backlog: 0 must not compile"),
    };

    let text = err.to_string();
    assert!(
        text.contains("backlog"),
        "the diagnostic must name the offending field: {text}"
    );
    assert!(
        text.contains("omit"),
        "the diagnostic must name the repair — omitting takes the default: {text}"
    );
}

#[test]
fn zero_connect_retry_interval_is_rejected() {
    // A zero interval turns the retry loop into a busy-spin that burns a
    // core for the whole dial window. Covered separately from the backlog
    // case so a validator that checks only its first field cannot pass.
    let err = match compile(
        "zero_interval",
        "        listen: \"127.0.0.1:9100\"\n        connect_retry_interval_ms: 0\n",
    ) {
        Err(e) => e,
        Ok(_) => panic!("connect_retry_interval_ms: 0 must not compile"),
    };

    assert!(
        err.to_string().contains("connect_retry_interval_ms"),
        "the diagnostic must name the offending field: {err}"
    );
}
