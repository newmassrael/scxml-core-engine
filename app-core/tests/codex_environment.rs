// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What reaches Codex of the person's environment, by the credential the connection chose.
//!
//! One source of credentials is used for a run and the others are taken away, so that what the
//! screen says the run is billed to is what it is billed to. A key that happens to be in the
//! person's shell does not replace the login a connection chose, and a login that happens to be
//! stored does not stand in for a key a connection asked the environment for (that is a problem
//! with the environment, said as one). Only names are handled here: a value of the person's is
//! never read into what is returned, so there is nothing of it to leak into a log.

use std::path::{Path, PathBuf};

use sce_app_core::codex_environment::{environment_for, Refused};
use sce_app_core::AuthSource;

const APP_HOME: &str = "/app/data/codex-home";

fn parent(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

fn run(
    auth: AuthSource,
    env: &[(&str, &str)],
) -> Result<sce_app_core::codex_environment::Environment, Refused> {
    environment_for(auth, Path::new(APP_HOME), &parent(env))
}

/// Everything in the shell that could pick a credential.
const CROWDED: &[(&str, &str)] = &[
    ("PATH", "/usr/bin"),
    ("HOME", "/home/person"),
    ("CODEX_API_KEY", "sk-codex-secret"),
    ("OPENAI_API_KEY", "sk-openai-secret"),
    ("CODEX_ACCESS_TOKEN", "token-secret"),
    ("OPENAI_IDENTITY_TOKEN", "identity-secret"),
    ("OPENAI_IDENTITY_AUDIENCE", "audience-secret"),
    ("CODEX_CONNECTORS_TOKEN", "connectors-secret"),
    ("AWS_SECRET_ACCESS_KEY", "aws-secret"),
    ("AZURE_CLIENT_SECRET", "azure-secret"),
    ("GOOGLE_APPLICATION_CREDENTIALS", "/home/person/gcp.json"),
];

fn removed(environment: &sce_app_core::codex_environment::Environment) -> Vec<&str> {
    environment.remove.iter().map(String::as_str).collect()
}

#[test]
fn the_stored_login_of_the_application_is_used_with_a_home_of_its_own() {
    let environment = run(AuthSource::AppStore, CROWDED).unwrap();

    assert_eq!(environment.home, Some(PathBuf::from(APP_HOME)));
}

#[test]
fn the_login_the_person_made_with_the_official_client_is_used_where_it_keeps_it() {
    let environment = run(AuthSource::OfficialLogin, CROWDED).unwrap();

    // No home of ours: the client's own is where that login is, and which one that is the
    // person's environment says (`CODEX_HOME`, when they set one), not the application.
    assert_eq!(environment.home, None);
    assert!(!removed(&environment).contains(&"CODEX_HOME"));
}

#[test]
fn a_key_in_the_environment_is_the_only_credential_and_the_application_home_keeps_a_stored_login_out(
) {
    let environment = run(AuthSource::EnvApiKey, CROWDED).unwrap();

    // Of every name that could pick a credential, only the one the connection chose stays.
    let gone = removed(&environment);
    assert!(!gone.contains(&"CODEX_API_KEY"), "{gone:?}");
    for name in [
        "OPENAI_API_KEY",
        "CODEX_ACCESS_TOKEN",
        "OPENAI_IDENTITY_TOKEN",
        "OPENAI_IDENTITY_AUDIENCE",
        "CODEX_CONNECTORS_TOKEN",
        "AWS_SECRET_ACCESS_KEY",
        "AZURE_CLIENT_SECRET",
        "GOOGLE_APPLICATION_CREDENTIALS",
    ] {
        assert!(gone.contains(&name), "{name} stayed: {gone:?}");
    }
    // A login stored in the person's own home is not what a connection that chose a key uses.
    assert_eq!(environment.home, Some(PathBuf::from(APP_HOME)));
}

#[test]
fn a_connection_that_chose_a_key_in_the_environment_does_not_fall_back_when_there_is_none() {
    // A stored login would run, billed to a plan, a connection that said a key: it is a problem
    // with the environment, and is said to be one.
    let without = &[("PATH", "/usr/bin"), ("OPENAI_API_KEY", "sk-other")];

    let refused = run(AuthSource::EnvApiKey, without).unwrap_err();

    assert_eq!(refused, Refused::VariableMissing("CODEX_API_KEY"));
    // An empty variable is none.
    let empty = &[("CODEX_API_KEY", "")];
    assert_eq!(
        run(AuthSource::EnvApiKey, empty).unwrap_err(),
        Refused::VariableMissing("CODEX_API_KEY")
    );
}

#[test]
fn a_stored_login_is_used_alone_whichever_store_it_is() {
    for auth in [AuthSource::AppStore, AuthSource::OfficialLogin] {
        let gone = run(auth, CROWDED).unwrap();
        let gone = removed(&gone);

        // The key in the shell does not take over from the login the connection chose.
        for name in [
            "CODEX_API_KEY",
            "OPENAI_API_KEY",
            "CODEX_ACCESS_TOKEN",
            "OPENAI_IDENTITY_TOKEN",
            "OPENAI_IDENTITY_AUDIENCE",
            "CODEX_CONNECTORS_TOKEN",
            "AWS_SECRET_ACCESS_KEY",
            "AZURE_CLIENT_SECRET",
            "GOOGLE_APPLICATION_CREDENTIALS",
        ] {
            assert!(gone.contains(&name), "{auth:?}: {name} stayed: {gone:?}");
        }
    }
}

#[test]
fn what_is_not_a_credential_is_left_alone() {
    for auth in [
        AuthSource::AppStore,
        AuthSource::OfficialLogin,
        AuthSource::EnvApiKey,
    ] {
        let environment = run(auth, CROWDED).unwrap();
        let gone = removed(&environment);

        for name in ["PATH", "HOME", "CODEX_HOME", "TMPDIR", "LANG"] {
            assert!(!gone.contains(&name), "{auth:?}: {name} was taken away");
        }
    }
}

#[test]
fn a_family_of_names_is_taken_whole_and_a_name_that_only_looks_like_one_is_not() {
    let environment = run(
        AuthSource::AppStore,
        &[
            ("OPENAI_IDENTITY_ANYTHING_NEW", "x"),
            ("OPENAI_IDENTITY_", "x"),
            ("OPENAI_IDENTITY", "x"),
            ("MY_OPENAI_IDENTITY_TOKEN", "x"),
        ],
    )
    .unwrap();
    let gone = removed(&environment);

    // A name a later version adds to the family is in it; a name that is not of it is not.
    assert!(gone.contains(&"OPENAI_IDENTITY_ANYTHING_NEW"), "{gone:?}");
    assert!(gone.contains(&"OPENAI_IDENTITY_"), "{gone:?}");
    assert!(!gone.contains(&"OPENAI_IDENTITY"), "{gone:?}");
    assert!(!gone.contains(&"MY_OPENAI_IDENTITY_TOKEN"), "{gone:?}");
}

#[test]
fn the_names_that_are_always_taken_are_taken_when_the_shell_has_none_of_them_yet() {
    // A variable set between the look and the start is as gone as one that was there.
    let environment = run(AuthSource::OfficialLogin, &[("PATH", "/usr/bin")]).unwrap();

    assert!(removed(&environment).contains(&"CODEX_API_KEY"));
    assert!(removed(&environment).contains(&"OPENAI_API_KEY"));
}

#[test]
fn nothing_of_the_persons_is_in_what_is_returned() {
    let environment = run(AuthSource::EnvApiKey, CROWDED).unwrap();

    let said = format!("{environment:?}");
    for secret in [
        "sk-codex-secret",
        "sk-openai-secret",
        "token-secret",
        "identity-secret",
        "audience-secret",
        "connectors-secret",
        "aws-secret",
        "azure-secret",
        "/home/person/gcp.json",
    ] {
        assert!(!said.contains(secret), "{secret} is in {said}");
    }
}

#[test]
fn a_source_that_is_not_one_of_codexs_is_refused() {
    for auth in [AuthSource::ServerKey, AuthSource::NoAuth] {
        assert_eq!(
            run(auth, CROWDED).unwrap_err(),
            Refused::NotACodexSource(auth)
        );
    }
}
