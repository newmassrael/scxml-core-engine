// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
//
// MLS is randomised (key packages, HPKE, the commit's path), so a vector of what it gives cannot be
// computed from fixed inputs: it is a recorded scenario. One device (bob) is kept as it was before it
// joined (its group state, which holds the private half of a key package) with the messages it is
// then given: the Welcome and the commit that made its epoch, the commit that adds a third member, and
// the commit that removes it, each with the membership log as data and what the device must derive. A
// lane replays it: it loads the device, joins, applies, and the keys it derives for each epoch (and
// for a section) are the recorded ones, until the device is removed and derives none. The Rust lane
// replays it here, and the C ABI's tests replay the same file from C and from Python.
//
// The scenario is regenerated with `SCE_MLS_SCENARIO=write cargo test -p sce-rust-mls --test
// a_known_answer_scenario_is_what_every_lane_replays`; the leaves last ten years, so the file does
// not go stale on a clock.

use sce_rust_mls::{Applied, Device, MlsError, Roster};
use sce_rust_trust::Identity;
use serde_json::{json, Value};

const LIST: &str = "home";
const PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/join_and_remove.json"
);

fn seed_of(node: u64) -> [u8; 32] {
    [node as u8; 32]
}

fn device(node: u64) -> Device {
    Device::new(LIST, node, Identity::from_seed(seed_of(node)))
}

fn key_of(node: u64) -> [u8; 32] {
    Identity::from_seed(seed_of(node)).public().to_bytes()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn unhex(text: &str) -> Vec<u8> {
    (0..text.len() / 2)
        .map(|index| u8::from_str_radix(&text[2 * index..2 * index + 2], 16).unwrap())
        .collect()
}

/// A log as the data a lane is told, and the roster that is the same.
fn log(epoch: u32, members: &[u64], commits: &[(u32, [u8; 32])]) -> (Value, Roster) {
    let mut roster = Roster::new(epoch);
    for node in members {
        roster.add_member(*node, key_of(*node));
    }
    for (named, hash) in commits {
        roster.name_commit(*named, *hash);
    }
    let value = json!({
        "epoch": epoch,
        "members": members.iter().map(|node| json!([node, hex(&key_of(*node))])).collect::<Vec<_>>(),
        "commits": commits.iter().map(|(named, hash)| json!([named, hex(hash)])).collect::<Vec<_>>(),
    });
    (value, roster)
}

fn roster_of(value: &Value) -> Roster {
    let mut roster = Roster::new(u32::try_from(value["epoch"].as_u64().unwrap()).unwrap());
    for member in value["members"].as_array().unwrap() {
        let key: [u8; 32] = unhex(member[1].as_str().unwrap()).try_into().unwrap();
        roster.add_member(member[0].as_u64().unwrap(), key);
    }
    for commit in value["commits"].as_array().unwrap() {
        let hash: [u8; 32] = unhex(commit[1].as_str().unwrap()).try_into().unwrap();
        roster.name_commit(u32::try_from(commit[0].as_u64().unwrap()).unwrap(), hash);
    }
    roster
}

/// alice (node 1) runs the group; bob (node 2) joins in epoch 1, carol (node 3) is added in epoch 2,
/// and bob is removed in epoch 3.
fn record() -> Value {
    let (mut alice, mut bob, mut carol) = (device(1), device(2), device(3));
    alice.create_group().unwrap();
    let (bob_package, carol_package) = (bob.key_package().unwrap(), carol.key_package().unwrap());
    let bob_before = bob.to_bytes().unwrap();

    // Epoch 1: bob is added.
    let (_, grant_bob) = log(0, &[1, 2], &[]);
    let one = alice.stage_add(&[bob_package], &grant_bob).unwrap();
    let (log_1, roster_1) = log(1, &[1, 2], &[(1, one.commit_hash)]);
    alice.merge_pending(&roster_1).unwrap();
    let welcome_1 = one.welcome.clone().unwrap();
    assert_eq!(bob.join(&welcome_1, &one.commit, &roster_1), Ok(1));
    let key_1 = bob.epoch_key().unwrap();
    assert_eq!(alice.epoch_key().unwrap(), key_1);
    let section = bob.section_key("chores").unwrap();

    // Epoch 2: carol is added, and bob applies the commit.
    let (_, grant_carol) = log(1, &[1, 2, 3], &[(1, one.commit_hash)]);
    let two = alice.stage_add(&[carol_package], &grant_carol).unwrap();
    let commits_2 = [(1, one.commit_hash), (2, two.commit_hash)];
    let (log_2, roster_2) = log(2, &[1, 2, 3], &commits_2);
    alice.merge_pending(&roster_2).unwrap();
    assert_eq!(
        bob.apply_commit(&two.commit, &roster_2),
        Ok(Applied::Epoch(2))
    );
    let key_2 = bob.epoch_key().unwrap();
    assert_eq!(alice.epoch_key().unwrap(), key_2);

    // Epoch 3: bob is removed.
    let (_, off_bob) = log(2, &[1, 3], &commits_2);
    let three = alice.stage_remove(&[2], &off_bob).unwrap();
    let commits_3 = [
        (1, one.commit_hash),
        (2, two.commit_hash),
        (3, three.commit_hash),
    ];
    let (log_3, roster_3) = log(3, &[1, 3], &commits_3);
    alice.merge_pending(&roster_3).unwrap();
    assert_eq!(
        bob.apply_commit(&three.commit, &roster_3),
        Ok(Applied::Removed)
    );
    assert_eq!(bob.epoch_key(), Err(MlsError::Removed));

    json!({
        "version": 1,
        "suite": "MLS_128_DHKEMX25519_AES128GCM_SHA256_Ed25519",
        "list": LIST,
        "device": {"node": 2, "seed": hex(&seed_of(2)), "state": hex(&bob_before)},
        "steps": [
            {"op": "join", "welcome": hex(&welcome_1), "commit": hex(&one.commit), "log": log_1,
             "epoch": 1, "epoch_key": hex(&key_1),
             "section": "chores", "section_key": hex(&section)},
            {"op": "apply", "commit": hex(&two.commit), "log": log_2,
             "epoch": 2, "epoch_key": hex(&key_2)},
            {"op": "apply", "commit": hex(&three.commit), "log": log_3, "removed": true},
        ],
    })
}

/// What a lane does with the scenario, in Rust.
fn replay(scenario: &Value) {
    let device = &scenario["device"];
    let mut bob = Device::from_bytes(
        scenario["list"].as_str().unwrap(),
        device["node"].as_u64().unwrap(),
        Identity::from_seed(unhex(device["seed"].as_str().unwrap()).try_into().unwrap()),
        &unhex(device["state"].as_str().unwrap()),
    )
    .unwrap();
    for step in scenario["steps"].as_array().unwrap() {
        let roster = roster_of(&step["log"]);
        let commit = unhex(step["commit"].as_str().unwrap());
        match step["op"].as_str().unwrap() {
            "join" => {
                let welcome = unhex(step["welcome"].as_str().unwrap());
                assert_eq!(
                    bob.join(&welcome, &commit, &roster),
                    Ok(step["epoch"].as_u64().unwrap())
                );
                let section = step["section"].as_str().unwrap();
                assert_eq!(
                    hex(&bob.section_key(section).unwrap()),
                    step["section_key"].as_str().unwrap()
                );
            }
            "apply" if step["removed"].as_bool() == Some(true) => {
                assert_eq!(bob.apply_commit(&commit, &roster), Ok(Applied::Removed));
                assert_eq!(bob.epoch_key(), Err(MlsError::Removed));
                continue;
            }
            "apply" => assert_eq!(
                bob.apply_commit(&commit, &roster),
                Ok(Applied::Epoch(step["epoch"].as_u64().unwrap()))
            ),
            other => panic!("{other}"),
        }
        assert_eq!(
            hex(&bob.epoch_key().unwrap()),
            step["epoch_key"].as_str().unwrap()
        );
    }
}

#[test]
fn the_recorded_scenario_replays_in_rust() {
    if std::env::var("SCE_MLS_SCENARIO").as_deref() == Ok("write") {
        let scenario = record();
        std::fs::create_dir_all(std::path::Path::new(PATH).parent().unwrap()).unwrap();
        std::fs::write(
            PATH,
            serde_json::to_string_pretty(&scenario).unwrap() + "\n",
        )
        .unwrap();
    }
    let text = std::fs::read_to_string(PATH).unwrap_or_else(|_| {
        panic!("{PATH} is not recorded: SCE_MLS_SCENARIO=write cargo test -p sce-rust-mls --test a_known_answer_scenario_is_what_every_lane_replays")
    });
    replay(&serde_json::from_str(&text).unwrap());
}

/// The same scenario recorded now replays too, so that a scenario that the library can no longer
/// replay is told from one that was recorded wrong.
#[test]
fn a_scenario_recorded_now_replays_in_rust() {
    replay(&record());
}
