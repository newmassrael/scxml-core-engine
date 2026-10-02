// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! An author's JSON that writes one key twice is refused by every reader that
//! takes an owner's file, not resolved to the last value.
//!
//! `serde_json` keeps the last value and says nothing. Reproduced against the
//! product before the readers went through `strict_json`: a scenario set that
//! wrote `origin` as `ai-proposed` and then as `owner-written` was judged as
//! the owner's own examples with `problems: 0`. The unit tests of
//! `strict_json` hold the refusal itself; this file holds that EACH READER
//! goes through it, since a reader that parsed with `serde_json::from_str`
//! directly would keep every one of those tests green.
//!
//! Each case hands a reader text that is wrong in exactly one way the reader
//! does not otherwise look at (a repeated key), beside a control that is the
//! same text without the repeat and is refused for what it is, so a pass here
//! is the repeat being named and not any other reason to refuse.

use sce_build::acceptance_record::AcceptanceRecord;
use sce_build::authoring_profile::{AuthoringProfile, ProfileFault};
use sce_build::requirement_manifest::RequirementManifest;
use sce_build::scenario_judge::Trace;
use sce_build::scenario_set::ScenarioSet;

const REPEATED: &str = r#"{"record": "one", "record": "two"}"#;
const SINGLE: &str = r#"{"record": "one"}"#;

fn names_the_repeat(said: &str, reader: &str) {
    assert!(
        said.contains(r#"the key "record" is written twice"#),
        "{reader} did not name the repeated key: {said}"
    );
}

fn does_not_name_a_repeat(said: &str, reader: &str) {
    assert!(
        !said.contains("is written twice"),
        "{reader}'s control (no repeat) was refused as a repeat: {said}"
    );
}

#[test]
fn a_scenario_set_reader_refuses_a_repeated_key() {
    let said = ScenarioSet::from_json(REPEATED, "set.json")
        .expect_err("a repeat is refused")
        .to_string();
    names_the_repeat(&said, "ScenarioSet::from_json");
    let control = ScenarioSet::from_json(SINGLE, "set.json")
        .expect_err("the control is not a scenario set")
        .to_string();
    does_not_name_a_repeat(&control, "ScenarioSet::from_json");
}

#[test]
fn an_observation_trace_reader_refuses_a_repeated_key() {
    let said = Trace::from_json(REPEATED, "trace.json")
        .expect_err("a repeat is refused")
        .to_string();
    names_the_repeat(&said, "Trace::from_json");
    let control = Trace::from_json(SINGLE, "trace.json")
        .expect_err("the control is not a trace")
        .to_string();
    does_not_name_a_repeat(&control, "Trace::from_json");
}

#[test]
fn an_authoring_profile_reader_refuses_a_repeated_key_as_a_shape_fault() {
    let refused = AuthoringProfile::from_text(REPEATED).expect_err("a repeat is refused");
    names_the_repeat(&refused.detail, "AuthoringProfile::from_text");
    // A repeat is well-formed JSON of the wrong shape, not text that is not JSON:
    // the caller that tells the two apart must call it the former.
    assert_eq!(ProfileFault::InvalidShape, refused.kind);
    let control = AuthoringProfile::from_text(SINGLE).expect_err("the control is not a profile");
    does_not_name_a_repeat(&control.detail, "AuthoringProfile::from_text");
}

#[test]
fn an_acceptance_record_reader_refuses_a_repeated_key() {
    let said = AcceptanceRecord::from_json(REPEATED)
        .expect_err("a repeat is refused")
        .to_string();
    names_the_repeat(&said, "AcceptanceRecord::from_json");
    let control = AcceptanceRecord::from_json(SINGLE)
        .expect_err("the control is not a record")
        .to_string();
    does_not_name_a_repeat(&control, "AcceptanceRecord::from_json");
}

#[test]
fn a_requirement_manifest_reader_refuses_a_repeated_key() {
    let said = RequirementManifest::from_json(REPEATED, "manifest.json")
        .expect_err("a repeat is refused")
        .to_string();
    names_the_repeat(&said, "RequirementManifest::from_json");
    let control = RequirementManifest::from_json(SINGLE, "manifest.json")
        .expect_err("the control is not a manifest")
        .to_string();
    does_not_name_a_repeat(&control, "RequirementManifest::from_json");
}
