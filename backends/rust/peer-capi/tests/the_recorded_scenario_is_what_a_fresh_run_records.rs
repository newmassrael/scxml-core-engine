// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
//
// The recorded scenario every lane replays through the peer core's C ABI
// (`tests/fixtures/two_devices_and_a_relay.json`). Two devices of one list run through the ABI
// against the real relay in this process: alice's, which holds the administrator's key and opens the
// list, and bob's, invited with the hash of the genesis and granted writing. Every call each device's
// core is given and every text it gives back is recorded in order: the machine's sends and what the
// core did with each (taken, held, not signed, refused and why), each request the core puts to the
// relay and the relay's answer, a failure of the transport, the outcome each machine is told, and
// bob's save, after which his core is made again and restored from it. What it shows: bob's writes
// and his note reach alice's machine; a restored device signs nothing new until its chain is read
// back, and gives a write it holds as held; another content under a stamp a device signed is
// refused; a round that loses its transport ends, and the next goes on.
//
// The core does no I/O, so a lane replays the scenario with no relay: it makes the same calls,
// compares each request, outcome and save the core gives with the recording, and gives the core the
// recorded answers. Everything in it is deterministic (Ed25519 signs deterministically, and nothing
// here reads a clock or draws a random number), so a run now records the same scenario, and a test
// fails when the recording and a fresh run differ: the file cannot drift from the library. The
// list's MLS group is not in it, since MLS draws randomness and a run would not record the same
// texts twice; the group's own C ABI has its own recorded scenario
// (`backends/rust/mls/tests/fixtures/join_and_remove.json`).
//
// The administrator's entries of the log (the genesis and the grants) are made with the trust, as
// `Peer` exposes no operation for them: alice's device starts from a save that holds them, made here
// by `Peer::save`, which the replay restores.
//
// The scenario is recorded again with `SCE_PEER_SCENARIO=write cargo test -p sce-rust-peer-capi
// --test the_recorded_scenario_is_what_a_fresh_run_records -- the_recorded_scenario_is_what`.

#![deny(unsafe_op_in_unsafe_fn)]

use std::ffi::{CStr, CString};
use std::path::Path;

use sce_peer_capi::*;
use sce_rust_peer::Peer;
use sce_rust_relay::{Authority, Limits, MemoryStore, Relay};
use sce_rust_trust::membership::{Body, Entry, Log, Profile};
use sce_rust_trust::Identity;
use serde_json::{json, Value};

const LIST: &str = "home";
const ALICE: u64 = 1;
const BOB: u64 = 2;
/// Where each device is in the recording's list of devices.
const AT_ALICE: usize = 0;
const AT_BOB: usize = 1;

const WRITE: &str = "record_list.write";
const SUPERSEDE: &str = "record_list.supersede";
const PROPOSE: &str = "record_list.propose";

const PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/two_devices_and_a_relay.json"
);
const WRITE_AGAIN: &str = "SCE_PEER_SCENARIO=write cargo test -p sce-rust-peer-capi --test \
     the_recorded_scenario_is_what_a_fresh_run_records -- the_recorded_scenario_is_what";

fn seed_of(node: u64) -> [u8; 32] {
    [node as u8; 32]
}

fn person(node: u64) -> Identity {
    Identity::from_seed(seed_of(node))
}

/// The administrator's key, which alice's device holds.
fn administrator() -> Identity {
    Identity::from_seed([9; 32])
}

fn genesis() -> Entry {
    Log::genesis_draft(
        LIST,
        Body::Genesis {
            administrators: vec![administrator().public()],
            quorum: 1,
            profile: Profile::Untrusted,
        },
    )
    .sign(&administrator())
    .finish()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn unhex(text: &str) -> Vec<u8> {
    (0..text.len() / 2)
        .map(|index| u8::from_str_radix(&text[2 * index..2 * index + 2], 16).unwrap())
        .collect()
}

/// A write of `value` to field 3 of row `id`, stamped by `node` at `wall`, as the record list's
/// `record_list.write` carries it: a 64-bit number as text, a narrower one as a number.
fn write(id: u32, node: u64, wall: i64, value: i64) -> String {
    json!({"id": id, "field": 3, "value": value.to_string(), "text": "",
           "wallTime": wall.to_string(), "counter": 0, "nodeId": node.to_string()})
    .to_string()
}

/// The note, as `record_list.supersede` carries it, that `by` wrote over the write of row `id`
/// that `node` stamped at `wall`, in the change stamped `at`.
fn supersede(id: u32, node: u64, wall: i64, by: u64, at: i64) -> String {
    json!({"id": id, "field": 3, "wallTime": wall.to_string(), "counter": 0,
           "nodeId": node.to_string(), "by": by.to_string(),
           "atWallTime": at.to_string(), "atCounter": 0})
    .to_string()
}

/// What alice's device keeps before the scenario begins: the genesis and the grants of alice
/// (administering) and bob (writing), which the administrator signed.
fn opening_save() -> String {
    let alice = Peer::open(LIST);
    let trust = alice.trust();
    trust.append(&genesis()).unwrap();
    for (node, role) in [(ALICE, 3u8), (BOB, 2)] {
        let entry = trust
            .draft(Body::Grant {
                node,
                role,
                key: person(node).public(),
                final_seq: None,
            })
            .sign(&administrator())
            .finish();
        trust.append(&entry).unwrap();
    }
    alice.save()
}

/// The devices of the scenario: each one's node, the seed of its person's key, and the hash of the
/// genesis it was invited with (none for the device that opens the list).
fn devices() -> Value {
    json!([
        {"node": ALICE, "seed": hex(&seed_of(ALICE)), "genesis": null},
        {"node": BOB, "seed": hex(&seed_of(BOB)), "genesis": hex(&genesis().hash())},
    ])
}

fn last_error() -> String {
    // SAFETY: the library returns a NUL-terminated text it owns.
    unsafe { CStr::from_ptr(sce_peer_last_error()) }
        .to_string_lossy()
        .into_owned()
}

fn check(status: i32, what: &str) {
    assert_eq!(status, SCE_PEER_OK, "{what}: {}", last_error());
}

fn empty() -> ScePeerBuf {
    ScePeerBuf {
        ptr: std::ptr::null_mut(),
        len: 0,
    }
}

/// The text of a buffer the library gave, which is given back; `None` for an empty buffer.
fn taken(buf: ScePeerBuf) -> Option<String> {
    if buf.ptr.is_null() {
        return None;
    }
    // SAFETY: a buffer the library gave is `len` readable bytes, and is given back once, here.
    let bytes = unsafe {
        let bytes = std::slice::from_raw_parts(buf.ptr, buf.len).to_vec();
        sce_peer_buf_free(buf);
        bytes
    };
    Some(String::from_utf8(bytes).expect("a text the library gives is UTF-8"))
}

fn text(value: &str) -> CString {
    CString::new(value).expect("a text of the scenario holds no NUL")
}

/// One device's core, reached through the C ABI as a lane reaches it.
struct Core(*mut ScePeer);

impl Core {
    /// The core of `device` (an entry of [`devices`]), made and given its person.
    fn make(device: &Value) -> Self {
        let list = text(LIST);
        let seed = unhex(device["seed"].as_str().unwrap());
        // SAFETY: the list is NUL-terminated, the genesis and the seed are 32 bytes.
        let peer = unsafe {
            match device["genesis"].as_str() {
                None => sce_peer_open(list.as_ptr()),
                Some(genesis) => {
                    let genesis = unhex(genesis);
                    sce_peer_invited(list.as_ptr(), genesis.as_ptr())
                }
            }
        };
        assert!(!peer.is_null(), "{}", last_error());
        let node = device["node"].as_u64().unwrap();
        // SAFETY: a live handle, and a seed of 32 bytes.
        check(
            unsafe { sce_peer_set_identity(peer, node, seed.as_ptr()) },
            "set_identity",
        );
        Core(peer)
    }

    /// What the core did with a send of the machine, and why when it refused it.
    fn outbound(&mut self, event: &str, data: &str) -> (String, String) {
        let (event, data) = (text(event), text(data));
        let mut kept = -1;
        let mut reason = empty();
        // SAFETY: a live handle, NUL-terminated texts, and places valid for a write.
        check(
            unsafe {
                sce_peer_outbound(
                    self.0,
                    event.as_ptr(),
                    data.as_ptr(),
                    &mut kept,
                    &mut reason,
                )
            },
            "outbound",
        );
        let answer = match kept {
            SCE_PEER_KEPT_TAKEN => "taken",
            SCE_PEER_KEPT_HELD => "held",
            SCE_PEER_KEPT_NOT_SIGNED => "not_signed",
            SCE_PEER_KEPT_REFUSED => "refused",
            other => panic!("{other} is no answer to a send"),
        };
        (answer.to_string(), taken(reason).unwrap_or_default())
    }

    fn start(&mut self, kind: &str) {
        let kind = match kind {
            "sync" => SCE_PEER_ROUND_SYNC,
            "resync" => SCE_PEER_ROUND_RESYNC,
            "reread" => SCE_PEER_ROUND_REREAD,
            other => panic!("{other} is no kind of round"),
        };
        // SAFETY: a live handle.
        check(unsafe { sce_peer_start(self.0, kind) }, "start");
    }

    fn poll(&mut self) -> Option<String> {
        let mut out = empty();
        // SAFETY: a live handle, and a place valid for a write.
        check(
            unsafe { sce_peer_poll_request(self.0, &mut out) },
            "poll_request",
        );
        taken(out)
    }

    fn respond(&mut self, response: &str) {
        let response = text(response);
        // SAFETY: a live handle, and a NUL-terminated text.
        check(
            unsafe { sce_peer_respond(self.0, response.as_ptr()) },
            "respond",
        );
    }

    fn fail(&mut self, reason: &str) {
        let reason = text(reason);
        // SAFETY: a live handle, and a NUL-terminated text.
        check(unsafe { sce_peer_fail(self.0, reason.as_ptr()) }, "fail");
    }

    /// The outcome of the exchange that is over: the core asks nothing more, and gives it once.
    fn outcome(&mut self) -> String {
        assert_eq!(self.poll(), None, "the exchange asks nothing more");
        let mut out = empty();
        // SAFETY: a live handle, and a place valid for a write.
        check(
            unsafe { sce_peer_take_outcome(self.0, &mut out) },
            "take_outcome",
        );
        taken(out).expect("an exchange ended")
    }

    fn save(&mut self) -> String {
        let mut out = empty();
        // SAFETY: a live handle, and a place valid for a write.
        check(unsafe { sce_peer_save(self.0, &mut out) }, "save");
        taken(out).expect("a save is a text")
    }

    fn restore(&mut self, saved: &str) -> u64 {
        let saved = text(saved);
        let mut count = u64::MAX;
        // SAFETY: a live handle, a NUL-terminated text, and a place valid for a write.
        check(
            unsafe { sce_peer_restore(self.0, saved.as_ptr(), &mut count) },
            "restore",
        );
        count
    }
}

impl Drop for Core {
    fn drop(&mut self) {
        // SAFETY: a handle this test made and frees once.
        unsafe { sce_peer_free(self.0) };
    }
}

/// The devices' cores run against the relay, each call and each text recorded as a step.
struct Recording {
    relay: Relay<MemoryStore>,
    authority: Authority,
    /// The relay's clock, in milliseconds: a second further at each request, so that no device
    /// spends its allowance however long the scenario.
    now: u64,
    devices: Value,
    cores: Vec<Option<Core>>,
    steps: Vec<Value>,
}

impl Recording {
    fn new() -> Self {
        let authority = Authority::new([3; 32], "the-relay");
        let devices = devices();
        let cores = devices.as_array().unwrap().iter().map(|_| None).collect();
        Self {
            relay: Relay::open(MemoryStore::new(), authority.clone(), Limits::default()).unwrap(),
            authority,
            now: 0,
            devices,
            cores,
            steps: Vec::new(),
        }
    }

    fn core(&mut self, device: usize) -> &mut Core {
        self.cores[device]
            .as_mut()
            .expect("the device's core is made")
    }

    /// The core of `device` made again (the device started), with its person.
    fn make(&mut self, device: usize) {
        self.cores[device] = None;
        self.cores[device] = Some(Core::make(&self.devices[device]));
        self.steps.push(json!({"device": device, "op": "make"}));
    }

    fn restore(&mut self, device: usize, saved: &str) -> u64 {
        let count = self.core(device).restore(saved);
        self.steps
            .push(json!({"device": device, "op": "restore", "text": saved, "count": count}));
        count
    }

    fn outbound(&mut self, device: usize, event: &str, data: &str) -> String {
        let (kept, reason) = self.core(device).outbound(event, data);
        self.steps
            .push(json!({"device": device, "op": "outbound", "event": event,
                               "data": data, "kept": kept, "reason": reason}));
        kept
    }

    fn start(&mut self, device: usize, kind: &str) {
        self.core(device).start(kind);
        self.steps
            .push(json!({"device": device, "op": "start", "kind": kind}));
    }

    /// The next request of `device`'s exchange, recorded when there is one.
    fn ask(&mut self, device: usize) -> Option<String> {
        let request = self.core(device).poll()?;
        self.steps
            .push(json!({"device": device, "op": "request", "text": request}));
        Some(request)
    }

    /// `request` carried to the relay, as `device`'s transport carries it, and the answer back.
    fn carry(&mut self, device: usize, request: &str) {
        let node = self.devices[device]["node"].as_u64().unwrap();
        let token = self.authority.issue(LIST, node, u64::MAX);
        self.now += 1000;
        let response = self.relay.handle(&token, request, self.now);
        self.core(device).respond(&response);
        self.steps
            .push(json!({"device": device, "op": "respond", "text": response}));
    }

    fn fail(&mut self, device: usize, reason: &str) {
        self.core(device).fail(reason);
        self.steps
            .push(json!({"device": device, "op": "fail", "reason": reason}));
    }

    fn outcome(&mut self, device: usize) -> Value {
        let outcome = self.core(device).outcome();
        self.steps
            .push(json!({"device": device, "op": "outcome", "text": outcome}));
        serde_json::from_str(&outcome).expect("an outcome is JSON")
    }

    /// A round of `kind` of `device`, every request carried to the relay and every answer back.
    fn round(&mut self, device: usize, kind: &str) -> Value {
        self.start(device, kind);
        while let Some(request) = self.ask(device) {
            self.carry(device, &request);
        }
        self.outcome(device)
    }

    fn save(&mut self, device: usize) -> String {
        let saved = self.core(device).save();
        self.steps
            .push(json!({"device": device, "op": "save", "text": saved}));
        saved
    }

    fn into_scenario(self) -> Value {
        json!({
            "version": 1,
            "list": LIST,
            "devices": self.devices,
            "steps": self.steps,
        })
    }
}

/// How many of an outcome's events for the machine are named `event`.
fn named(outcome: &Value, event: &str) -> usize {
    outcome["events"]
        .as_array()
        .expect("an ended exchange tells events")
        .iter()
        .filter(|pair| pair[0] == event)
        .count()
}

fn refused(outcome: &Value) -> usize {
    outcome["refused"]
        .as_array()
        .expect("an ended exchange tells refusals")
        .len()
}

/// The scenario, run now against the relay.
fn record() -> Value {
    let mut run = Recording::new();

    // Alice's device opens the list from what it keeps: the genesis and the grants. Restored, it
    // signs nothing new before a round has read its own chain back from the relay.
    run.make(AT_ALICE);
    assert_eq!(run.restore(AT_ALICE, &opening_save()), 0);
    assert_eq!(
        run.outbound(AT_ALICE, WRITE, &write(9, ALICE, 500, 1)),
        "refused",
        "a restored device signs nothing new before its chain is read back"
    );
    let opened = run.round(AT_ALICE, "sync");
    assert_eq!(opened["outcome"], "ended", "{opened}");
    assert_eq!(refused(&opened), 0, "{opened}");
    assert_eq!(opened["pushed"], 3, "the genesis and two grants: {opened}");

    // Bob's device, invited with the hash of the genesis, reads the log.
    run.make(AT_BOB);
    let joined = run.round(AT_BOB, "sync");
    assert_eq!(refused(&joined), 0, "{joined}");

    // Bob writes a field, writes it again, and notes that the second write is over the first; his
    // machine also sends an event that is not signed, and a write under a stamp he signed with
    // another value.
    assert_eq!(
        run.outbound(AT_BOB, WRITE, &write(1, BOB, 1000, 10)),
        "taken"
    );
    assert_eq!(
        run.outbound(AT_BOB, WRITE, &write(1, BOB, 2000, 20)),
        "taken"
    );
    assert_eq!(
        run.outbound(AT_BOB, SUPERSEDE, &supersede(1, BOB, 1000, BOB, 2000)),
        "taken"
    );
    assert_eq!(
        run.outbound(AT_BOB, PROPOSE, r#"{"before":"30000"}"#),
        "not_signed"
    );
    assert_eq!(
        run.outbound(AT_BOB, WRITE, &write(1, BOB, 1000, 11)),
        "refused",
        "another content under a stamp bob signed"
    );
    let pushed = run.round(AT_BOB, "sync");
    assert_eq!(pushed["pushed"], 3, "{pushed}");
    let told = run.round(AT_ALICE, "sync");
    assert_eq!(named(&told, WRITE), 2, "{told}");
    assert_eq!(named(&told, SUPERSEDE), 1, "{told}");

    // Bob's device is saved, stopped and started again from its save, and his machine, built again
    // from his application's own log, sends his writes again: what the core holds comes back held,
    // a new write waits until a round has read his chain back, and then is taken.
    let saved = run.save(AT_BOB);
    run.make(AT_BOB);
    assert_eq!(run.restore(AT_BOB, &saved), 3, "his three operations");
    assert_eq!(
        run.outbound(AT_BOB, WRITE, &write(1, BOB, 1000, 10)),
        "held"
    );
    assert_eq!(
        run.outbound(AT_BOB, WRITE, &write(2, BOB, 3000, 30)),
        "refused",
        "nothing new before his chain is read back"
    );
    let back = run.round(AT_BOB, "sync");
    assert_eq!(refused(&back), 0, "{back}");
    assert_eq!(back["pushed"], 0, "{back}");
    assert_eq!(
        run.outbound(AT_BOB, WRITE, &write(1, BOB, 2000, 20)),
        "held"
    );
    assert_eq!(
        run.outbound(AT_BOB, SUPERSEDE, &supersede(1, BOB, 1000, BOB, 2000)),
        "held"
    );
    assert_eq!(
        run.outbound(AT_BOB, WRITE, &write(2, BOB, 3000, 30)),
        "taken"
    );
    let rebuilt = run.round(AT_BOB, "resync");
    assert_eq!(refused(&rebuilt), 0, "{rebuilt}");
    assert_eq!(rebuilt["pushed"], 1, "the one new write: {rebuilt}");

    // Alice's transport fails in the middle of a round; the round ends, and the next goes on.
    run.start(AT_ALICE, "sync");
    assert!(run.ask(AT_ALICE).is_some(), "the round asks");
    run.fail(AT_ALICE, "the connection closed");
    let failed = run.outcome(AT_ALICE);
    assert_eq!(failed["outcome"], "failed", "{failed}");
    assert_eq!(failed["failure"], "link", "{failed}");
    let told = run.round(AT_ALICE, "sync");
    assert_eq!(named(&told, WRITE), 1, "bob's new write: {told}");
    assert_eq!(refused(&told), 0, "{told}");

    run.into_scenario()
}

/// What a lane does with the scenario, done here through the ABI from Rust: the same calls, with no
/// relay, every text the cores give compared with the recording. The number of steps replayed.
fn replay(scenario: &Value) -> usize {
    let devices = scenario["devices"].as_array().unwrap();
    let mut cores: Vec<Option<Core>> = devices.iter().map(|_| None).collect();
    let steps = scenario["steps"].as_array().unwrap();
    for (at, step) in steps.iter().enumerate() {
        let device = usize::try_from(step["device"].as_u64().unwrap()).unwrap();
        let field = |name: &str| -> String {
            step[name]
                .as_str()
                .unwrap_or_else(|| panic!("step {at} has no `{name}`"))
                .to_string()
        };
        if step["op"] == "make" {
            cores[device] = None;
            cores[device] = Some(Core::make(&devices[device]));
            continue;
        }
        let core = cores[device]
            .as_mut()
            .unwrap_or_else(|| panic!("step {at}: the device's core is not made"));
        match step["op"].as_str().unwrap() {
            "restore" => assert_eq!(
                core.restore(&field("text")),
                step["count"].as_u64().unwrap(),
                "step {at}"
            ),
            "outbound" => assert_eq!(
                core.outbound(&field("event"), &field("data")),
                (field("kept"), field("reason")),
                "step {at}"
            ),
            "start" => core.start(&field("kind")),
            "request" => assert_eq!(core.poll(), Some(field("text")), "step {at}"),
            "respond" => core.respond(&field("text")),
            "fail" => core.fail(&field("reason")),
            "outcome" => assert_eq!(core.outcome(), field("text"), "step {at}"),
            "save" => assert_eq!(core.save(), field("text"), "step {at}"),
            other => panic!("step {at}: `{other}` is no step of the scenario"),
        }
    }
    steps.len()
}

fn recorded() -> Value {
    let text = std::fs::read_to_string(PATH)
        .unwrap_or_else(|_| panic!("{PATH} is not recorded: {WRITE_AGAIN}"));
    serde_json::from_str(&text).expect("the recording is JSON")
}

/// The recording is what a run records now, step by step; the first step that differs is named.
#[test]
fn the_recorded_scenario_is_what_a_fresh_run_records() {
    let fresh = record();
    if std::env::var("SCE_PEER_SCENARIO").as_deref() == Ok("write") {
        std::fs::create_dir_all(Path::new(PATH).parent().unwrap()).unwrap();
        std::fs::write(PATH, serde_json::to_string_pretty(&fresh).unwrap() + "\n").unwrap();
    }
    let recorded = recorded();
    let (was, now) = (
        recorded["steps"].as_array().unwrap(),
        fresh["steps"].as_array().unwrap(),
    );
    if let Some(at) = (0..was.len().max(now.len())).find(|at| was.get(*at) != now.get(*at)) {
        panic!(
            "the recording and a fresh run differ at step {at}:\nrecorded: {}\nnow:      {}\n\
             record it again with {WRITE_AGAIN}",
            was.get(at)
                .map_or_else(|| "nothing".to_string(), Value::to_string),
            now.get(at)
                .map_or_else(|| "nothing".to_string(), Value::to_string),
        );
    }
    assert_eq!(
        recorded, fresh,
        "the recording's devices or list differ: {WRITE_AGAIN}"
    );
}

#[test]
fn the_recorded_scenario_replays_through_the_abi_from_rust() {
    let recorded = recorded();
    assert_eq!(
        replay(&recorded),
        recorded["steps"].as_array().unwrap().len()
    );
}

/// The scenario recorded now replays too, so that a recording the library can no longer replay is
/// told from a replay that is wrong.
#[test]
fn a_scenario_recorded_now_replays_through_the_abi() {
    let scenario = record();
    assert!(replay(&scenario) > 0);
}
