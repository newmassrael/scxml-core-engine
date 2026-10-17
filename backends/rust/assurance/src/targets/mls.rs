// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The group key code reads MLS messages another device made (a Welcome, a commit, a key package) and
//! the state a device kept of itself. Each is given the recorded scenario's true bytes
//! (`backends/rust/mls/tests/fixtures`). The library may refuse (it must say so as an error) and may
//! not panic, a device that refused is as it was, and one that was given a changed commit does not
//! derive a key it was not given.

use sce_mls_capi::{
    sce_mls_apply_commit, sce_mls_device_free, sce_mls_device_from_bytes, sce_mls_join,
    sce_mls_log_free, sce_mls_log_new,
};
use sce_rust_mls::{Device, Roster};
use sce_rust_trust::Identity;
use serde_json::Value;

use super::Target;

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../mls/tests/fixtures/join_and_remove.json"
);

/// The rounds the library's own cost allows: a group operation is much dearer than a parse.
const ROUNDS: usize = 300;

fn unhex(text: &str) -> Vec<u8> {
    (0..text.len() / 2)
        .map(|index| u8::from_str_radix(&text[2 * index..2 * index + 2], 16).unwrap())
        .collect()
}

fn roster_of(value: &Value) -> Roster {
    let mut roster = Roster::new(u32::try_from(value["epoch"].as_u64().unwrap()).unwrap());
    for member in value["members"].as_array().unwrap() {
        roster.add_member(
            member[0].as_u64().unwrap(),
            unhex(member[1].as_str().unwrap()).try_into().unwrap(),
        );
    }
    for commit in value["commits"].as_array().unwrap() {
        roster.name_commit(
            u32::try_from(commit[0].as_u64().unwrap()).unwrap(),
            unhex(commit[1].as_str().unwrap()).try_into().unwrap(),
        );
    }
    roster
}

/// The recorded scenario: a device's seed and state, the Welcome that joins it, the commits that
/// follow, and the log's roster at each.
pub struct Scenario {
    pub seed: [u8; 32],
    pub state: Vec<u8>,
    pub welcome: Vec<u8>,
    pub commits: Vec<Vec<u8>>,
    pub rosters: Vec<Value>,
}

pub fn scenario() -> Scenario {
    let value: Value = serde_json::from_str(&std::fs::read_to_string(FIXTURE).unwrap()).unwrap();
    let steps = value["steps"].as_array().unwrap();
    Scenario {
        seed: unhex(value["device"]["seed"].as_str().unwrap())
            .try_into()
            .unwrap(),
        state: unhex(value["device"]["state"].as_str().unwrap()),
        welcome: unhex(steps[0]["welcome"].as_str().unwrap()),
        commits: steps
            .iter()
            .map(|step| unhex(step["commit"].as_str().unwrap()))
            .collect(),
        rosters: steps.iter().map(|step| step["log"].clone()).collect(),
    }
}

fn device(scenario: &Scenario) -> Device {
    Device::from_bytes(
        "home",
        2,
        Identity::from_seed(scenario.seed),
        &scenario.state,
    )
    .unwrap()
}

pub fn device_state() -> Target {
    let scenario = scenario();
    Target::new(
        "device-state",
        ROUNDS,
        vec![scenario.state.clone()],
        move |input| {
            let _ = Device::from_bytes("home", 2, Identity::from_seed(scenario.seed), input);
        },
    )
}

/// A Welcome that was changed does not make the device a member of anything; the true commit that
/// names the epoch is given with it.
pub fn welcome() -> Target {
    let scenario = scenario();
    let roster = roster_of(&scenario.rosters[0]);
    Target::new(
        "welcome",
        ROUNDS,
        vec![scenario.welcome.clone()],
        move |input| {
            let mut bob = device(&scenario);
            if bob.join(input, &scenario.commits[0], &roster).is_ok() {
                // Only the true Welcome, or one that decrypts to the same group, makes it a member.
                assert_eq!(bob.epoch(), Ok(1));
            } else {
                assert!(bob.epoch().is_err(), "a refused Welcome left a group");
            }
        },
    )
}

/// A commit (the first, the epoch that makes the device a member) that was changed, with the log
/// naming the hash of the true one: the hash does not match, and the device is not moved.
pub fn join_commit() -> Target {
    let scenario = scenario();
    let roster = roster_of(&scenario.rosters[0]);
    Target::new(
        "join-commit",
        ROUNDS,
        vec![scenario.commits[0].clone()],
        move |input| {
            let mut bob = device(&scenario);
            assert!(
                bob.join(&scenario.welcome, input, &roster).is_err()
                    || input == scenario.commits[0]
            );
        },
    )
}

/// A commit the device applies in epoch 2, that was changed: it is refused and the device stays in
/// epoch 1 with its key.
pub fn apply_commit() -> Target {
    let scenario = scenario();
    let roster = roster_of(&scenario.rosters[0]);
    let roster_2 = roster_of(&scenario.rosters[1]);
    Target::new(
        "apply-commit",
        ROUNDS,
        vec![scenario.commits[1].clone()],
        move |input| {
            let mut bob = device(&scenario);
            bob.join(&scenario.welcome, &scenario.commits[0], &roster)
                .unwrap();
            let key = bob.epoch_key().unwrap();
            if input != scenario.commits[1] {
                assert!(bob.apply_commit(input, &roster_2).is_err());
                assert_eq!(bob.epoch(), Ok(1));
                assert_eq!(
                    bob.epoch_key().unwrap(),
                    key,
                    "a refused commit moved the key"
                );
            }
        },
    )
}

/// A key package that was changed is refused by the administrator's device.
pub fn key_package() -> Target {
    let mut carol = Device::new("home", 3, Identity::from_seed([3; 32]));
    let package = carol.key_package().unwrap();
    let mut members = Roster::new(0);
    members.add_member(1, Identity::from_seed([1; 32]).public().to_bytes());
    members.add_member(3, Identity::from_seed([3; 32]).public().to_bytes());
    Target::new("key-package", ROUNDS, vec![package.clone()], move |input| {
        let mut again = Device::new("home", 1, Identity::from_seed([1; 32]));
        again.create_group().unwrap();
        if input != package.as_slice() {
            let _ = again.stage_add(&[input.to_vec()], &members);
        }
    })
}

/// The same through the C ABI, where a lane passes pointers and lengths: a changed Welcome and commit
/// are given with the true length and with a length that is not theirs.
pub fn capi() -> Target {
    let scenario = scenario();
    let corpus = vec![scenario.welcome.clone(), scenario.commits[0].clone()];
    Target::new("capi", ROUNDS, corpus, move |input| {
        let list = c"home";
        // SAFETY: every pointer is to a live buffer for the length given, or null with length 0.
        unsafe {
            let device = sce_mls_device_from_bytes(
                list.as_ptr(),
                2,
                scenario.seed.as_ptr(),
                scenario.state.as_ptr(),
                scenario.state.len(),
            );
            assert!(!device.is_null());
            let log = sce_mls_log_new(1);
            let mut epoch = 0u64;
            let mut removed = 0i32;
            // A length that is the buffer's, and one that is zero.
            for length in [input.len(), 0] {
                let _ = sce_mls_join(
                    device,
                    input.as_ptr(),
                    length,
                    scenario.commits[0].as_ptr(),
                    scenario.commits[0].len(),
                    log,
                    &mut epoch,
                );
                let _ = sce_mls_apply_commit(
                    device,
                    input.as_ptr(),
                    length,
                    log,
                    &mut removed,
                    &mut epoch,
                );
            }
            sce_mls_log_free(log);
            sce_mls_device_free(device);
        }
    })
}
