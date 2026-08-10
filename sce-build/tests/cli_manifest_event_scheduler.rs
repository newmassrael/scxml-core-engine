// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// The manifest's `needs_event_scheduler` field — which driving entry
// point the emitted machine requires of its host.
//
// `sce_rust_runtime::Engine` exposes two: `step()` runs a macrostep,
// `tick()` polls the delayed-send scheduler and ticks invoked children
// before running that macrostep. A machine that schedules a `<send
// delay>` or drives a child session and is driven by `step()` alone
// loses those events silently — no error, no diagnostic, just events
// that never arrive.
//
// The generator knows which entry point a document needs: the analyzer
// sets `SCXMLModel::needs_event_scheduler` from `<send delay>` /
// `<cancel>`, and the emitted policy's `HAS_CHILD_TICK` is gated on
// session-bearing invokes. `needs_script_engine` already publishes the
// sibling question — what the host has to supply — so a host reading
// the manifest can answer "do I need a script engine?" and, until this
// field existed, could not answer "which entry point drives this?"
// without reading the runtime's source.

use std::path::{Path, PathBuf};
use std::process::Command;

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn write(dir: &Path, name: &str, body: &str) -> PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, body).expect("write fixture");
    p
}

/// Run `check` (which writes nothing) and return the parsed manifest.
fn check_manifest(doc: &Path) -> serde_json::Value {
    let out = Command::new(sce_codegen_bin())
        .args(["check", doc.to_str().expect("utf-8 path"), "-l", "rust"])
        .output()
        .expect("spawn sce-codegen");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let line = stdout
        .lines()
        .find(|l| l.trim_start().starts_with('{'))
        .unwrap_or_else(|| {
            panic!(
                "no manifest line on stdout.\nstdout: {stdout}\nstderr: {}",
                String::from_utf8_lossy(&out.stderr)
            )
        });
    serde_json::from_str(line).expect("manifest is one JSON object")
}

fn needs_event_scheduler(doc: &Path) -> bool {
    let m = check_manifest(doc);
    m.get("needs_event_scheduler")
        .unwrap_or_else(|| {
            panic!(
                "manifest carries no `needs_event_scheduler`; a host cannot tell \
                 whether to drive this machine with tick() or step(). manifest: {m}"
            )
        })
        .as_bool()
        .expect("needs_event_scheduler is a bool")
}

const PLAIN: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" name="plain" version="1.0"
       initial="a" datamodel="ecmascript">
  <state id="a"><transition event="go" target="b"/></state>
  <final id="b"/>
</scxml>"##;

const DELAYED_SEND: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" name="delayed" version="1.0"
       initial="a" datamodel="ecmascript">
  <state id="a">
    <onentry><send event="tick" delay="200ms"/></onentry>
    <transition event="tick" target="b"/>
  </state>
  <final id="b"/>
</scxml>"##;

const CANCEL: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" name="canceller" version="1.0"
       initial="a" datamodel="ecmascript">
  <state id="a">
    <transition event="stop" target="b"><cancel sendid="pending"/></transition>
  </state>
  <final id="b"/>
</scxml>"##;

/// A parent whose only claim on `tick()` is the child it drives — it
/// schedules nothing itself. The child's delayed send reaches the child's
/// queue only through the parent's `tick_children`, which `step()` never
/// calls.
const CHILD: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" name="child" version="1.0"
       initial="c1" datamodel="ecmascript">
  <state id="c1">
    <onentry><send event="ping" delay="100ms"/></onentry>
    <transition event="ping" target="c2"/>
  </state>
  <final id="c2"/>
</scxml>"##;

const PARENT_OF_CHILD: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" name="parent" version="1.0"
       initial="a" datamodel="ecmascript">
  <state id="a">
    <invoke type="http://www.w3.org/TR/scxml/" src="child.scxml"/>
    <transition event="done.invoke" target="b"/>
  </state>
  <final id="b"/>
</scxml>"##;

/// A document that schedules nothing and drives nothing needs neither
/// half of `tick()`, and must say so.
///
/// The negative arm is what makes the positives mean something: a field
/// hard-coded to `true` would satisfy every other test in this file.
#[test]
fn a_machine_that_schedules_nothing_reports_false() {
    let dir = tempfile::tempdir().expect("tempdir");
    let doc = write(dir.path(), "plain.scxml", PLAIN);
    assert!(
        !needs_event_scheduler(&doc),
        "a machine with no delayed send, no cancel and no child session \
         is fully driven by step()"
    );
}

/// Each construct that puts work on the scheduler is reported.
///
/// Driven per construct rather than through one representative: the
/// analyzer sets the flag from two separate arms (`<send delay>` and
/// `<cancel>`), and a union that dropped either would still pass a
/// single-construct test.
#[test]
fn every_scheduling_construct_is_reported() {
    for (label, body) in [("send-delay", DELAYED_SEND), ("cancel", CANCEL)] {
        let dir = tempfile::tempdir().expect("tempdir");
        let doc = write(dir.path(), "doc.scxml", body);
        assert!(
            needs_event_scheduler(&doc),
            "[{label}] puts an entry on the delayed-send scheduler, which only \
             tick() drains — a host driving this with step() loses the event \
             with no diagnostic"
        );
    }
}

/// The child axis, which the document's own text does not show.
///
/// `tick()` is two mechanisms: draining the scheduler *and* ticking
/// invoked children. A parent that schedules nothing still needs it,
/// because `tick_children` runs from `tick()` alone. Reporting only the
/// document's own `<send delay>` would leave exactly this shape silently
/// mis-driven, which is the failure the field exists to prevent.
#[test]
fn a_parent_driving_a_child_session_is_reported() {
    let dir = tempfile::tempdir().expect("tempdir");
    write(dir.path(), "child.scxml", CHILD);
    let parent = write(dir.path(), "parent.scxml", PARENT_OF_CHILD);
    assert!(
        needs_event_scheduler(&parent),
        "the parent schedules nothing of its own, but its child's events \
         reach the child only through tick_children(), which step() never calls"
    );
}

/// The field is always present, like `needs_script_engine`.
///
/// A consumer must be able to read `false` as an answer rather than as
/// an absent field it has to guess about; omitting it when false would
/// make "no scheduler needed" and "generator too old to know"
/// indistinguishable on the wire.
#[test]
fn the_field_is_always_present_like_its_sibling() {
    let dir = tempfile::tempdir().expect("tempdir");
    for (name, body) in [("plain.scxml", PLAIN), ("delayed.scxml", DELAYED_SEND)] {
        let doc = write(dir.path(), name, body);
        let m = check_manifest(&doc);
        assert!(
            m.get("needs_script_engine").is_some(),
            "[{name}] sibling field vanished — the mirror this one follows is gone"
        );
        assert!(
            m.get("needs_event_scheduler").is_some(),
            "[{name}] needs_event_scheduler must be present whatever its value, \
             matching needs_script_engine"
        );
    }
}

/// Run `check` over a document *set* and return the parsed manifest.
fn check_set_manifest(docs: &[&Path]) -> serde_json::Value {
    let mut args: Vec<String> = vec!["check".to_string()];
    for d in docs {
        args.push("--scxml".to_string());
        args.push(d.to_str().expect("utf-8 path").to_string());
    }
    let out = Command::new(sce_codegen_bin())
        .args(&args)
        .output()
        .expect("spawn sce-codegen");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let line = stdout
        .lines()
        .find(|l| l.trim_start().starts_with('{'))
        .unwrap_or_else(|| {
            panic!(
                "no manifest line.\nstdout: {stdout}\nstderr: {}",
                String::from_utf8_lossy(&out.stderr)
            )
        });
    serde_json::from_str(line).expect("manifest is one JSON object")
}

/// On the document-set route the field describes the **set** — the union
/// over its inputs — because that is the form the question takes for a
/// build deciding how to drive the machines it just generated.
///
/// Asserted with a mixed set rather than an all-scheduling one: a route
/// that reported only the first document, or only the last, would agree
/// with a uniform set and disagree here.
#[test]
fn the_document_set_route_reports_the_union() {
    let dir = tempfile::tempdir().expect("tempdir");
    let plain = write(dir.path(), "plain.scxml", PLAIN);
    let delayed = write(dir.path(), "delayed.scxml", DELAYED_SEND);

    let alone = check_set_manifest(&[plain.as_path()]);
    assert_eq!(
        alone["needs_event_scheduler"], false,
        "a set of one non-scheduling document needs no scheduler"
    );

    for (label, set) in [
        ("scheduling-last", vec![plain.as_path(), delayed.as_path()]),
        ("scheduling-first", vec![delayed.as_path(), plain.as_path()]),
    ] {
        let m = check_set_manifest(&set);
        assert_eq!(
            m["needs_event_scheduler"], true,
            "[{label}] one scheduling document in the set makes the set need tick()"
        );
    }
}
