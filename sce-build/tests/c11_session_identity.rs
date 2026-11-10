// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.10 on the C11 backend: `_sessionid` identifies the session.
//
// The clause binds `_sessionid` to "the system-generated id for the
// current SCXML session", and says it is an NMTOKEN. The C11 profile used
// to bake `<document>_session` at codegen time, and the comment above that
// literal restated the requirement as "bound, and unchanging through the
// session" — true of the clause, and missing the two parts that constrain
// the value. Every instance of one machine therefore read the same id, and
// since C.1.1 derives the published `_ioprocessors` location from it, two
// live sessions of one document also published one address: a `<send>`
// addressed to either was addressed to both.
//
// The decisive assertion is behavioural and it is the one no channel made:
// two sessions that are alive at the same time have different ids. The
// rest of the clause is checked alongside it, because a value that is
// unique and unusable satisfies nothing — it has to stay put for the life
// of the session, it has to be an NMTOKEN, and the address has to name the
// same session the id does.
//
// The assertions run against the machine's own accessors rather than the
// datamodel, so the fixture needs no script engine and the test needs no
// Lua on the link line. What that leaves uncovered is the Lua binding
// itself, which the `w3c-c11` lane exercises through fixtures that read
// `_sessionid` from documents.

use std::path::{Path, PathBuf};
use std::process::Command;

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// Two states so the driver can step the machine and read the id again:
/// "same value through the session" is only observable across a step.
const FIXTURE: &str = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       initial="s1" datamodel="ecmascript">
  <state id="s1">
    <transition event="go" target="s2"/>
  </state>
  <final id="s2"/>
</scxml>
"#;

/// Prints `<id>|<location>` per instance, before and after a step.
const DRIVER: &str = r#"#include <stdio.h>
#include "sessions_sm.h"

int main(void) {
    sessions_t a;
    sessions_t b;
    sessions_init(&a);
    sessions_init(&b);
    printf("a0=%s|%s\n", sessions_session_id(&a), sessions_scxml_location(&a));
    printf("b0=%s|%s\n", sessions_session_id(&b), sessions_scxml_location(&b));
    sessions_step(&a);
    printf("a1=%s|%s\n", sessions_session_id(&a), sessions_scxml_location(&a));
    return 0;
}
"#;

/// `NMTOKEN` is `(NameChar)+`. The ids this profile issues stay inside the
/// ASCII subset, so that is what is checked; a wider alphabet would need
/// the XML production rather than this predicate.
fn is_ascii_nmtoken(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | ':'))
}

fn field<'a>(lines: &'a [&'a str], key: &str) -> (&'a str, &'a str) {
    let line = lines
        .iter()
        .find(|l| l.starts_with(&format!("{key}=")))
        .unwrap_or_else(|| panic!("driver printed no `{key}` line: {lines:?}"));
    let rest = &line[key.len() + 1..];
    let (id, location) = rest
        .split_once('|')
        .unwrap_or_else(|| panic!("malformed `{key}` line: {line}"));
    (id, location)
}

/// The lines `driver` prints, compiled under `-Werror` with the machine of
/// [`FIXTURE`] generated beside it in a directory of its own (`tag`); `None`
/// when no C compiler is located.
fn run_driver(tag: &str, driver: &str) -> Option<String> {
    let Some(cc) = sce_build::toolchain::locate_any(&["gcc", "cc"]) else {
        eprintln!("no C compiler located; skipping");
        return None;
    };

    let out_dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("c11-session-identity-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out_dir);
    std::fs::create_dir_all(&out_dir).expect("create out dir");
    let scxml = out_dir.join("sessions.scxml");
    std::fs::write(&scxml, FIXTURE).expect("write fixture");
    std::fs::write(out_dir.join("driver.c"), driver).expect("write driver");

    let gen = Command::new(sce_codegen_bin())
        .arg("generate")
        .arg(&scxml)
        .args(["-l", "c11", "-o"])
        .arg(&out_dir)
        .output()
        .expect("invoke sce-codegen");
    assert!(
        gen.status.success(),
        "generation failed: {}",
        String::from_utf8_lossy(&gen.stderr),
    );

    let build = Command::new(&cc)
        .args(["-std=c11", "-Wall", "-Wextra", "-Werror"])
        .arg("-I")
        .arg(repo_root().join("backends/c/runtime/include"))
        .arg("-I")
        .arg(&out_dir)
        .arg("-o")
        .arg(out_dir.join("driver"))
        .arg(out_dir.join("driver.c"))
        .arg(out_dir.join("sessions_sm.c"))
        .output()
        .expect("invoke C compiler");
    assert!(
        build.status.success(),
        "driver must compile under -Werror:\n{}",
        String::from_utf8_lossy(&build.stderr),
    );

    let run = Command::new(out_dir.join("driver"))
        .output()
        .expect("run driver");
    assert!(run.status.success(), "driver exited non-zero");
    let stdout = String::from_utf8_lossy(&run.stdout).into_owned();
    let _ = std::fs::remove_dir_all(&out_dir);
    Some(stdout)
}

#[test]
fn two_live_sessions_of_one_document_have_different_ids() {
    let Some(stdout) = run_driver("drawn", DRIVER) else {
        return;
    };

    let lines: Vec<&str> = stdout.lines().collect();
    let (a0, a0_loc) = field(&lines, "a0");
    let (b0, b0_loc) = field(&lines, "b0");
    let (a1, a1_loc) = field(&lines, "a1");

    // The claim the old binding could not make: these two sessions are
    // alive at the same time, so they are two sessions.
    assert_ne!(
        a0, b0,
        "two live sessions of one document share the id `{a0}` — \
         `_sessionid` is naming the document, not the session",
    );
    assert_ne!(
        a0_loc, b0_loc,
        "two live sessions publish one address `{a0_loc}`, so a `<send>` \
         addressed to either is addressed to both",
    );

    // W3C SCXML 5.10: the same value through the session.
    assert_eq!(a1, a0, "the id changed across a step");
    assert_eq!(
        a1_loc, a0_loc,
        "the published address changed across a step"
    );

    // W3C SCXML C.1.1: the address names the session the id does, or a
    // peer told to answer at that address answers a different session.
    for (id, location) in [(a0, a0_loc), (b0, b0_loc)] {
        assert_eq!(
            location,
            format!("sce://scxml/{id}"),
            "the published location does not spell this session's id",
        );
    }

    // W3C SCXML 5.10: "(This is of type NMTOKEN.)" — the half of the
    // clause no channel checked.
    for id in [a0, b0] {
        assert!(
            is_ascii_nmtoken(id),
            "`_sessionid` `{id}` is not an NMTOKEN"
        );
    }
}

/// Starts a machine under an id its host gives, and under ids it cannot take,
/// and prints what each start answered and what the machine then holds. A
/// refused start must leave the storage as it was, so its machine is filled
/// with a pattern first and read back byte by byte.
const GIVEN_DRIVER: &str = r#"#include <stdio.h>
#include <string.h>
#include "sessions_sm.h"

static sessions_t given;
static sessions_t refused;
static sessions_t drawn;

/* Whether every byte of `sm` is still `pattern`. */
static int untouched(const sessions_t *sm, unsigned char pattern) {
    const unsigned char *bytes = (const unsigned char *)sm;
    for (size_t i = 0u; i < sizeof(*sm); ++i) {
        if (bytes[i] != pattern) {
            return 0;
        }
    }
    return 1;
}

int main(void) {
    char too_long[SCE_MAX_SESSION_ID_LEN + 1];
    memset(too_long, 'a', sizeof(too_long) - 1u);
    too_long[sizeof(too_long) - 1u] = '\0';

    const sce_start_refusal_t taken = sessions_init_with_session_id(&given, "host-given.1");
    printf("given=%d|%s|%s\n", taken == SCE_START_REFUSAL_NONE, sessions_session_id(&given),
           sessions_scxml_location(&given));
    sessions_step(&given);
    printf("given_after_step=%s\n", sessions_session_id(&given));

    memset(&refused, 0x5A, sizeof(refused));
    printf("too_long=%d|%d\n", sessions_init_with_session_id(&refused, too_long) == SCE_START_REFUSAL_SESSION_ID_TOO_LONG,
           untouched(&refused, 0x5A));
    printf("not_nmtoken=%d|%d\n", sessions_init_with_session_id(&refused, "a b") == SCE_START_REFUSAL_SESSION_ID_NOT_NMTOKEN,
           untouched(&refused, 0x5A));
    printf("empty=%d|%d\n", sessions_init_with_session_id(&refused, "") == SCE_START_REFUSAL_SESSION_ID_EMPTY,
           untouched(&refused, 0x5A));
    printf("none=%d|%d\n", sessions_init_with_session_id(&refused, NULL) == SCE_START_REFUSAL_SESSION_ID_EMPTY,
           untouched(&refused, 0x5A));

    sessions_init(&drawn);
    printf("drawn=%s\n", sessions_session_id(&drawn));
    return 0;
}
"#;

/// The line `key=` of `lines`, what follows the `=`.
fn value<'a>(lines: &'a [&'a str], key: &str) -> &'a str {
    let line = lines
        .iter()
        .find(|l| l.starts_with(&format!("{key}=")))
        .unwrap_or_else(|| panic!("driver printed no `{key}` line: {lines:?}"));
    &line[key.len() + 1..]
}

/// docs/adr/0045 ("Questions the owner answered" 2): a root takes its session
/// id from its host where the host gives one, as on every backend, and draws
/// its own only where it is given none. The id given is the session's for its
/// whole life and the location it publishes spells it; one the machine cannot
/// hold whole, or that is no Nmtoken, is refused with its reason and the
/// storage left as it was, never cut and never replaced by a drawn one.
#[test]
fn a_root_takes_the_session_id_its_host_gives() {
    let Some(stdout) = run_driver("given", GIVEN_DRIVER) else {
        return;
    };
    let lines: Vec<&str> = stdout.lines().collect();

    assert_eq!(
        value(&lines, "given"),
        "1|host-given.1|sce://scxml/host-given.1",
        "the given id is the session's, and its location spells it"
    );
    assert_eq!(
        value(&lines, "given_after_step"),
        "host-given.1",
        "the given id is kept through the session"
    );
    for key in ["too_long", "not_nmtoken", "empty", "none"] {
        assert_eq!(
            value(&lines, key),
            "1|1",
            "`{key}`: refused with its own reason, the storage untouched"
        );
    }
    let drawn = value(&lines, "drawn");
    assert!(
        drawn.starts_with("sessions_session_"),
        "given no id, the machine draws its own, not `{drawn}`"
    );
}
