// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A connection runs only as many requests at once as it has places for, across every spec.
//!
//! The rule is the request machine's (`machines/request_life`); what only a folder can break is
//! the count it is told: that it reads every spec, that it frees a place when a request ends or
//! its lease runs out, that one connection's full lane says nothing of another's, and that two
//! executors who meet at the last place for two specs do not both get it.

mod common;

use std::sync::{Arc, Barrier};
use std::thread;

use sce_specs::requests::{Failure, Inputs, State};
use sce_specs::{
    ConnectionId, ConnectionRef, ManualClock, Pin, Registration, Revision, SpecId, SpecStore,
    StoreError,
};
use serde_json::Map;

const T0: u64 = 1_791_190_800;

struct Lanes {
    clock: Arc<ManualClock>,
    store: SpecStore<Arc<ManualClock>>,
}

/// A request registered in a spec of its own, for the connection `connection`.
struct Made {
    spec: SpecId,
    request: String,
    offered: ConnectionRef,
}

fn lanes(label: &str) -> Lanes {
    let clock = Arc::new(ManualClock::at(T0));
    let store = SpecStore::with_clock(common::scratch(label), Arc::clone(&clock));
    Lanes { clock, store }
}

fn pin(connection: &str) -> Pin {
    Pin::new(
        ConnectionId::parse(connection).unwrap(),
        Revision::of(connection.as_bytes()),
        Map::new(),
    )
    .unwrap()
}

impl Lanes {
    fn made(&self, title: &str, connection: Option<&str>) -> Made {
        let spec = self.store.create_spec(title).unwrap().id;
        self.store
            .save_source(&spec, "The lock opens.", None)
            .unwrap();
        let source = self.store.head(&spec).unwrap().expect("a text");
        let pinned = connection.map(pin);
        let offered = pinned
            .as_ref()
            .map(Pin::reference)
            .unwrap_or_else(|| pin("none").reference());
        let request = self
            .store
            .register_request_for(
                &spec,
                Registration {
                    key: "press",
                    origin: "gui",
                    expect: Inputs {
                        source,
                        answers: None,
                    },
                    supersede: false,
                },
                pinned,
            )
            .unwrap()
            .request
            .id;
        Made {
            spec,
            request,
            offered,
        }
    }

    fn claim(&self, made: &Made, holder: &str, places: usize) -> Result<State, StoreError> {
        self.store
            .claim_request_within(
                &made.spec,
                &made.request,
                holder,
                None,
                false,
                Some(&made.offered),
                places,
            )
            .map(|view| view.state)
    }

    fn state_of(&self, made: &Made) -> State {
        self.store
            .read_request(&made.spec, &made.request)
            .unwrap()
            .state
    }
}

fn refused_as(result: Result<State, StoreError>) -> String {
    match result {
        Err(StoreError::Refused { kind, .. }) => kind.to_string(),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[test]
fn a_connection_with_one_place_runs_one_request_of_any_spec_at_a_time() {
    let lanes = lanes("lane-one-place");
    let first = lanes.made("First", Some("qwen"));
    let second = lanes.made("Second", Some("qwen"));

    assert_eq!(lanes.claim(&first, "desktop", 1).unwrap(), State::Running);
    assert_eq!(refused_as(lanes.claim(&second, "desktop", 1)), "lane-full");
    // A refusal leaves the request where it was, for the next look.
    assert_eq!(lanes.state_of(&second), State::Queued);
}

#[test]
fn a_connection_with_two_places_runs_two_and_refuses_the_third() {
    let lanes = lanes("lane-two-places");
    let made: Vec<Made> = (0..3)
        .map(|n| lanes.made(&format!("Spec {n}"), Some("claude")))
        .collect();

    assert_eq!(lanes.claim(&made[0], "desktop", 2).unwrap(), State::Running);
    assert_eq!(lanes.claim(&made[1], "desktop", 2).unwrap(), State::Running);
    assert_eq!(refused_as(lanes.claim(&made[2], "desktop", 2)), "lane-full");
}

#[test]
fn a_full_lane_says_nothing_of_another_connections_lane() {
    let lanes = lanes("lane-independent");
    let slow = lanes.made("Slow", Some("qwen"));
    let waiting = lanes.made("Waiting", Some("qwen"));
    let quick = lanes.made("Quick", Some("claude"));

    assert_eq!(lanes.claim(&slow, "desktop", 1).unwrap(), State::Running);
    assert_eq!(refused_as(lanes.claim(&waiting, "desktop", 1)), "lane-full");
    // The request of the other connection is taken while the first lane is full.
    assert_eq!(lanes.claim(&quick, "desktop", 1).unwrap(), State::Running);
}

#[test]
fn a_place_is_free_again_when_the_request_in_it_ends() {
    let lanes = lanes("lane-ends");
    let first = lanes.made("First", Some("qwen"));
    let second = lanes.made("Second", Some("qwen"));
    lanes.claim(&first, "desktop", 1).unwrap();
    assert_eq!(refused_as(lanes.claim(&second, "desktop", 1)), "lane-full");

    lanes
        .store
        .fail_request(
            &first.spec,
            &first.request,
            "desktop",
            1,
            "it could not",
            Failure::unclassified(),
        )
        .unwrap();

    assert_eq!(lanes.claim(&second, "desktop", 1).unwrap(), State::Running);
}

#[test]
fn a_place_is_free_again_when_the_lease_in_it_runs_out() {
    let lanes = lanes("lane-lease");
    let first = lanes.made("First", Some("qwen"));
    let second = lanes.made("Second", Some("qwen"));
    lanes.claim(&first, "desktop", 1).unwrap();
    assert_eq!(refused_as(lanes.claim(&second, "desktop", 1)), "lane-full");

    // The executor stops renewing: the request is read as let go of, and holds no place.
    lanes.clock.advance(61);
    assert_eq!(lanes.state_of(&first), State::Interrupted);

    assert_eq!(lanes.claim(&second, "desktop", 1).unwrap(), State::Running);
}

#[test]
fn a_request_that_is_called_off_frees_its_place_too() {
    let lanes = lanes("lane-cancelled");
    let first = lanes.made("First", Some("qwen"));
    let second = lanes.made("Second", Some("qwen"));
    lanes.claim(&first, "desktop", 1).unwrap();

    lanes
        .store
        .cancel_request(&first.spec, &first.request)
        .unwrap();

    assert_eq!(lanes.claim(&second, "desktop", 1).unwrap(), State::Running);
}

#[test]
fn a_request_that_is_not_taken_is_told_what_it_always_was_whatever_the_lane_holds() {
    let lanes = lanes("lane-held");
    let first = lanes.made("First", Some("qwen"));
    let second = lanes.made("Second", Some("qwen"));
    lanes.claim(&first, "desktop", 1).unwrap();
    lanes.claim(&second, "other", 2).unwrap();

    // A request held under a lease that has not run out is told it is held, not that the lane
    // is full: the lane is not what the caller can act on.
    assert_eq!(
        refused_as(lanes.claim(&second, "desktop", 1)),
        "request-held"
    );
}

/// What a queued request waits for, as the store says it: behind the running ones of its
/// connection, and how many will be taken first.
fn waits(lanes: &Lanes) -> Vec<(SpecId, usize, usize, usize)> {
    lanes
        .store
        .lane_waits()
        .unwrap()
        .into_iter()
        .map(|wait| (wait.spec, wait.running, wait.places, wait.ahead))
        .collect()
}

#[test]
fn a_request_that_waits_for_a_place_is_told_what_it_waits_behind_and_how_far() {
    let lanes = lanes("lane-waits");
    // Requests made in the same second are taken in the order of their spec's id; these are made
    // a second apart so that the order they were made in is the order they are taken in.
    let running = lanes.made("Running", Some("qwen"));
    lanes.clock.advance(1);
    let next = lanes.made("Next", Some("qwen"));
    lanes.clock.advance(1);
    let later = lanes.made("Later", Some("qwen"));
    lanes.clock.advance(1);
    let other = lanes.made("Other", Some("claude"));

    // Nobody has said how many places `qwen` has, so nothing waits on it.
    assert_eq!(waits(&lanes), vec![]);

    lanes.claim(&running, "desktop", 1).unwrap();
    assert_eq!(refused_as(lanes.claim(&next, "desktop", 1)), "lane-full");

    // The queue is read in the order the requests were made, and the request of another
    // connection waits for nothing.
    assert_eq!(
        waits(&lanes),
        vec![(next.spec.clone(), 1, 1, 0), (later.spec.clone(), 1, 1, 1),]
    );
    assert_eq!(lanes.state_of(&other), State::Queued);

    // When the place is free again nobody waits for it.
    lanes
        .store
        .cancel_request(&running.spec, &running.request)
        .unwrap();
    assert_eq!(waits(&lanes), vec![]);
}

#[test]
fn a_connection_with_a_place_free_keeps_nobody_waiting() {
    let lanes = lanes("lane-waits-free");
    let first = lanes.made("First", Some("claude"));
    let second = lanes.made("Second", Some("claude"));

    lanes.claim(&first, "desktop", 2).unwrap();

    assert_eq!(lanes.state_of(&second), State::Queued);
    assert_eq!(waits(&lanes), vec![]);
}

#[test]
fn the_queue_is_a_command_every_host_reads_the_same_way() {
    let lanes = lanes("lane-waits-command");
    let first = lanes.made("First", Some("qwen"));
    let second = lanes.made("Second", Some("qwen"));
    lanes.claim(&first, "desktop", 1).unwrap();

    let answer = sce_specs::call(
        &lanes.store,
        &common::FakeRenderer,
        "read_queue",
        serde_json::json!({}),
    )
    .unwrap();

    assert_eq!(
        answer,
        serde_json::json!({ "waiting": [{
            "spec": second.spec.as_str(),
            "request": second.request,
            "connection": "qwen",
            "running": 1,
            "places": 1,
            "ahead": 0,
        }] })
    );
}

#[test]
fn the_places_a_connection_has_are_counted_only_from_requests_that_name_it() {
    let lanes = lanes("lane-unpinned");
    let unpinned = lanes.made("Unpinned", None);
    let pinned = lanes.made("Pinned", Some("qwen"));

    // A request of no connection has no lane: it is claimed whatever places are asked for.
    lanes
        .store
        .claim_request_within(
            &unpinned.spec,
            &unpinned.request,
            "desktop",
            None,
            false,
            None,
            1,
        )
        .unwrap();
    assert_eq!(lanes.claim(&pinned, "desktop", 1).unwrap(), State::Running);
}

#[test]
fn a_connection_with_no_places_is_refused_before_anything_is_counted() {
    let lanes = lanes("lane-zero");
    let made = lanes.made("First", Some("qwen"));
    assert_eq!(refused_as(lanes.claim(&made, "desktop", 0)), "bad-lane");
}

#[test]
fn two_executors_meeting_at_the_last_place_for_two_specs_do_not_both_get_it() {
    for round in 0..20 {
        let lanes = lanes(&format!("lane-race-{round}"));
        let made: Vec<Made> = (0..2)
            .map(|n| lanes.made(&format!("Spec {n}"), Some("qwen")))
            .collect();
        let barrier = Barrier::new(2);
        let results = thread::scope(|scope| {
            let handles: Vec<_> = made
                .iter()
                .enumerate()
                .map(|(n, made)| {
                    let (lanes, barrier) = (&lanes, &barrier);
                    scope.spawn(move || {
                        barrier.wait();
                        lanes.claim(made, &format!("worker-{n}"), 1)
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|handle| handle.join().unwrap())
                .collect::<Vec<_>>()
        });
        let taken = results.iter().filter(|result| result.is_ok()).count();
        assert_eq!(taken, 1, "round {round}: {results:?}");
        let refused: Vec<String> = results
            .into_iter()
            .filter_map(|result| result.err())
            .map(|error| match error {
                StoreError::Refused { kind, .. } => kind.to_string(),
                other => panic!("{other:?}"),
            })
            .collect();
        assert_eq!(refused, vec!["lane-full".to_string()], "round {round}");
    }
}
