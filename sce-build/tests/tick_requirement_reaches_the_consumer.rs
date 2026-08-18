// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Whether a machine must be driven with `tick()` rather than `step()` is one
// fact with two publics, and until this round only one of them was served.
//
// A CLI caller reads `needs_event_scheduler` off the generate manifest. A
// `build.rs` caller reads nothing: `sce_build::compile_scxml` returns `()`, so
// the two independent downstream consumers of that route (pinion, sprag) had
// no route at all to the requirement — and a machine driven with `step` alone
// delivers no delayed event, raises no error and logs no warning.
//
// `SCXMLModel::needs_event_scheduler_driving` already owned the union and its
// doc already promised "the manifest's answer and the emitted code cannot
// disagree about which entry point the machine needs". Nothing measured that:
// the emitted code did not carry the answer at all. These tests are that
// measurement, on the analyzer's field and on the emitted text.

use std::fs;
use std::path::Path;

use tempfile::tempdir;

use sce_build::generator::Language;
use sce_build::{compile_scxml_lang_typed, find_template_dir_for};

/// A document whose own `<send>` carries a delay.
const OWN_DELAY: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="waiting" name="own_delay">
  <state id="waiting">
    <onentry><send event="timeout" delay="200ms"/></onentry>
    <transition event="timeout" target="done"/>
  </state>
  <final id="done"/>
</scxml>
"#;

/// A document that schedules nothing itself but invokes a child session. The
/// parent reaches that child's queue only through `tick_children`, which
/// `step` never calls — so the requirement is the parent's too.
const INVOKES_A_CHILD: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="parent" name="invokes_child">
  <state id="parent">
    <invoke type="http://www.w3.org/TR/scxml/" id="kid">
      <content>
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="inner">
          <state id="inner"><transition target="over"/></state>
          <final id="over"/>
        </scxml>
      </content>
    </invoke>
    <transition event="done.invoke.kid" target="finished"/>
  </state>
  <final id="finished"/>
</scxml>
"#;

/// Neither a delayed send nor a child session: a `step` loop drives it whole.
const NEITHER: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="only" name="neither">
  <state id="only"><transition event="go" target="done"/></state>
  <final id="done"/>
</scxml>
"#;

fn emitted_constant(source: &str, name: &str) -> bool {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join(format!("{name}.scxml"));
    fs::write(&path, source).expect("write fixture");
    let template_dir = find_template_dir_for(Language::Rust);
    let generated = compile_scxml_lang_typed(
        path.to_str().expect("utf-8 path"),
        &template_dir,
        Language::Rust,
    )
    .expect("the fixture compiles");

    parse_constant(&generated, &path)
}

fn parse_constant(generated: &sce_build::generator::GeneratedOutput, path: &Path) -> bool {
    let needle = "const NEEDS_EVENT_SCHEDULER: bool = ";
    let line = generated
        .files
        .iter()
        .flat_map(|(_, content)| content.lines())
        .find(|l| l.contains(needle))
        .unwrap_or_else(|| {
            panic!(
                "every generated policy declares the constant; {} did not",
                path.display()
            )
        });
    match line
        .split(needle)
        .nth(1)
        .map(|rest| rest.trim_end_matches(';'))
    {
        Some("true") => true,
        Some("false") => false,
        other => panic!("the constant is a bool literal, got {other:?}"),
    }
}

/// The emitted policy answers, and it answers per machine rather than always
/// the same way. A constant that is always `true` would satisfy the delayed
/// case while telling a consumer nothing.
#[test]
fn the_emitted_policy_declares_the_entry_point_each_machine_needs() {
    assert!(
        emitted_constant(OWN_DELAY, "own_delay"),
        "a `<send delay>` puts events somewhere only `tick` looks",
    );
    assert!(
        !emitted_constant(NEITHER, "neither"),
        "no delayed send and no child session — `step` drives it completely",
    );
}

/// The case the union exists for, and the one a naive reading of "needs event
/// scheduler" gets wrong: the parent owns no scheduler entries at all, and
/// still cannot be driven with `step`.
#[test]
fn a_parent_whose_child_needs_ticking_needs_ticking_too() {
    assert!(
        emitted_constant(INVOKES_A_CHILD, "invokes_child"),
        "the child's queue is reachable only through the parent's `tick_children`",
    );
}

/// The field the templates read and the method the manifest reads are the same
/// verdict. They are separate code paths — the analyzer writes the field, the
/// CLI calls the method — and this is what keeps that from becoming two
/// answers to one question.
#[test]
fn the_field_the_templates_read_agrees_with_the_method_the_manifest_reads() {
    for (source, name) in [
        (OWN_DELAY, "own_delay"),
        (INVOKES_A_CHILD, "invokes_child"),
        (NEITHER, "neither"),
    ] {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join(format!("{name}.scxml"));
        fs::write(&path, source).expect("write fixture");

        let mut parser = sce_build::parser::SCXMLParser::new();
        let mut model = parser
            .parse_file(path.to_str().expect("utf-8 path"))
            .expect("the fixture parses");
        sce_build::analyzer::analyze(&mut model, path.to_str().expect("utf-8 path"));

        assert_eq!(
            model.needs_tick_driving,
            model.needs_event_scheduler_driving(),
            "{name}: the analyzer's field and the model's method are one verdict",
        );
        assert_eq!(
            model.needs_tick_driving,
            emitted_constant(source, name),
            "{name}: what the templates emit is what the analyzer decided",
        );
    }
}
