// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Whether a machine must be driven with `tick()` rather than `step()` is one
// fact with two publics, and for a long time only one of them was served.
//
// A CLI caller reads `needs_event_scheduler` off the generate manifest. A
// `build.rs` caller reads nothing: `sce_build::compile_scxml` returns `()`, so
// the two independent downstream consumers of that route (pinion, sprag) had no
// route at all to the requirement — and a machine driven with `step` alone
// delivers no delayed event, raises no error and logs no warning.
//
// The silence was measured, not argued. Driving the same delayed-send document
// with the wrong entry point leaves it in its initial state after two seconds
// in C++, Go and Python alike; the right one finishes it.
//
// `SCXMLModel::needs_event_scheduler_driving` already owned the union and its
// doc already promised "the manifest's answer and the emitted code cannot
// disagree about which entry point the machine needs". Nothing measured that:
// the emitted code did not carry the answer at all.
//
// These tests are that measurement, and they run over `Language::ALL` so a
// seventh backend cannot join by being skipped.

use std::fs;
use std::path::Path;

use tempfile::tempdir;

use sce_build::generator::{GeneratedOutput, Language};
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

fn generate(source: &str, name: &str, lang: Language) -> GeneratedOutput {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join(format!("{name}.scxml"));
    fs::write(&path, source).expect("write fixture");
    let template_dir = find_template_dir_for(lang);
    compile_scxml_lang_typed(path.to_str().expect("utf-8 path"), &template_dir, lang)
        .expect("the fixture compiles")
}

fn all_text(generated: &GeneratedOutput) -> String {
    generated
        .files
        .iter()
        .map(|(_, content)| content.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

/// The literal that follows a one-line declaration.
fn literal_after(text: &str, needle: &str, path: &Path) -> String {
    let line = text
        .lines()
        .find(|l| l.contains(needle))
        .unwrap_or_else(|| {
            panic!(
                "{}: no generated line carries {needle:?} — this backend's host has to guess",
                path.display()
            )
        });
    line.split(needle)
        .nth(1)
        .map(|r| r.trim().trim_end_matches(';').trim().to_string())
        .unwrap_or_default()
}

/// The literal returned by the first `return` after a declaration line. Go and
/// Python answer with a method rather than a constant.
fn literal_returned_after(text: &str, decl: &str, path: &Path) -> String {
    let mut lines = text.lines().skip_while(|l| !l.contains(decl));
    lines.next().unwrap_or_else(|| {
        panic!(
            "{}: no generated line declares {decl:?} — this backend's host has to guess",
            path.display()
        )
    });
    for l in lines.take(12) {
        let t = l.trim();
        if let Some(rest) = t.strip_prefix("return ") {
            return rest.trim().to_string();
        }
    }
    panic!("{}: {decl:?} declares nothing it returns", path.display());
}

/// What this backend's generated artifact says about needing a tick-driven
/// host, read the way that backend spells it.
///
/// Driven from `Language::ALL` by the callers, so a seventh backend lands here
/// as a compile error rather than as a silent gap. C11 is the one that answers
/// structurally instead of with a flag: it emits the `_tick` entry point only
/// for a machine that needs one, which refuses a wrong driving loop at link
/// time rather than merely describing the right one.
fn declares_scheduler_requirement(lang: Language, source: &str, name: &str) -> bool {
    let generated = generate(source, name, lang);
    let text = all_text(&generated);
    let path = Path::new(name);
    let literal = match lang {
        Language::Rust => literal_after(&text, "const NEEDS_EVENT_SCHEDULER: bool = ", path),
        Language::Cpp => literal_after(
            &text,
            "static constexpr bool NEEDS_EVENT_SCHEDULER = ",
            path,
        ),
        Language::Kotlin => {
            literal_after(&text, "override val needsEventScheduler: Boolean = ", path)
        }
        Language::Go => literal_returned_after(&text, ") NeedsEventScheduler() bool {", path),
        Language::Python => literal_returned_after(&text, "def needs_event_scheduler(", path),
        Language::C11 => {
            // Structural: the entry point exists exactly when it is needed.
            let tick_decl = format!("void {name}_tick(");
            return text.contains(&tick_decl);
        }
    };
    match literal.as_str() {
        "true" | "True" => true,
        "false" | "False" => false,
        other => panic!("{name}/{lang:?}: the declaration is a boolean literal, got {other:?}"),
    }
}

/// Every backend answers, and answers per machine rather than always the same
/// way. A declaration that is always `true` would satisfy the delayed case
/// while telling a consumer nothing — which is why both directions are asked of
/// every language rather than of one.
#[test]
fn every_backend_declares_which_entry_point_the_machine_needs() {
    for &lang in Language::ALL {
        assert!(
            declares_scheduler_requirement(lang, OWN_DELAY, "own_delay"),
            "{lang:?}: a `<send delay>` puts events somewhere only tick-driving reaches",
        );
        assert!(
            !declares_scheduler_requirement(lang, NEITHER, "neither"),
            "{lang:?}: no delayed send and no child session — a plain macrostep loop drives it",
        );
    }
}

/// Backends whose runtime reaches an invoked child only from the tick entry
/// point, so a parent that schedules nothing itself still has to be ticked.
///
/// Measured, one runtime at a time: Rust `tick`, C++ `tick()`, Go `Tick`,
/// Kotlin `tick()` and Python `advance_time` each drive children, and their
/// plain-macrostep sibling does not. C11 is deliberately absent — see
/// [`c11_drives_an_invoked_child_from_the_macrostep_itself`].
const CHILD_NEEDS_THE_PARENT_TICKED: &[Language] = &[
    Language::Rust,
    Language::Cpp,
    Language::Kotlin,
    Language::Go,
    Language::Python,
];

/// The case the union exists for, and the one a naive reading of "needs event
/// scheduler" gets wrong: the parent owns no scheduler entries at all, and
/// still cannot be driven with a plain macrostep loop.
#[test]
fn a_parent_whose_child_needs_ticking_needs_ticking_too() {
    for &lang in CHILD_NEEDS_THE_PARENT_TICKED {
        assert!(
            declares_scheduler_requirement(lang, INVOKES_A_CHILD, "invokes_child"),
            "{lang:?}: the child's queue is reachable only through the parent's tick",
        );
    }
}

/// C11 answers this question differently, and correctly.
///
/// Its generated parent drives the child from inside its own macrostep — and
/// picks `_step` or `_tick` for that child according to what the child needs —
/// so the parent itself needs a `_tick` only when the parent's own document
/// carries a delayed `<send>`. Emitting one anyway would offer a host an entry
/// point with nothing to do.
///
/// This is why the constant is not simply `needs_tick_driving` in every
/// backend: the union encodes where the runtime reaches children from, and C11
/// reaches them from somewhere else. Asserted rather than left as a comment,
/// because the difference is invisible in the emitted flag and would otherwise
/// be re-litigated as a bug.
#[test]
fn c11_drives_an_invoked_child_from_the_macrostep_itself() {
    let parent_only = generate(INVOKES_A_CHILD, "invokes_child", Language::C11);
    let text = all_text(&parent_only);

    assert!(
        !text.contains("void invokes_child_tick("),
        "the parent schedules nothing of its own, so a `_tick` on it would be an empty entry point",
    );
    assert!(
        text.contains("_step(&sm->child_kid)"),
        "the child has no delayed send, so the parent drives it with the child's `_step`",
    );
}

/// The field the templates read and the method the manifest reads are the same
/// verdict. They are separate code paths — the analyzer writes the field, the
/// CLI calls the method — and this is what keeps that from becoming two answers
/// to one question.
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
        for &lang in CHILD_NEEDS_THE_PARENT_TICKED {
            assert_eq!(
                model.needs_tick_driving,
                declares_scheduler_requirement(lang, source, name),
                "{name}/{lang:?}: what the backend emits is what the analyzer decided",
            );
        }
        // C11 emits the entry point for its own scheduler only, because it
        // reaches an invoked child from the macrostep rather than from a tick.
        assert_eq!(
            model.needs_event_scheduler.unwrap_or(false),
            declares_scheduler_requirement(Language::C11, source, name),
            "{name}/C11: the `_tick` entry point exists exactly when this document schedules",
        );
    }
}
