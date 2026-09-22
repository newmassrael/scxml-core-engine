// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE-VERIFIES: mesh-9.5
//
// SCE_MESH §9.5 "Target selection — `src` vs `srcexpr`": a `srcexpr`
// resolves at `<invoke>` entry to "a static deploy.yaml binding whose
// key the expression resolves to", and the section says again that the
// construct "only allows the author to pick among already-declared
// bindings". The candidate set is therefore the machine's bindings, and
// the router the generator emits has to carry them.
//
// ⚠ WHAT THIS PINS, and why it is not implied by the fixtures. The
// dispatch table used to be built from static `src` sites alone, so a
// document whose ONLY mesh-rpc invoke is a srcexpr got no
// `<machine>_transport.h` at all: `performMeshInvoke` found no callback,
// returned false, and every resolution raised the §9.5 miss — including
// the names deploy.yaml had a binding for. Both srcexpr fixtures under
// `tests/mesh/` hid it by carrying a static-src invoke, one of them in a
// state no run can reach, so the runtime tests passed over a table that
// existed only by that workaround (measured 2026-09-22).

use std::fs;
use std::io::Write;
use std::path::PathBuf;

use sce_build::generator::Language;

struct Fixture {
    dir: PathBuf,
}

impl Fixture {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("sce_mesh_srcexpr_bindings_{tag}"));
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

/// A caller whose only mesh-rpc invoke names its target at run time.
fn srcexpr_only_caller(target: &str) -> String {
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" datamodel="ecmascript" name="caller" initial="idle">
  <state id="idle">
    <transition event="probe.go" target="calling"/>
  </state>
  <state id="calling">
    <invoke type="sce:mesh-rpc" srcexpr="'#{target}'">
      <param name="_mesh_event" expr="'service.request.ping'"/>
    </invoke>
    <transition event="error.execution" target="done"/>
  </state>
  <final id="done"/>
</scxml>
"##
    )
}

/// A receiver that answers the request the caller sends.
fn responder(name: &str) -> String {
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="{name}" initial="idle">
  <state id="idle">
    <transition event="service.request.ping" target="answering"/>
  </state>
  <state id="answering">
    <onentry>
      <send target="#caller" event="service.response.ping"/>
    </onentry>
    <transition event="done.ping" target="idle"/>
  </state>
</scxml>
"##
    )
}

fn deploy(bindings: &[&str]) -> String {
    let mut caller_bindings = String::new();
    let mut machines = String::new();
    for name in bindings {
        caller_bindings.push_str(&format!("          \"#{name}\": {{ transport: shm }}\n"));
        machines.push_str(&format!(
            "      {name}:\n        source: {name}.scxml\n        bindings:\n          \"#caller\": {{ transport: shm }}\n"
        ));
    }
    format!(
        r##"
version: "1.0"
topology:
  ecu1:
    machines:
      caller:
        source: caller.scxml
        bindings:
{caller_bindings}{machines}"##
    )
}

fn parse(src: &str, name: &str) -> sce_build::model::SCXMLModel {
    let mut parser = sce_build::parser::SCXMLParser::new();
    parser.parse_string(src, name).expect("parse")
}

/// Everything the deployment declares reaches the router, and the router
/// exists at all.
#[test]
fn a_srcexpr_only_document_routes_through_every_declared_binding() {
    let fx = Fixture::new("two_bindings");
    let caller = srcexpr_only_caller("alpha");
    fx.write("caller.scxml", &caller);
    fx.write("alpha.scxml", &responder("alpha"));
    fx.write("beta.scxml", &responder("beta"));
    let deploy_path = fx.write("deploy.yaml", &deploy(&["alpha", "beta"]));

    let mut model = parse(&caller, "caller");
    let result = sce_build::compile_mesh_transport(&mut model, &deploy_path, Language::Cpp)
        .expect("compile_mesh_transport");

    let transport = result
        .output
        .files
        .iter()
        .find(|(name, _)| name == "caller_transport.h")
        .map(|(_, code)| code.clone())
        .unwrap_or_else(|| {
            panic!(
                "no transport emitted for a srcexpr-only document; files: {:?}",
                result
                    .output
                    .files
                    .iter()
                    .map(|(n, _)| n)
                    .collect::<Vec<_>>()
            )
        });

    // Both bindings, not only the one this expression happens to name:
    // the expression is evaluated at run time and may name either, so a
    // table holding one of them decides at build time what §9.5 leaves
    // to the run.
    for name in ["alpha", "beta"] {
        assert!(
            transport.contains(name),
            "the router does not carry the declared binding '#{name}'"
        );
    }
}

/// The miss stays a miss: a name no binding carries reaches no dispatch
/// entry, which is what §9.5 answers with `error.execution` /
/// `INVOKE_SRC_NOT_FOUND` at run time.
#[test]
fn a_name_no_binding_carries_reaches_no_dispatch_entry() {
    let fx = Fixture::new("ghost");
    let caller = srcexpr_only_caller("ghost_target");
    fx.write("caller.scxml", &caller);
    fx.write("alpha.scxml", &responder("alpha"));
    let deploy_path = fx.write("deploy.yaml", &deploy(&["alpha"]));

    let mut model = parse(&caller, "caller");
    let result = sce_build::compile_mesh_transport(&mut model, &deploy_path, Language::Cpp)
        .expect("compile_mesh_transport");

    let transport = result
        .output
        .files
        .iter()
        .find(|(name, _)| name == "caller_transport.h")
        .map(|(_, code)| code.clone())
        .expect("transport emitted");

    assert!(
        !transport.contains("ghost_target"),
        "a name the deployment does not declare must not become a route"
    );
    assert!(
        transport.contains("alpha"),
        "the declared binding is still the table's one entry"
    );
}
