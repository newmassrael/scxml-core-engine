// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Two monitors that raise the same event, and the emitted module could not be
// loaded.
//
// An observer's event list is the DOMAIN's membership, not one entry per
// monitor. Watching one quantity against two different threshold pairs and
// calling both "entered" is the ordinary shape: a gauge whose red zone starts
// at 4000 on one scale and 5000 on another has two monitors and one event.
//
// `generate_observer` built the list by pushing `on_enter` (and `on_leave`)
// once per monitor with no de-duplication, so the enum carried a repeated
// variant:
//
//     class ForgeDomainTag(Enum):
//         RED_ZONE_ENTERED = "RED_ZONE_ENTERED"
//         RED_ZONE_LEFT    = "RED_ZONE_LEFT"
//         RED_ZONE_ENTERED = "RED_ZONE_ENTERED"   <- second monitor
//         RED_ZONE_LEFT    = "RED_ZONE_LEFT"
//
// ⚠ Python refuses that at import (`TypeError: 'RED_ZONE_ENTERED' already
// defined`), so the artifact was unusable rather than subtly wrong — which is
// the good case. The reason it survived is that nothing had written an
// observer with two monitors sharing an event: the shape is ordinary in
// specifications and absent from this tree's fixtures.
//
// Measured 2026-09-19 while checking, for the first time, whether a converted
// document had been given the right KIND. It had not; writing the same logic
// as the observer it actually is produced this.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

static SCRATCH: AtomicU64 = AtomicU64::new(0);

fn scratch(label: &str) -> PathBuf {
    let id = SCRATCH.fetch_add(1, Ordering::SeqCst);
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("{label}-{}-{id}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

const TWO_MONITORS_ONE_EVENT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="observer" name="red_zone" version="1.0">
  <datamodel>
    <data id="level" sce:type="float64" sce:direction="in"/>
    <data id="lowScale" sce:type="bool" sce:direction="in"/>
    <data id="onLowScale" sce:monitor="threshold"
          sce:enter="lowScale &amp;&amp; (level &gt;= 4000)"
          sce:leave="lowScale &amp;&amp; (level &lt;= 3500)"
          sce:on-enter="ZoneEntered" sce:on-leave="ZoneLeft"/>
    <data id="onHighScale" sce:monitor="threshold"
          sce:enter="!lowScale &amp;&amp; (level &gt;= 5000)"
          sce:leave="!lowScale &amp;&amp; (level &lt;= 4500)"
          sce:on-enter="ZoneEntered" sce:on-leave="ZoneLeft"/>
  </datamodel>
</scxml>
"#;

fn emit(language: &str, label: &str) -> String {
    let dir = scratch(label);
    let path = dir.join("red_zone.scxml");
    std::fs::write(&path, TWO_MONITORS_ONE_EVENT).expect("write document");
    let run = Command::new(sce_codegen_bin())
        .args([
            "generate",
            path.to_str().unwrap(),
            "-l",
            language,
            "-o",
            dir.to_str().unwrap(),
        ])
        .current_dir(repo_root())
        .output()
        .expect("spawn sce-codegen");
    assert!(
        run.status.success(),
        "{language}: generation refused a well-formed observer: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    let mut emitted = String::new();
    for entry in std::fs::read_dir(&dir).expect("read scratch dir") {
        let file = entry.expect("dir entry").path();
        if file.extension().is_some_and(|e| e != "scxml") {
            emitted.push_str(&std::fs::read_to_string(&file).unwrap_or_default());
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
    emitted
}

/// How many lines DECLARE this member, as opposed to referring to it.
///
/// Two narrower readings were tried and both broke. Matching a per-language
/// punctuation broke on the member a language leaves without a trailing
/// comma; slicing a brace-delimited block broke on Python, whose enum has no
/// braces and whose first mention of the tag is an import. A declaration is
/// simply a line that BEGINS with the member, and every reference to it
/// begins with something else.
fn declarations_of(emitted: &str, member: &str) -> usize {
    emitted
        .lines()
        .filter(|line| line.trim_start().starts_with(member))
        .count()
}

/// The shared event appears once in the domain, on every backend.
#[test]
fn a_shared_event_is_declared_once() {
    for language in ["python", "rust", "kotlin"] {
        let emitted = emit(language, &format!("obs-share-{language}"));
        for member in ["ZONE_ENTERED", "ZONE_LEFT"] {
            let seen = declarations_of(&emitted, member);
            assert_eq!(
                1, seen,
                "{language}: the domain declares {member} on {seen} lines; two \
                 monitors raising one event are one member, not two"
            );
        }
    }
}

/// And both monitors still raise it — the de-duplication is of the DOMAIN, not
/// of the monitors. Without this, emitting an empty domain would pass the case
/// above.
#[test]
fn both_monitors_still_raise_the_shared_event() {
    let emitted = emit("python", "obs-share-raises");
    assert!(
        emitted.matches("ForgeDomainTag.ZONE_ENTERED").count() >= 2,
        "each monitor pushes the shared event, so it is raised from two sites"
    );
    assert!(
        emitted.contains("_onLowScale") && emitted.contains("_onHighScale"),
        "both monitors keep their own threshold state"
    );
}
