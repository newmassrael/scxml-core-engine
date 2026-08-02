// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE_MESH §7.7 "Circular dependency detection" — the build tool warns
// when machines form a ring of cross-machine `<invoke>`s, because each
// participant holds its invoking state until the next answers.
//
// Four properties are pinned here, and the negative ones carry as much
// weight as the positive one:
//   * a mutual invoke pair is reported,
//   * it is reported by exactly one machine, so an N-machine ring does
//     not produce N copies across a full build,
//   * an acyclic invoke chain is silent,
//   * a mutual `<send>` pair is silent — sends are fire-and-forget and
//     mutual sends are the ordinary request/response topology, so
//     treating them as edges would make the check pure noise.

use std::fs;
use std::io::Write;
use std::path::PathBuf;

use sce_build::generator::Language;

struct Fixture {
    dir: PathBuf,
}

impl Fixture {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("sce_mesh_invoke_cycle_{tag}"));
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

/// A machine that mesh-rpc-invokes `peer` and answers `peer`'s request.
fn rpc_both_ways(name: &str, peer: &str) -> String {
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="{name}" initial="idle">
  <state id="idle">
    <transition event="service.request.work" target="calling"/>
  </state>
  <state id="calling">
    <invoke type="sce:mesh-rpc" src="#{peer}">
      <param name="_mesh_event" expr="'service.request.work'"/>
    </invoke>
    <transition event="done.invoke" target="ok"/>
  </state>
  <final id="ok"/>
</scxml>
"##
    )
}

/// A machine that mesh-rpc-invokes `peer` but answers nothing.
fn rpc_one_way(name: &str, peer: &str) -> String {
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="{name}" initial="calling">
  <state id="calling">
    <invoke type="sce:mesh-rpc" src="#{peer}">
      <param name="_mesh_event" expr="'service.request.work'"/>
    </invoke>
    <transition event="done.invoke" target="ok"/>
  </state>
  <final id="ok"/>
</scxml>
"##
    )
}

/// A machine that only answers.
fn leaf(name: &str) -> String {
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="{name}" initial="ready">
  <state id="ready">
    <transition event="service.request.work" target="ready"/>
  </state>
</scxml>
"##
    )
}

/// A machine that `<send>`s to `peer` and handles `peer`'s send back.
fn sends_both_ways(name: &str, peer: &str) -> String {
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="{name}" initial="idle">
  <state id="idle">
    <transition event="go" target="idle">
      <send target="#{peer}" event="service.fire_forget.ping"/>
    </transition>
    <transition event="service.fire_forget.ping" target="idle"/>
  </state>
</scxml>
"##
    )
}

fn deploy_two(a: &str, b: &str) -> String {
    format!(
        r##"
version: "1.0"
topology:
  ecu1:
    machines:
      {a}:
        source: {a}.scxml
        bindings:
          "#{b}": {{ transport: shm }}
      {b}:
        source: {b}.scxml
        bindings:
          "#{a}": {{ transport: shm }}
"##
    )
}

fn parse(src: &str, name: &str) -> sce_build::model::SCXMLModel {
    let mut parser = sce_build::parser::SCXMLParser::new();
    parser.parse_string(src, name).expect("parse")
}

#[test]
fn mutual_mesh_rpc_invokes_report_a_cycle() {
    let fx = Fixture::new("mutual");
    let alpha = rpc_both_ways("alpha", "beta");
    let beta = rpc_both_ways("beta", "alpha");
    fx.write("alpha.scxml", &alpha);
    fx.write("beta.scxml", &beta);
    let deploy = fx.write("deploy.yaml", &deploy_two("alpha", "beta"));

    let mut model = parse(&alpha, "alpha");
    let result = sce_build::compile_mesh_transport(&mut model, &deploy, Language::Cpp)
        .expect("compile_mesh_transport");

    assert_eq!(
        result.invoke_wait_cycles.len(),
        1,
        "alpha invokes beta and beta invokes alpha — one ring, got {:?}",
        result.invoke_wait_cycles
    );
    assert_eq!(
        result.invoke_wait_cycles[0].machines,
        vec!["alpha".to_string(), "beta".to_string()],
        "cycle lists both members starting from the smallest"
    );
    let rendered = result.invoke_wait_cycles[0].to_string();
    assert!(
        rendered.contains("alpha -> beta -> alpha"),
        "warning names the full ring, got: {rendered}"
    );
}

#[test]
fn a_ring_is_reported_by_only_its_smallest_member() {
    // Same deployment, compiled from the other end. Every machine in a
    // deployment gets compiled, so if both ends reported, a two-machine
    // ring would surface twice per build.
    let fx = Fixture::new("smallest");
    let alpha = rpc_both_ways("alpha", "beta");
    let beta = rpc_both_ways("beta", "alpha");
    fx.write("alpha.scxml", &alpha);
    fx.write("beta.scxml", &beta);
    let deploy = fx.write("deploy.yaml", &deploy_two("alpha", "beta"));

    let mut model = parse(&beta, "beta");
    let result = sce_build::compile_mesh_transport(&mut model, &deploy, Language::Cpp)
        .expect("compile_mesh_transport");

    assert!(
        result.invoke_wait_cycles.is_empty(),
        "beta is not the smallest member, so it must stay silent; got {:?}",
        result.invoke_wait_cycles
    );
}

#[test]
fn acyclic_invoke_chain_reports_nothing() {
    let fx = Fixture::new("acyclic");
    let alpha = rpc_one_way("alpha", "beta");
    let beta = leaf("beta");
    fx.write("alpha.scxml", &alpha);
    fx.write("beta.scxml", &beta);
    let deploy = fx.write("deploy.yaml", &deploy_two("alpha", "beta"));

    let mut model = parse(&alpha, "alpha");
    let result = sce_build::compile_mesh_transport(&mut model, &deploy, Language::Cpp)
        .expect("compile_mesh_transport");

    assert!(
        result.invoke_wait_cycles.is_empty(),
        "alpha -> beta with no edge back is not a ring; got {:?}",
        result.invoke_wait_cycles
    );
}

#[test]
fn mutual_sends_are_not_a_ring() {
    // The load-bearing negative. `<send>` is fire-and-forget, so a
    // mutual send pair — the ordinary request/response topology — must
    // not be flagged. A check that used the send graph would fire here.
    let fx = Fixture::new("sends");
    let alpha = sends_both_ways("alpha", "beta");
    let beta = sends_both_ways("beta", "alpha");
    fx.write("alpha.scxml", &alpha);
    fx.write("beta.scxml", &beta);
    let deploy = fx.write("deploy.yaml", &deploy_two("alpha", "beta"));

    let mut model = parse(&alpha, "alpha");
    let result = sce_build::compile_mesh_transport(&mut model, &deploy, Language::Cpp)
        .expect("compile_mesh_transport");

    assert!(
        result.invoke_wait_cycles.is_empty(),
        "mutual sends carry no wait and must not be reported; got {:?}",
        result.invoke_wait_cycles
    );
}
