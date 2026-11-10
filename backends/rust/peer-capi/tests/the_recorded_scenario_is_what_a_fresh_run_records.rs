// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
//
// The recorded scenario every lane replays through the peer core's C ABI
// (`tests/fixtures/two_devices_and_a_relay.json`). Two devices of one list run through the ABI
// against the real relay in this process: alice's, which holds the administrator's key, opens the
// list, invites bob's device by the text of a QR code (ADR 0080 section 4) and grants writing to the
// key his device proved when it joined under the invitation, the two devices showing one
// fingerprint; and bob's, made from that text alone, which holds the list, the genesis alice's core
// gave, his node and the relay's address. The invitation's secret is fixed here, as the data key and
// the relay's challenge key are.
// Every call each device's core is given and every text it gives back is recorded in order: the
// administrator's acts and the genesis the opening gave, the machine's sends and what the core did
// with each (taken, held, not signed, kept waiting and why, refused and why), each request the core
// puts to the relay and the relay's answer, a failure of the transport, the outcome each machine is
// told, what each device stands at, and bob's sealed state, after which his core is made again given
// nothing but the list and the genesis, and restored from it. What it shows: a list is opened and a
// device admitted through the ABI alone; a change bob's machine makes before his device has read the
// log that lets him write is kept, once, and his first round signs it and gives it the relay; bob's
// changes reach alice's machine whole, a write and its write-over as one (docs/adr/0085, section
// 1); a device restored from its sealed state needs no key given again, says it waits for its own
// chain, keeps a new change until the chain is read back and then signs it, and gives a change it
// holds as held; another content under a dot a device signed is refused; a device started again
// from an older state whose copy keeps a journal offers two changes that wait, the round reads its
// chain back, says its own stamp rose, and withdraws the first, which goes back along the chain,
// with the one after it; a copy that keeps a journal has the change it offers signed in the call,
// refused when it is in another's name, and held back from the relay as a record until its host
// recorded it, and a device started again from its state and its journal signs nothing again, is
// handed its history in causal order with a change that waits as provisional, and has that change
// signed and held back as a record by the round that reads its chain back (docs/adr/0083 and 0093);
// a device that wrote nothing since it was told more marks it, and the stable cut moves by the mark
// (docs/adr/0085); a round that loses its
// transport ends, and the next goes on; and a device taken off is
// refused by the relay, so that what it writes afterwards reaches no one. Bob's device is handed no
// token: it logs in at the relay with his person's key (ADR 0080), over a connection that presents
// none, and the relay issues the token his rounds then present; once taken off, his device is
// refused a token. Alice's device, which gives the relay the list's first entries, presents a token
// of the relay's authority, as a relay whose creation of lists is closed takes it.
//
// The core does no I/O, so a lane replays the scenario with no relay: it makes the same calls,
// compares each request, outcome, standing, genesis and sealed state the core gives with the
// recording, and gives the core the recorded answers. Everything in it is deterministic (Ed25519
// signs deterministically, a state is sealed under a data key the recording fixes and under a nonce
// that is a keyed hash of what it seals, and nothing here reads a clock or draws a random number),
// so a run now records the same scenario, and a test fails when the recording and a fresh run
// differ: the file cannot drift from the library. The list's MLS group is not in it, since MLS draws randomness and a run
// would not record the same texts twice; the group's own C ABI has its own recorded scenario
// (`backends/rust/mls/tests/fixtures/join_and_remove.json`), and the administrator's acts on a group
// through this ABI are held by `the_administrators_acts_on_a_group_go_through_the_abi`.
//
// The scenario is recorded again with `SCE_PEER_SCENARIO=write cargo test -p sce-rust-peer-capi
// --test the_recorded_scenario_is_what_a_fresh_run_records -- the_recorded_scenario_is_what`.

#![deny(unsafe_op_in_unsafe_fn)]

use std::ffi::{CStr, CString};
use std::path::Path;
use std::sync::Mutex;

use sce_peer_capi::*;
use sce_rust_relay::{Authority, Doorway, Hub, Limits, LoginPolicy, MemoryStore, Relay};
use sce_rust_trust::membership::{Body, Entry, Log, Profile};
use sce_rust_trust::Identity;
use serde_json::{json, Value};

const LIST: &str = "home";
const ALICE: u64 = 1;
const BOB: u64 = 2;
/// Where each device is in the recording's list of devices.
const AT_ALICE: usize = 0;
const AT_BOB: usize = 1;

const CHANGE: &str = "record_list.change";
/// A command of the list's own, which no host signs.
const EDIT: &str = "record_list.edit";

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

/// The seed of the administrator's key, which alice's device holds.
const ADMINISTRATOR: [u8; 32] = [9; 32];

/// The data key bob's state is sealed under: fixed here, so that the recording holds the same
/// sealed bytes at every run. A host draws one for each save (`sce_peer_save_sealed`).
const DATA_KEY: [u8; 32] = [7; 32];

/// The relay's name, which a device is configured with and logs in for.
const AUDIENCE: &str = "the-relay";

/// The key the relay's challenges are made under: fixed here, so that the recording holds the same
/// nonces at every run. A relay draws one at start (`Doorway::new`).
const CHALLENGE_KEY: [u8; 32] = [5; 32];

/// The secret of alice's invitation of bob: fixed here, so that the recording holds the same text at
/// every run. An administrator's device draws one for each invitation (`sce_peer_invite`).
const INVITATION_SECRET: [u8; 32] = [6; 32];

/// The relay's address the invitation carries; nothing dials it, since the recording carries the
/// texts itself.
const RELAY_ADDRESS: &str = "wss://relay.example";

/// When alice's device makes the invitation by its host's clock, and how long it lives: a day.
const INVITED_AT_MS: u64 = 0;
const INVITATION_MS: u64 = 86_400_000;

/// The genesis a list with one administrator, a quorum of one and a hub that is not trusted has:
/// what alice's core is to write when it opens the list, made here by hand.
fn genesis() -> Entry {
    let administrator = Identity::from_seed(ADMINISTRATOR);
    Log::genesis_draft(
        LIST,
        Body::Genesis {
            administrators: vec![administrator.public()],
            quorum: 1,
            profile: Profile::Untrusted,
        },
    )
    .sign(&administrator)
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

/// A change of one write of `value` to field 3 of row `id`, stamped by `node` at `wall`, as the
/// record list's `record_list.change` carries it: a 64-bit number as text, a narrower one as a
/// number, and the write's own dot, which is its change's, 0.
fn write(id: u32, node: u64, wall: i64, value: i64) -> String {
    json!({"by": node.to_string(), "wallTime": wall.to_string(), "counter": 0, "ops": [
        {"kind": 0, "id": id, "field": 3, "value": value.to_string(), "text": "",
         "wallTime": "0", "counter": 0, "nodeId": "0"}]})
    .to_string()
}

/// The change of `by`'s stamped at `at` that writes over the write of row `id`, field 3, that
/// `node` stamped at `wall`, and writes the field anew with `value`: two operations, one change.
fn rewrite(id: u32, node: u64, wall: i64, by: u64, at: i64, value: i64) -> String {
    json!({"by": by.to_string(), "wallTime": at.to_string(), "counter": 0, "ops": [
        {"kind": 1, "id": id, "field": 3, "value": "0", "text": "",
         "wallTime": wall.to_string(), "counter": 0, "nodeId": node.to_string()},
        {"kind": 0, "id": id, "field": 3, "value": value.to_string(), "text": "",
         "wallTime": "0", "counter": 0, "nodeId": "0"}]})
    .to_string()
}

/// How many operations each change of `event` an exchange told the machine holds, in order.
fn ops_of_each(outcome: &Value, event: &str) -> Vec<usize> {
    outcome["events"]
        .as_array()
        .expect("an ended exchange tells events")
        .iter()
        .filter(|pair| pair[0] == event)
        .map(|pair| {
            let data: Value =
                serde_json::from_str(pair[1].as_str().expect("an event's data is a text"))
                    .expect("an event's data is JSON");
            data["ops"].as_array().map_or(0, Vec::len)
        })
        .collect()
}

/// The devices of the scenario: each one's node, the seed of its person's key, the hash of the
/// genesis it was invited with (none for the device that opens the list), and the seed of the
/// administrator's key it holds (none for a device that holds none).
fn devices() -> Value {
    json!([
        {"node": ALICE, "seed": hex(&seed_of(ALICE)), "genesis": null,
         "administrator": hex(&ADMINISTRATOR)},
        {"node": BOB, "seed": hex(&seed_of(BOB)), "genesis": hex(&genesis().hash()),
         "administrator": null},
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

/// The word a recording names what the core did with a send or an offer by.
fn kept_word(kept: i32) -> String {
    match kept {
        SCE_PEER_KEPT_TAKEN => "taken",
        SCE_PEER_KEPT_HELD => "held",
        SCE_PEER_KEPT_NOT_SIGNED => "not_signed",
        SCE_PEER_KEPT_REFUSED => "refused",
        SCE_PEER_KEPT_WAITING => "waiting",
        other => panic!("{other} is no answer to a send"),
    }
    .to_string()
}

/// What a copy that keeps a journal offers its host for a change of one write of `value` to field
/// 3 of row `id`, stamped by `node` at `wall` and closed by the command `command`:
/// `record_list.made`'s payload.
fn made(id: u32, node: u64, wall: i64, value: i64, command: u64) -> String {
    json!({"by": node.to_string(), "wallTime": wall.to_string(), "counter": 0, "ops": [
        {"kind": 0, "id": id, "field": 3, "value": value.to_string(), "text": "",
         "wallTime": "0", "counter": 0, "nodeId": "0"}],
        "command": command.to_string()})
    .to_string()
}

/// One device's core, reached through the C ABI as a lane reaches it.
struct Core(*mut ScePeer);

impl Core {
    /// The core of `device` (an entry of [`devices`]) given nothing but the list, and the genesis
    /// it was invited with: what a sealed state is restored into.
    fn bare(device: &Value) -> Self {
        let list = text(LIST);
        // SAFETY: the list is NUL-terminated, the genesis is 32 bytes.
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
        Core(peer)
    }

    /// The core of `device` (an entry of [`devices`]), made and given its person, and the
    /// administrator's key when it holds one.
    fn make(device: &Value) -> Self {
        Self::bare(device).given(device)
    }

    /// The core of the device the invitation `invitation` invites, made from its text alone and
    /// given the person of `device` (an entry of [`devices`]).
    fn make_invited(device: &Value, invitation: &str) -> Self {
        // SAFETY: the text for its length.
        let peer = unsafe { sce_peer_invited_by(invitation.as_ptr(), invitation.len()) };
        assert!(!peer.is_null(), "{}", last_error());
        Core(peer).given(device)
    }

    /// This core given the person of `device`, and its administrator's key when it holds one.
    fn given(self, device: &Value) -> Self {
        let core = self;
        let seed = unhex(device["seed"].as_str().unwrap());
        let node = device["node"].as_u64().unwrap();
        // SAFETY: a live handle, and a seed of 32 bytes.
        check(
            unsafe { sce_peer_set_identity(core.0, node, seed.as_ptr()) },
            "set_identity",
        );
        if let Some(administrator) = device["administrator"].as_str() {
            let administrator = unhex(administrator);
            // SAFETY: a live handle, and a seed of 32 bytes.
            check(
                unsafe { sce_peer_set_administrator(core.0, administrator.as_ptr()) },
                "set_administrator",
            );
        }
        core
    }

    /// The list opened with the hub's profile `profile`, and the hash of its genesis, in hex.
    fn open(&mut self, profile: &str) -> String {
        let profile = match profile {
            "untrusted" => SCE_PEER_PROFILE_UNTRUSTED,
            "trusted_hub" => SCE_PEER_PROFILE_TRUSTED_HUB,
            other => panic!("{other} is no profile"),
        };
        let mut genesis = [0u8; 32];
        // SAFETY: a live handle, and a place valid for a write of 32 bytes.
        check(
            unsafe { sce_peer_open_list(self.0, profile, genesis.as_mut_ptr()) },
            "open_list",
        );
        hex(&genesis)
    }

    /// The device of `node`, which signs with the key `key` (in hex), granted `role`.
    fn grant(&mut self, node: u64, role: u32, key: &str) {
        let key = unhex(key);
        // SAFETY: a live handle, and a key of 32 bytes.
        check(
            unsafe { sce_peer_grant(self.0, node, role, key.as_ptr()) },
            "grant",
        );
    }

    fn remove(&mut self, node: u64) {
        // SAFETY: a live handle.
        check(
            unsafe { sce_peer_remove_member(self.0, node) },
            "remove_member",
        );
    }

    fn standing(&mut self) -> String {
        let mut out = empty();
        // SAFETY: a live handle, and a place valid for a write.
        check(unsafe { sce_peer_standing(self.0, &mut out) }, "standing");
        taken(out).expect("a standing is a text")
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
        (kept_word(kept), taken(reason).unwrap_or_default())
    }

    /// The verdict the core gave a change the copy offered, and why when it refused it or keeps it
    /// waiting.
    fn made(&mut self, data: &str) -> (String, String) {
        let data = text(data);
        let mut kept = -1;
        let mut reason = empty();
        // SAFETY: a live handle, a NUL-terminated text, and places valid for a write.
        check(
            unsafe { sce_peer_made(self.0, data.as_ptr(), &mut kept, &mut reason) },
            "made",
        );
        (kept_word(kept), taken(reason).unwrap_or_default())
    }

    /// The records the host has not recorded, as the ABI's JSON text.
    fn records(&mut self) -> String {
        let mut out = empty();
        // SAFETY: a live handle, and a place valid for a write.
        check(
            unsafe { sce_peer_take_records(self.0, &mut out) },
            "take_records",
        );
        taken(out).expect("the records are a text")
    }

    fn recorded(&mut self, through: u64) {
        // SAFETY: a live handle.
        check(unsafe { sce_peer_recorded(self.0, through) }, "recorded");
    }

    /// The records of a journal given back, and how many the core did not hold.
    fn restore_records(&mut self, records: &str) -> u64 {
        let records = text(records);
        let mut count = u64::MAX;
        // SAFETY: a live handle, a NUL-terminated text, and a place valid for a write.
        check(
            unsafe { sce_peer_restore_records(self.0, records.as_ptr(), &mut count) },
            "restore_records",
        );
        count
    }

    /// The whole history, as the ABI's JSON text.
    fn history(&mut self) -> String {
        let mut out = empty();
        // SAFETY: a live handle, no cut, and a place valid for a write.
        check(
            unsafe { sce_peer_history(self.0, std::ptr::null(), &mut out) },
            "history",
        );
        taken(out).expect("a history is a text")
    }

    /// The stable cut, as the ABI's JSON text.
    fn stable_cut(&mut self) -> String {
        let mut out = empty();
        // SAFETY: a live handle, and a place valid for a write.
        check(
            unsafe { sce_peer_stable_cut(self.0, &mut out) },
            "stable_cut",
        );
        taken(out).expect("a cut is a text")
    }

    /// The number of the mark the core signed, 0 when none was owed.
    fn mark(&mut self) -> u64 {
        let mut seq = u64::MAX;
        // SAFETY: a live handle, and a place valid for a write.
        check(unsafe { sce_peer_mark(self.0, &mut seq) }, "mark");
        seq
    }

    /// The highest stamp of the device's own changes, and whether its chain was read back.
    fn own_stamp(&mut self) -> (i64, u32, i32) {
        let (mut wall_time, mut counter, mut read_back) = (-1i64, u32::MAX, -1i32);
        // SAFETY: a live handle, and places valid for a write.
        check(
            unsafe { sce_peer_own_stamp(self.0, &mut wall_time, &mut counter, &mut read_back) },
            "own_stamp",
        );
        (wall_time, counter, read_back)
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

    /// Begin the login at the relay named `audience`.
    fn login(&mut self, audience: &str) {
        let audience = text(audience);
        // SAFETY: a live handle, and a NUL-terminated text.
        check(
            unsafe { sce_peer_login(self.0, audience.as_ptr()) },
            "login",
        );
    }

    /// The administrator's invitation of the device of `node`, under `secret` (in hex), as the
    /// text a QR code holds.
    fn invite(
        &mut self,
        relay: &str,
        audience: &str,
        node: u64,
        now_ms: u64,
        valid_ms: u64,
        secret: &str,
    ) -> String {
        let (relay, audience, secret) = (text(relay), text(audience), unhex(secret));
        let mut out = empty();
        // SAFETY: a live handle, NUL-terminated texts, 32 bytes of secret, a place valid for a write.
        check(
            unsafe {
                sce_peer_invite_under(
                    self.0,
                    relay.as_ptr(),
                    audience.as_ptr(),
                    node,
                    now_ms,
                    valid_ms,
                    secret.as_ptr(),
                    &mut out,
                )
            },
            "invite_under",
        );
        taken(out).expect("an invitation is a text")
    }

    /// What the invitation `invitation` says, as the ABI's JSON text.
    fn invitation(invitation: &str) -> String {
        let mut out = empty();
        // SAFETY: the text for its length, a place valid for a write.
        check(
            unsafe { sce_peer_invitation_read(invitation.as_ptr(), invitation.len(), &mut out) },
            "invitation_read",
        );
        taken(out).expect("what an invitation says is a text")
    }

    /// Begin the join under `invitation`, and the fingerprint the device shows.
    fn join(&mut self, invitation: &str) -> String {
        let mut fingerprint = empty();
        // SAFETY: a live handle, the text for its length, a place valid for a write.
        check(
            unsafe {
                sce_peer_join(
                    self.0,
                    invitation.as_ptr(),
                    invitation.len(),
                    &mut fingerprint,
                )
            },
            "join",
        );
        taken(fingerprint).expect("a fingerprint is a text")
    }

    /// The devices that joined under this core's invitation of `node`, empty when none.
    fn joined(&mut self, node: u64) -> String {
        let mut out = empty();
        // SAFETY: a live handle, a place valid for a write.
        check(unsafe { sce_peer_joined(self.0, node, &mut out) }, "joined");
        taken(out).unwrap_or_default()
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

    /// What the device keeps, sealed under the data key `key` (in hex), as hex.
    fn save(&mut self, key: &str) -> String {
        let key = unhex(key);
        let mut out = empty();
        // SAFETY: a live handle, a key of 32 bytes, and a place valid for a write.
        check(
            unsafe { sce_peer_save_sealed_under(self.0, key.as_ptr(), &mut out) },
            "save_sealed_under",
        );
        assert!(!out.ptr.is_null(), "a sealed state is not empty");
        // SAFETY: a buffer the library gave is `len` readable bytes, and is given back once, here.
        let state = unsafe {
            let state = std::slice::from_raw_parts(out.ptr, out.len).to_vec();
            sce_peer_buf_free(out);
            state
        };
        hex(&state)
    }

    /// The state sealed under `key` (both in hex) for the device of `node`, taken back.
    fn restore(&mut self, node: u64, key: &str, state: &str) -> u64 {
        let (key, state) = (unhex(key), unhex(state));
        let mut count = u64::MAX;
        // SAFETY: a live handle, a state of its length, a key of 32 bytes, and a place valid for
        // a write.
        check(
            unsafe {
                sce_peer_restore_sealed(
                    self.0,
                    node,
                    state.as_ptr(),
                    state.len(),
                    key.as_ptr(),
                    &mut count,
                )
            },
            "restore_sealed",
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
    /// The relay, offering login, reached as a transport reaches it ([`Hub::answer`]).
    relay: Mutex<Relay<MemoryStore>>,
    /// The relay's clock, in milliseconds: a second further at each request, so that no device
    /// spends its allowance however long the scenario.
    now: u64,
    devices: Value,
    cores: Vec<Option<Core>>,
    /// The token each device's transport presents, none for a device that has not logged in.
    tokens: Vec<Option<String>>,
    steps: Vec<Value>,
}

/// The relay's authority, whose tokens the relay takes and issues.
fn authority() -> Authority {
    Authority::new([3; 32], AUDIENCE)
}

impl Recording {
    fn new() -> Self {
        let devices = devices();
        let cores = devices.as_array().unwrap().iter().map(|_| None).collect();
        let tokens = devices.as_array().unwrap().iter().map(|_| None).collect();
        let relay = Relay::open(MemoryStore::new(), authority(), Limits::default())
            .unwrap()
            .with_login(Doorway::with_key(LoginPolicy::default(), CHALLENGE_KEY));
        Self {
            relay: Mutex::new(relay),
            now: 0,
            devices,
            cores,
            tokens,
            steps: Vec::new(),
        }
    }

    /// The device's transport presents a token of the relay's authority, as an operator issues it:
    /// what gives a relay whose creation of lists is closed a list's first entries.
    fn hand_token(&mut self, device: usize) {
        let node = self.devices[device]["node"].as_u64().unwrap();
        self.tokens[device] = Some(authority().issue(LIST, node, u64::MAX));
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

    /// The core of `device` made from the invitation's text alone (the device started from its QR
    /// code), with its person.
    fn make_invited(&mut self, device: usize, invitation: &str) {
        self.cores[device] = None;
        self.cores[device] = Some(Core::make_invited(&self.devices[device], invitation));
        self.steps
            .push(json!({"device": device, "op": "make_invited", "invitation": invitation}));
    }

    /// `device`'s invitation of the device of `node`, under the recording's fixed secret.
    fn invite(&mut self, device: usize, node: u64) -> String {
        let secret = hex(&INVITATION_SECRET);
        let invitation = self.core(device).invite(
            RELAY_ADDRESS,
            AUDIENCE,
            node,
            INVITED_AT_MS,
            INVITATION_MS,
            &secret,
        );
        self.steps.push(
            json!({"device": device, "op": "invite", "relay": RELAY_ADDRESS,
                               "audience": AUDIENCE, "node": node, "now_ms": INVITED_AT_MS,
                               "valid_ms": INVITATION_MS, "secret": secret,
                               "text": invitation}),
        );
        invitation
    }

    /// What the invitation says, read by `device`'s host.
    fn invitation(&mut self, device: usize, invitation: &str) -> Value {
        let said = Core::invitation(invitation);
        self.steps.push(json!({"device": device, "op": "invitation",
                               "invitation": invitation, "text": said}));
        serde_json::from_str(&said).expect("what an invitation says is JSON")
    }

    /// `device` joins under `invitation`, every request carried and every answer back; the
    /// fingerprint it shows, and the outcome.
    fn join(&mut self, device: usize, invitation: &str) -> (String, Value) {
        let fingerprint = self.core(device).join(invitation);
        self.steps.push(
            json!({"device": device, "op": "join", "invitation": invitation,
                               "fingerprint": fingerprint}),
        );
        while let Some(request) = self.ask(device) {
            self.carry(device, &request);
        }
        (fingerprint, self.outcome(device))
    }

    /// What `device` shows of the devices that joined under its invitation of `node`.
    fn joined(&mut self, device: usize, node: u64) -> Value {
        let joined = self.core(device).joined(node);
        self.steps
            .push(json!({"device": device, "op": "joined", "node": node, "text": joined}));
        if joined.is_empty() {
            Value::Null
        } else {
            serde_json::from_str(&joined).expect("the devices that joined are JSON")
        }
    }

    fn open(&mut self, device: usize, profile: &str) -> String {
        let genesis = self.core(device).open(profile);
        self.steps
            .push(json!({"device": device, "op": "open", "profile": profile, "genesis": genesis}));
        genesis
    }

    fn grant(&mut self, device: usize, node: u64, role: u32, key: &[u8]) {
        let key = hex(key);
        self.core(device).grant(node, role, &key);
        self.steps
            .push(json!({"device": device, "op": "grant", "node": node, "role": role, "key": key}));
    }

    fn remove(&mut self, device: usize, node: u64) {
        self.core(device).remove(node);
        self.steps
            .push(json!({"device": device, "op": "remove", "node": node}));
    }

    fn standing(&mut self, device: usize) -> Value {
        let standing = self.core(device).standing();
        self.steps
            .push(json!({"device": device, "op": "standing", "text": standing}));
        serde_json::from_str(&standing).expect("a standing is JSON")
    }

    /// The core of `device` made again (the device started) given nothing but the list and the
    /// genesis it was invited with, and the state sealed under `key` (both in hex) taken back.
    fn restore(&mut self, device: usize, key: &str, state: &str) -> u64 {
        let node = self.devices[device]["node"].as_u64().unwrap();
        self.cores[device] = None;
        self.cores[device] = Some(Core::bare(&self.devices[device]));
        let count = self.core(device).restore(node, key, state);
        self.steps
            .push(json!({"device": device, "op": "restore", "key": key,
                               "state": state, "count": count}));
        count
    }

    fn outbound(&mut self, device: usize, event: &str, data: &str) -> String {
        let (kept, reason) = self.core(device).outbound(event, data);
        self.steps
            .push(json!({"device": device, "op": "outbound", "event": event,
                               "data": data, "kept": kept, "reason": reason}));
        kept
    }

    /// The verdict the core of `device` gives the change its copy offered.
    fn made(&mut self, device: usize, data: &str) -> String {
        let (kept, reason) = self.core(device).made(data);
        self.steps
            .push(json!({"device": device, "op": "made", "data": data,
                               "kept": kept, "reason": reason}));
        kept
    }

    /// The records `device`'s host has not recorded, as the ABI's text and as JSON.
    fn records(&mut self, device: usize) -> (String, Value) {
        let records = self.core(device).records();
        self.steps
            .push(json!({"device": device, "op": "records", "text": records}));
        let value = serde_json::from_str(&records).expect("the records are JSON");
        (records, value)
    }

    fn recorded(&mut self, device: usize, through: u64) {
        self.core(device).recorded(through);
        self.steps
            .push(json!({"device": device, "op": "recorded", "through": through}));
    }

    fn restore_records(&mut self, device: usize, records: &str) -> u64 {
        let count = self.core(device).restore_records(records);
        self.steps
            .push(json!({"device": device, "op": "restore_records",
                               "records": records, "count": count}));
        count
    }

    fn history(&mut self, device: usize) -> Value {
        let history = self.core(device).history();
        self.steps
            .push(json!({"device": device, "op": "history", "text": history}));
        serde_json::from_str(&history).expect("a history is JSON")
    }

    fn stable_cut(&mut self, device: usize) -> Value {
        let cut = self.core(device).stable_cut();
        self.steps
            .push(json!({"device": device, "op": "stable_cut", "text": cut}));
        serde_json::from_str(&cut).expect("a cut is JSON")
    }

    fn mark(&mut self, device: usize) -> u64 {
        let seq = self.core(device).mark();
        self.steps
            .push(json!({"device": device, "op": "mark", "seq": seq}));
        seq
    }

    fn own_stamp(&mut self, device: usize) -> (i64, u32, i32) {
        let (wall_time, counter, read_back) = self.core(device).own_stamp();
        self.steps.push(
            json!({"device": device, "op": "own_stamp", "wall_time": wall_time,
                               "counter": counter, "read_back": read_back}),
        );
        (wall_time, counter, read_back)
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

    /// `request` carried to the relay, as `device`'s transport carries it with the token it
    /// presents, or none, and the answer back.
    fn carry(&mut self, device: usize, request: &str) {
        self.now += 1000;
        let response = self
            .relay
            .answer(self.tokens[device].as_deref(), None, request, self.now);
        self.core(device).respond(&response);
        self.steps
            .push(json!({"device": device, "op": "respond", "text": response}));
    }

    /// `device` logs in at the relay, every request carried and every answer back; its transport
    /// presents the token the outcome holds from then on, when it holds one.
    fn login(&mut self, device: usize) -> Value {
        self.core(device).login(AUDIENCE);
        self.steps
            .push(json!({"device": device, "op": "login", "audience": AUDIENCE}));
        while let Some(request) = self.ask(device) {
            self.carry(device, &request);
        }
        let outcome = self.outcome(device);
        if let Some(token) = outcome["token"]["token"].as_str() {
            self.tokens[device] = Some(token.to_string());
        }
        outcome
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

    /// What `device` keeps, sealed under `key` (in hex), as hex.
    fn save(&mut self, device: usize, key: &str) -> String {
        let state = self.core(device).save(key);
        self.steps
            .push(json!({"device": device, "op": "save", "key": key, "state": state}));
        state
    }

    fn into_scenario(self) -> Value {
        json!({
            "version": 6,
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

/// The nodes a standing names as members, as texts.
fn members(standing: &Value) -> Vec<String> {
    standing["members"]
        .as_array()
        .expect("a standing names the members")
        .iter()
        .map(|member| member["node"].as_str().unwrap().to_string())
        .collect()
}

/// The scenario, run now against the relay.
fn record() -> Value {
    let mut run = Recording::new();

    // Alice's device opens the list as its administrator: the genesis is the one a list with her
    // administrator's key, a quorum of one and a hub that is not trusted has, which is what bob is
    // invited with, and her person is its first member, with the highest role. She invites bob's
    // device by the text of a QR code (ADR 0080 section 4), and her round gives the relay the
    // genesis and her grant, presenting a token of the relay's authority: a list is created with one.
    run.make(AT_ALICE);
    run.hand_token(AT_ALICE);
    assert_eq!(
        run.open(AT_ALICE, "untrusted"),
        hex(&genesis().hash()),
        "the genesis bob is invited with"
    );
    let opened = run.standing(AT_ALICE);
    assert_eq!(opened["administrator"], true, "{opened}");
    assert_eq!(opened["role"], 3, "{opened}");
    assert_eq!(opened["awaits_own_chain"], false, "{opened}");
    assert_eq!(members(&opened), vec!["1"], "{opened}");
    let invitation = run.invite(AT_ALICE, BOB);
    let given = run.round(AT_ALICE, "sync");
    assert_eq!(given["outcome"], "ended", "{given}");
    assert_eq!(refused(&given), 0, "{given}");
    assert_eq!(given["pushed"], 2, "the genesis and her grant: {given}");

    // Bob's device is made from the text alone: the list, the genesis, the relay and his node are
    // in it. It is used offline before it has read the log: his machine writes a field, and sends
    // the write again. His core cannot sign it yet and keeps it, once.
    run.make_invited(AT_BOB, &invitation);
    let said = run.invitation(AT_BOB, &invitation);
    assert_eq!(said["node"], BOB.to_string(), "{said}");
    assert_eq!(said["audience"], AUDIENCE, "{said}");
    assert_eq!(said["relay"], RELAY_ADDRESS, "{said}");
    assert_eq!(
        said["genesis"],
        sce_rust_trust::encoding::encode(&genesis().hash()),
        "{said}"
    );
    assert_eq!(
        run.outbound(AT_BOB, CHANGE, &write(1, BOB, 1000, 10)),
        "waiting"
    );
    assert_eq!(
        run.outbound(AT_BOB, CHANGE, &write(1, BOB, 1000, 10)),
        "waiting",
        "the same write waits once"
    );
    assert_eq!(run.standing(AT_BOB)["waiting"], 1);

    // Bob's device joins under the invitation over a connection with no token, leaving his key with
    // a proof made with the invitation's secret, and shows a fingerprint. Alice's next round reads
    // the join, her device shows the same digits for the two of them to compare, and she grants
    // the key the proof names; her round gives the relay the grant.
    let (shown, left) = run.join(AT_BOB, &invitation);
    assert_eq!(left["outcome"], "ended", "{left}");
    assert_eq!(
        left["events"],
        json!([]),
        "a join tells the machine nothing: {left}"
    );
    let read = run.round(AT_ALICE, "sync");
    assert_eq!(refused(&read), 0, "{read}");
    let devices = run.joined(AT_ALICE, BOB);
    assert_eq!(
        devices,
        json!([{"key": person(BOB).public().to_text(), "fingerprint": shown}]),
        "the fingerprint the same on both sides"
    );
    run.grant(AT_ALICE, BOB, 2, &person(BOB).public().to_bytes());
    let granted = run.standing(AT_ALICE);
    assert_eq!(members(&granted), vec!["1", "2"], "{granted}");
    assert_eq!(
        run.joined(AT_ALICE, BOB),
        Value::Null,
        "admitted: the invitation is done"
    );
    let given = run.round(AT_ALICE, "sync");
    assert_eq!(refused(&given), 0, "{given}");
    assert_eq!(given["pushed"], 1, "the grant: {given}");

    // Bob's device was handed no token: it logs in with his person's key, and the relay, which holds
    // the grant alice gave it, issues a token for an hour, which his rounds present from then on.
    // His first round reads the log that lets him write, signs the write that waited, and gives it
    // the relay with the rest of what it lacks.
    let logged_in = run.login(AT_BOB);
    assert_eq!(logged_in["outcome"], "ended", "{logged_in}");
    assert!(
        logged_in["token"]["token"].is_string(),
        "the outcome holds the token: {logged_in}"
    );
    assert_eq!(
        logged_in["events"].as_array().map(Vec::len),
        Some(0),
        "a login tells the machine nothing: {logged_in}"
    );
    let joined = run.round(AT_BOB, "sync");
    assert_eq!(refused(&joined), 0, "{joined}");
    assert_eq!(joined["pushed"], 1, "the write that waited: {joined}");
    assert_eq!(
        joined["taken"].as_array().map(Vec::len),
        Some(1),
        "{joined}"
    );
    assert_eq!(run.standing(AT_BOB)["waiting"], 0);

    // Bob writes the field again over his first write, one change of a write-over and a write; his
    // machine also sends an event that is not signed, and a change under a dot he signed with
    // another value.
    assert_eq!(
        run.outbound(AT_BOB, CHANGE, &rewrite(1, BOB, 1000, BOB, 2000, 20)),
        "taken"
    );
    assert_eq!(run.outbound(AT_BOB, EDIT, r#"{"id":1}"#), "not_signed");
    assert_eq!(
        run.outbound(AT_BOB, CHANGE, &write(1, BOB, 1000, 11)),
        "refused",
        "another content under a dot bob signed"
    );
    let pushed = run.round(AT_BOB, "sync");
    assert_eq!(pushed["pushed"], 1, "one change, one operation: {pushed}");
    let told = run.round(AT_ALICE, "sync");
    assert_eq!(
        ops_of_each(&told, CHANGE),
        vec![1, 2],
        "his first change and his second, whole: {told}"
    );

    // Bob's device is saved, sealed, stopped and started again from its sealed state with no key
    // given again, and his machine, built again from his application's own log, sends his changes
    // again: the core says it waits for his own chain, what it holds comes back held, and a new
    // change is kept until a round has read his chain back. That round signs it at the number
    // after his last and gives it the relay; sent again, it is held.
    let key = hex(&DATA_KEY);
    let sealed = run.save(AT_BOB, &key);
    assert_eq!(run.restore(AT_BOB, &key, &sealed), 2, "his two operations");
    let restored = run.standing(AT_BOB);
    assert_eq!(restored["awaits_own_chain"], true, "{restored}");
    assert_eq!(restored["node"], "2", "his person came back from the state");
    assert_eq!(
        run.outbound(AT_BOB, CHANGE, &write(1, BOB, 1000, 10)),
        "held"
    );
    assert_eq!(
        run.outbound(AT_BOB, CHANGE, &write(2, BOB, 3000, 30)),
        "waiting",
        "nothing new is signed before his chain is read back, and nothing is lost"
    );
    let waiting = run.standing(AT_BOB);
    assert_eq!(waiting["waiting"], 1, "{waiting}");
    let back = run.round(AT_BOB, "sync");
    assert_eq!(refused(&back), 0, "{back}");
    assert_eq!(back["pushed"], 1, "the write that waited: {back}");
    assert_eq!(back["taken"].as_array().map(Vec::len), Some(1), "{back}");
    let read_back = run.standing(AT_BOB);
    assert_eq!(read_back["awaits_own_chain"], false, "{read_back}");
    assert_eq!(read_back["waiting"], 0, "{read_back}");
    assert_eq!(
        run.outbound(AT_BOB, CHANGE, &rewrite(1, BOB, 1000, BOB, 2000, 20)),
        "held"
    );
    assert_eq!(
        run.outbound(AT_BOB, CHANGE, &write(2, BOB, 3000, 30)),
        "held",
        "the change that waited, signed once"
    );
    let rebuilt = run.round(AT_BOB, "resync");
    assert_eq!(refused(&rebuilt), 0, "{rebuilt}");
    assert_eq!(rebuilt["pushed"], 0, "nothing new: {rebuilt}");

    // Bob's device is started again from the same older state, and his copy, which keeps a journal
    // now, offers two changes before his chain is read back: both wait. The first is stamped before
    // the change of his the state lacks, so it never will be signed, and the one after it was made
    // on a state that held it. The round reads his chain back, says his own stamp rose to that
    // change's, and withdraws both, earliest first (docs/adr/0093, sections 4.4 and 6.2).
    assert_eq!(run.restore(AT_BOB, &key, &sealed), 2);
    assert_eq!(run.made(AT_BOB, &made(5, BOB, 2500, 25, 3)), "waiting");
    assert_eq!(run.made(AT_BOB, &made(6, BOB, 6000, 60, 4)), "waiting");
    let withdrawn = run.round(AT_BOB, "sync");
    assert_eq!(refused(&withdrawn), 0, "{withdrawn}");
    assert_eq!(withdrawn["pushed"], 0, "{withdrawn}");
    assert_eq!(withdrawn["taken"], json!([]), "{withdrawn}");
    assert_eq!(
        withdrawn["withdrawn"].as_array().map(Vec::len),
        Some(2),
        "{withdrawn}"
    );
    assert_eq!(withdrawn["withdrawn"][0]["wallTime"], "2500", "{withdrawn}");
    assert_eq!(
        withdrawn["raised"],
        json!({"wallTime": "3000", "counter": 0}),
        "{withdrawn}"
    );
    assert_eq!(
        withdrawn["rebuild"], false,
        "withdrawn in place: {withdrawn}"
    );

    // Alice's transport fails in the middle of a round; the round ends, and the next goes on.
    run.start(AT_ALICE, "sync");
    assert!(run.ask(AT_ALICE).is_some(), "the round asks");
    run.fail(AT_ALICE, "the connection closed");
    let failed = run.outcome(AT_ALICE);
    assert_eq!(failed["outcome"], "failed", "{failed}");
    assert_eq!(failed["failure"], "link", "{failed}");
    let told = run.round(AT_ALICE, "sync");
    assert_eq!(named(&told, CHANGE), 1, "bob's new change: {told}");
    assert_eq!(refused(&told), 0, "{told}");

    // Alice's copy keeps a journal (docs/adr/0083, section 3; docs/adr/0093, section 3): the change
    // it offers is signed in the call, the verdict given at once, and is a record her host journals,
    // which the relay is given only once her host says it recorded it. A change offered in bob's
    // name is refused.
    let older_alice = run.save(AT_ALICE, &key);
    assert_eq!(run.made(AT_ALICE, &made(4, ALICE, 5000, 50, 1)), "taken");
    assert_eq!(
        run.made(AT_ALICE, &made(4, BOB, 5000, 50, 2)),
        "refused",
        "a change in bob's name"
    );
    let (journal, records) = run.records(AT_ALICE);
    assert_eq!(records[0]["seq"], "1", "{records}");
    let held_back = run.round(AT_ALICE, "sync");
    assert_eq!(held_back["pushed"], 0, "nothing not recorded: {held_back}");
    run.recorded(AT_ALICE, 1);
    assert_eq!(run.records(AT_ALICE).1, json!([]));
    let given = run.round(AT_ALICE, "sync");
    assert_eq!(given["pushed"], 1, "her change, once recorded: {given}");

    // Alice's device is started again from the state saved before her change, and her host gives
    // back the record its journal holds: nothing is signed again. Her copy offers a change before
    // her chain is read back, which waits; the history her rebuilt machine is given is bob's three
    // changes and hers in causal order, each with its tag, and the one that waits, provisional.
    // The round reads her chain back, signs the change that waited, which is a record held back
    // until she recorded it, and says her own stamp rose.
    assert_eq!(run.restore(AT_ALICE, &key, &older_alice), 3, "bob's three");
    assert_eq!(run.restore_records(AT_ALICE, &journal), 1);
    assert_eq!(run.made(AT_ALICE, &made(5, ALICE, 6000, 60, 3)), "waiting");
    let history = run.history(AT_ALICE);
    let changes = history["changes"].as_array().expect("a history's changes");
    assert_eq!(changes.len(), 5, "{history}");
    assert_eq!(changes[3]["author"], "1", "{history}");
    assert_eq!(changes[3]["context"], json!([["2", "3"]]), "{history}");
    assert_eq!(changes[4]["provisional"], true, "{history}");
    assert_eq!(run.own_stamp(AT_ALICE), (5000, 0, 0));
    let confirmed = run.round(AT_ALICE, "sync");
    assert_eq!(refused(&confirmed), 0, "{confirmed}");
    assert_eq!(confirmed["taken"].as_array().map(Vec::len), Some(1));
    assert_eq!(confirmed["pushed"], 0, "a record not recorded: {confirmed}");
    assert_eq!(
        confirmed["raised"],
        json!({"wallTime": "6000", "counter": 0}),
        "{confirmed}"
    );
    assert_eq!(run.own_stamp(AT_ALICE), (6000, 0, 1));
    let (_, records) = run.records(AT_ALICE);
    assert_eq!(records[0]["seq"], "2", "{records}");
    run.recorded(AT_ALICE, 2);
    let given = run.round(AT_ALICE, "sync");
    assert_eq!(given["pushed"], 1, "{given}");

    // Bob's machine is told her two changes, and he has written nothing since: he marks what he
    // was told. His mark reaches the relay and her core, which tells her machine nothing of it, and
    // the stable cut is every change of both (docs/adr/0085, sections 3.1 and 4).
    let read = run.round(AT_BOB, "sync");
    assert_eq!(named(&read, CHANGE), 2, "{read}");
    assert_eq!(run.mark(AT_BOB), 4, "his fourth operation");
    assert_eq!(run.mark(AT_BOB), 0, "nothing more was told since");
    let marked = run.round(AT_BOB, "sync");
    assert_eq!(marked["pushed"], 1, "{marked}");
    let read_mark = run.round(AT_ALICE, "sync");
    assert_eq!(read_mark["events"], json!([]), "{read_mark}");
    assert_eq!(run.stable_cut(AT_ALICE), json!([["1", "2"], ["2", "4"]]));

    // Alice takes bob off: his chain ends at the last operation of his her device holds, and her
    // round gives the relay the entry.
    run.remove(AT_ALICE, BOB);
    let taken_off = run.standing(AT_ALICE);
    assert_eq!(members(&taken_off), vec!["1"], "{taken_off}");
    let removed = run.round(AT_ALICE, "sync");
    assert_eq!(refused(&removed), 0, "{removed}");
    assert_eq!(removed["pushed"], 1, "the removal: {removed}");

    // Bob's machine writes before his device has read that he is off, and his core signs it. His
    // round reads the removal, and the relay, which lets a device the log took off read and give
    // nothing, refuses him: the write reaches no one, and alice's machine is told nothing of it.
    assert_eq!(
        run.outbound(AT_BOB, CHANGE, &write(3, BOB, 4000, 40)),
        "taken"
    );
    let cut = run.round(AT_BOB, "sync");
    assert_eq!(cut["outcome"], "ended", "{cut}");
    assert_eq!(cut["pushed"], 0, "{cut}");
    assert_eq!(refused(&cut), 1, "the relay's refusal: {cut}");
    let off = run.standing(AT_BOB);
    assert_eq!(off["role"], 0, "{off}");
    assert_eq!(members(&off), vec!["1"], "{off}");
    // Logging in again is refused: the latest grant of his node at the relay gives him no role.
    let refused_login = run.login(AT_BOB);
    assert_eq!(refused_login["outcome"], "failed", "{refused_login}");
    assert_eq!(refused_login["code"], "forbidden", "{refused_login}");
    let after = run.round(AT_ALICE, "sync");
    assert_eq!(named(&after, CHANGE), 0, "{after}");
    assert_eq!(refused(&after), 0, "{after}");

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
        let number = |name: &str| -> u64 {
            step[name]
                .as_u64()
                .unwrap_or_else(|| panic!("step {at} has no `{name}`"))
        };
        if step["op"] == "make" {
            cores[device] = None;
            cores[device] = Some(Core::make(&devices[device]));
            continue;
        }
        if step["op"] == "make_invited" {
            cores[device] = None;
            cores[device] = Some(Core::make_invited(&devices[device], &field("invitation")));
            continue;
        }
        if step["op"] == "restore" {
            // The device started again: a core given nothing but the list and the genesis.
            cores[device] = None;
            let mut core = Core::bare(&devices[device]);
            let node = devices[device]["node"].as_u64().unwrap();
            assert_eq!(
                core.restore(node, &field("key"), &field("state")),
                number("count"),
                "step {at}"
            );
            cores[device] = Some(core);
            continue;
        }
        let core = cores[device]
            .as_mut()
            .unwrap_or_else(|| panic!("step {at}: the device's core is not made"));
        match step["op"].as_str().unwrap() {
            "open" => assert_eq!(core.open(&field("profile")), field("genesis"), "step {at}"),
            "grant" => core.grant(
                number("node"),
                u32::try_from(number("role")).unwrap(),
                &field("key"),
            ),
            "remove" => core.remove(number("node")),
            "standing" => assert_eq!(core.standing(), field("text"), "step {at}"),
            "outbound" => assert_eq!(
                core.outbound(&field("event"), &field("data")),
                (field("kept"), field("reason")),
                "step {at}"
            ),
            "start" => core.start(&field("kind")),
            "login" => core.login(&field("audience")),
            "request" => assert_eq!(core.poll(), Some(field("text")), "step {at}"),
            "respond" => core.respond(&field("text")),
            "fail" => core.fail(&field("reason")),
            "outcome" => assert_eq!(core.outcome(), field("text"), "step {at}"),
            "save" => assert_eq!(core.save(&field("key")), field("state"), "step {at}"),
            "invite" => assert_eq!(
                core.invite(
                    &field("relay"),
                    &field("audience"),
                    number("node"),
                    number("now_ms"),
                    number("valid_ms"),
                    &field("secret"),
                ),
                field("text"),
                "step {at}"
            ),
            "invitation" => assert_eq!(
                Core::invitation(&field("invitation")),
                field("text"),
                "step {at}"
            ),
            "join" => assert_eq!(
                core.join(&field("invitation")),
                field("fingerprint"),
                "step {at}"
            ),
            "joined" => assert_eq!(core.joined(number("node")), field("text"), "step {at}"),
            "made" => assert_eq!(
                core.made(&field("data")),
                (field("kept"), field("reason")),
                "step {at}"
            ),
            "records" => assert_eq!(core.records(), field("text"), "step {at}"),
            "recorded" => core.recorded(number("through")),
            "restore_records" => assert_eq!(
                core.restore_records(&field("records")),
                number("count"),
                "step {at}"
            ),
            "history" => assert_eq!(core.history(), field("text"), "step {at}"),
            "stable_cut" => assert_eq!(core.stable_cut(), field("text"), "step {at}"),
            "mark" => assert_eq!(core.mark(), number("seq"), "step {at}"),
            "own_stamp" => assert_eq!(
                core.own_stamp(),
                (
                    step["wall_time"].as_i64().expect("a wall time"),
                    u32::try_from(number("counter")).unwrap(),
                    i32::try_from(number("read_back")).unwrap(),
                ),
                "step {at}"
            ),
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
