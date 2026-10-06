// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Which ways of signing in the workbench uses, and which it does not.
//!
//! What a provider's terms allow is a fact about a way of reaching its model, not about a
//! connection: the same Claude Code is reached by a subscription, by an API key, by a cloud
//! provider's credential, and the terms say something different of each. The design keeps
//! that as one table of routes with a status each, and the table is a constant of the build:
//! a person cannot widen it, a release can narrow it (switch a route off) without a change to
//! anything else, and a route that is not in it is not used. This file is that table, written
//! down a second time, so that a change to one cannot go unnoticed in the other.

use sce_app_core::{AdapterKind, AuthSource, Decision, Observed, Policy, Reason, Route, Status};
use serde_json::json;

/// The design's table of routes (the section on which sign-in methods are allowed).
const TABLE: [(Route, Status); 13] = [
    (Route::ClaudeOfficialLogin, Status::Conditional),
    (Route::ClaudeApiKey, Status::Allowed),
    (Route::ClaudeCloudProvider, Status::Allowed),
    (Route::ClaudeLoginCommandByApp, Status::Unconfirmed),
    (Route::ClaudeLoginScreenByApp, Status::Forbidden),
    (Route::CodexAppServerAuth, Status::Forbidden),
    (Route::CodexSignInWithChatGpt, Status::Unconfirmed),
    (Route::CodexCliApiKey, Status::Allowed),
    (Route::CodexCliChatGptLogin, Status::Conditional),
    (Route::CodexAppServerApiKey, Status::Unconfirmed),
    (Route::CodexAccessToken, Status::Unconfirmed),
    (Route::LocalServer, Status::Allowed),
    (Route::Unlisted, Status::Unconfirmed),
];

#[test]
fn every_route_has_the_status_the_design_table_gives_it() {
    for (route, status) in TABLE {
        assert_eq!(route.status(), status, "{route:?}");
    }
}

#[test]
fn the_table_here_lists_every_route_there_is() {
    assert_eq!(Route::ALL.len(), TABLE.len());
    for route in Route::ALL {
        assert!(
            TABLE.iter().any(|(r, _)| r == route),
            "{route:?} is a route and is not in the table above: say what the design says of it"
        );
    }
}

#[test]
fn only_allowed_and_conditional_routes_are_used() {
    let policy = Policy::shipped();

    for (route, status) in TABLE {
        let decision = policy.decide(route);
        match status {
            Status::Allowed | Status::Conditional => {
                assert_eq!(decision, Decision::Use { status }, "{route:?}");
            }
            Status::Forbidden => assert_eq!(
                decision,
                Decision::Refuse {
                    status,
                    reason: Reason::Forbidden
                },
                "{route:?}"
            ),
            Status::Unconfirmed => assert_eq!(
                decision,
                Decision::Refuse {
                    status,
                    reason: Reason::Unconfirmed
                },
                "{route:?}"
            ),
        }
    }
}

#[test]
fn nothing_is_switched_off_in_the_shipped_build() {
    assert!(Policy::shipped().switched_off().is_empty());
}

#[test]
fn a_route_can_be_switched_off_and_nothing_else_changes() {
    let policy = Policy::shipped().with_switched_off([Route::ClaudeOfficialLogin]);

    assert_eq!(
        policy.decide(Route::ClaudeOfficialLogin),
        Decision::Refuse {
            status: Status::Conditional,
            reason: Reason::SwitchedOff
        }
    );
    for (route, _) in TABLE {
        if route != Route::ClaudeOfficialLogin {
            assert_eq!(
                policy.decide(route),
                Policy::shipped().decide(route),
                "{route:?}"
            );
        }
    }
}

#[test]
fn a_switch_only_ever_turns_a_route_off() {
    // Every route switched off: a refused route is still refused for the reason it always was,
    // which says more than "switched off", and the routes that were used are the ones that stop.
    let policy = Policy::shipped().with_switched_off(Route::ALL.iter().copied());

    for (route, status) in TABLE {
        let reason = match status {
            Status::Forbidden => Reason::Forbidden,
            Status::Unconfirmed => Reason::Unconfirmed,
            Status::Allowed | Status::Conditional => Reason::SwitchedOff,
        };
        assert_eq!(
            policy.decide(route),
            Decision::Refuse { status, reason },
            "{route:?}"
        );
    }
}

#[test]
fn the_route_a_connection_takes_follows_what_its_client_reports() {
    use AdapterKind::{ClaudeCode, Codex, Local};
    use AuthSource::{AppStore, EnvApiKey, NoAuth, OfficialLogin, ServerKey};
    use Observed::{AccessToken, ApiKey, CloudProvider, Other, Subscription};
    let table = [
        (
            ClaudeCode,
            OfficialLogin,
            Some(Subscription),
            Some(Route::ClaudeOfficialLogin),
        ),
        (
            ClaudeCode,
            OfficialLogin,
            Some(ApiKey),
            Some(Route::ClaudeApiKey),
        ),
        (
            ClaudeCode,
            OfficialLogin,
            Some(CloudProvider),
            Some(Route::ClaudeCloudProvider),
        ),
        (
            ClaudeCode,
            OfficialLogin,
            Some(AccessToken),
            Some(Route::Unlisted),
        ),
        (
            ClaudeCode,
            OfficialLogin,
            Some(Other),
            Some(Route::Unlisted),
        ),
        // Signed out: there is no route yet, and the screen says to sign in.
        (ClaudeCode, OfficialLogin, None, None),
        (Codex, AppStore, Some(ApiKey), Some(Route::CodexCliApiKey)),
        (
            Codex,
            AppStore,
            Some(Subscription),
            Some(Route::CodexCliChatGptLogin),
        ),
        (
            Codex,
            OfficialLogin,
            Some(Subscription),
            Some(Route::CodexCliChatGptLogin),
        ),
        (
            Codex,
            OfficialLogin,
            Some(ApiKey),
            Some(Route::CodexCliApiKey),
        ),
        (
            Codex,
            AppStore,
            Some(AccessToken),
            Some(Route::CodexAccessToken),
        ),
        (Codex, AppStore, Some(CloudProvider), Some(Route::Unlisted)),
        (Codex, AppStore, Some(Other), Some(Route::Unlisted)),
        (Codex, AppStore, None, None),
        // The variable is the credential: what a stored login says does not decide it.
        (Codex, EnvApiKey, None, Some(Route::CodexCliApiKey)),
        (
            Codex,
            EnvApiKey,
            Some(Subscription),
            Some(Route::CodexCliApiKey),
        ),
        // A local server has no provider to ask.
        (Local, NoAuth, None, Some(Route::LocalServer)),
        (Local, ServerKey, None, Some(Route::LocalServer)),
    ];

    for (adapter, auth, observed, route) in table {
        assert_eq!(
            Route::of(adapter, auth, observed),
            route,
            "{adapter:?} with {auth:?}, the client reporting {observed:?}"
        );
    }
}

#[test]
fn a_route_a_status_and_a_decision_are_sent_under_stable_words() {
    assert_eq!(
        serde_json::to_value(Route::ClaudeOfficialLogin).unwrap(),
        json!("claude-official-login")
    );
    assert_eq!(
        serde_json::to_value(Route::CodexCliChatGptLogin).unwrap(),
        json!("codex-cli-chat-gpt-login")
    );
    for (status, word) in [
        (Status::Allowed, "allowed"),
        (Status::Conditional, "conditional"),
        (Status::Forbidden, "forbidden"),
        (Status::Unconfirmed, "unconfirmed"),
    ] {
        assert_eq!(serde_json::to_value(status).unwrap(), json!(word));
    }
    assert_eq!(
        serde_json::to_value(Decision::Use {
            status: Status::Allowed
        })
        .unwrap(),
        json!({ "decision": "use", "status": "allowed" })
    );
    assert_eq!(
        serde_json::to_value(Decision::Refuse {
            status: Status::Conditional,
            reason: Reason::SwitchedOff
        })
        .unwrap(),
        json!({ "decision": "refuse", "status": "conditional", "reason": "switched-off" })
    );
}
