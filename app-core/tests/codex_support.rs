// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Which Codex the application runs, and what it refuses to run beside.
//!
//! Codex has no switch that turns its built-in tools off, and it gains tools from one version to
//! the next. A specification is text a person may have pasted from anywhere, and a client that
//! has read it must not be one that can be talked into reading the person's files or reaching
//! the network. So a version is run only when it was verified, by a person, against a
//! specification written to attack it, and what was verified is recorded: the operating system,
//! the version, and which instructions and which tools were switched off at the time. A
//! version that is not on the list is not run, whatever else is true of it, and a feature a
//! version has switched on that nobody switched off or reviewed is not run beside either, so
//! that a tool added by an update is a refusal and not an opening.
//!
//! The list is data that ships with the application and is empty until somebody has done that
//! verification.

use sce_app_core::codex_support::{enabled_features, Support, Unsupported};

/// What `codex features list` printed, in the shape the 0.159.0 install gives it.
const LISTED: &str = "\
apps                                     stable             true
browser_use                              stable             true
code_mode                                under development  false
fast_mode                                stable             true
goals                                    stable             true
shell_tool                               stable             true
unified_exec                             stable             true
view_image                               stable             true
web_search_cached                        deprecated         false
apply_patch_freeform                     removed            false
";

fn support(verified: &str, disabled: &[&str], reviewed: &[&str]) -> Support {
    Support::from_json(
        &serde_json::json!({
            "verified": serde_json::from_str::<serde_json::Value>(verified).unwrap(),
            "disabled_features": disabled,
            "reviewed_features": reviewed,
        })
        .to_string(),
    )
    .unwrap()
}

const ONE: &str = r#"[{"os":"linux","version":"0.159.0","instructions":"codex/abc123"}]"#;

#[test]
fn the_features_a_version_has_switched_on_are_the_ones_that_say_true() {
    let on = enabled_features(LISTED);

    assert_eq!(
        on,
        vec![
            "apps",
            "browser_use",
            "fast_mode",
            "goals",
            "shell_tool",
            "unified_exec",
            "view_image"
        ]
    );
}

#[test]
fn a_line_that_is_not_a_feature_is_not_one() {
    assert_eq!(enabled_features(""), Vec::<String>::new());
    assert_eq!(
        enabled_features("a warning line\n\n  \n"),
        Vec::<String>::new()
    );
    // A stage of two words is still a stage.
    assert_eq!(enabled_features("x  under development  true\n"), vec!["x"]);
}

#[test]
fn nothing_is_verified_in_a_build_nobody_has_verified_a_version_for() {
    let shipped = support("[]", &["shell_tool"], &[]);

    assert!(shipped.verified().is_empty());
    let refused = shipped
        .check("linux", "0.159.0", "codex/abc123", &[])
        .unwrap_err();
    assert!(
        matches!(refused, Unsupported::Version { .. }),
        "{refused:?}"
    );
}

#[test]
fn shipped_verifications_name_the_current_execution_contract() {
    let shipped = Support::shipped();
    let instructions = sce_app_core::codex::instructions_of(&shipped);
    assert!(shipped
        .verified()
        .iter()
        .any(|v| v.os == "linux" && v.version == "0.159.0"));
    for entry in shipped.verified() {
        assert_eq!(
            entry.instructions, instructions,
            "execution contract changed: rerun codex_live before updating support"
        );
        assert!(shipped
            .check(&entry.os, &entry.version, &instructions, &[])
            .is_ok());
    }
    assert!(shipped
        .check("windows", "0.159.0", &instructions, &[])
        .is_err());
    assert!(shipped
        .check("linux", "0.159.1", &instructions, &[])
        .is_err());
}

#[test]
fn a_build_that_ships_a_list_switches_the_shell_off_in_it() {
    // What is switched off is part of what a person verifies, and the shell is the first thing.
    assert!(Support::shipped()
        .disabled_features()
        .iter()
        .any(|f| f == "shell_tool"));
}

#[test]
fn a_version_runs_only_as_it_was_verified() {
    let listed = support(ONE, &["shell_tool"], &[]);

    assert_eq!(
        listed.check("linux", "0.159.0", "codex/abc123", &[]),
        Ok(())
    );
    for (os, version, instructions) in [
        ("windows", "0.159.0", "codex/abc123"),
        ("linux", "0.159.1", "codex/abc123"),
        ("linux", "0.159.0", "codex/zzz999"),
    ] {
        let refused = listed.check(os, version, instructions, &[]).unwrap_err();
        assert!(
            matches!(refused, Unsupported::Version { .. }),
            "{os} {version} {instructions}"
        );
    }
}

#[test]
fn a_refusal_says_to_install_one_that_is_supported_or_choose_another_connection() {
    let refused = Support::shipped()
        .check("linux", "0.159.0", "codex/abc123", &[])
        .unwrap_err();

    let said = refused.to_string();
    assert!(said.contains("0.159.0"), "{said}");
    assert!(said.contains("not been verified"), "{said}");
    assert!(said.contains("another connection"), "{said}");
}

#[test]
fn a_feature_that_is_on_and_that_nobody_switched_off_or_reviewed_is_not_run_beside() {
    let listed = support(
        ONE,
        &["shell_tool", "unified_exec"],
        &["fast_mode", "goals"],
    );
    let on = enabled_features(LISTED);

    let refused = listed
        .check("linux", "0.159.0", "codex/abc123", &on)
        .unwrap_err();

    // `apps`, `browser_use` and `view_image` are on, and nobody said what to do about them.
    assert_eq!(
        refused,
        Unsupported::Features(vec![
            "apps".to_string(),
            "browser_use".to_string(),
            "view_image".to_string()
        ])
    );
    assert!(refused.to_string().contains("browser_use"));
}

#[test]
fn what_is_switched_off_or_reviewed_is_not_in_the_way() {
    let listed = support(
        ONE,
        &[
            "shell_tool",
            "unified_exec",
            "apps",
            "browser_use",
            "view_image",
        ],
        &["fast_mode", "goals"],
    );
    let on = enabled_features(LISTED);

    assert_eq!(
        listed.check("linux", "0.159.0", "codex/abc123", &on),
        Ok(())
    );
}

#[test]
fn a_feature_that_is_off_is_not_asked_about() {
    let listed = support(ONE, &[], &[]);

    assert_eq!(
        listed.check("linux", "0.159.0", "codex/abc123", &["x".to_string()]),
        Err(Unsupported::Features(vec!["x".to_string()]))
    );
    assert_eq!(
        listed.check("linux", "0.159.0", "codex/abc123", &[]),
        Ok(())
    );
}

#[test]
fn a_list_that_is_not_one_is_refused_and_says_where() {
    for text in [
        "not json",
        r#"{"verified": "none"}"#,
        r#"{"verified": [{"os": "linux"}]}"#,
        r#"{"verified": [], "unknown_field": 1}"#,
    ] {
        assert!(Support::from_json(text).is_err(), "{text}");
    }
}
