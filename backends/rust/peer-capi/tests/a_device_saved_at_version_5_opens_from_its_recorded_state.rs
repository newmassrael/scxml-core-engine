// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A recorded sealed state of version 5 (`tests/fixtures/a_device_saved_at_version_5.json`): what a
// device kept before a change of the list was one unit (docs/adr/0085, landing 4c), with the data
// key it was sealed under and the relay's history of the list in the second event domain, so that
// a host tests its upgrade path on bytes that do not move. Alice's device holds the administrator's
// key and the list's log, and her list was written by three devices in parts: a change of two
// writes, a value two devices cleared at once, a write-over sent as a change of one, and a write
// after it. The parts are signed over the bytes the reader of the second domain gives
// (`sce_rust_trust::parts`), which are held to the vectors that domain's own encoder pinned before
// it was removed, and the state is the text of version 5 sealed under `sce-peer/state/5`. The
// recording also holds the fold a core gave when it was made (`sce_peer_fold`, restores of at most
// 64 values), which a host's upgrade can be compared with.
//
// What is asked of the core of today, through the ABI as a host reaches it: the state opens under
// its key and is upgraded (section 8.3), every part held; the fold is the recorded one and holds the
// rows the list held; no machine is told a part; a relay that holds the recorded log and parts
// starts, the device's round reads its chain back from it with nothing to build again, and its next
// save opens with the same fold.
//
// The recording is a device's old save: it is written once, and a run checks the parts it holds
// are the ones signed now, not the sealed bytes, which a core of version 5 sealed and no core of
// today seals again. It is written with `SCE_PEER_STATE_5=write cargo test -p sce-rust-peer-capi
// --test a_device_saved_at_version_5_opens_from_its_recorded_state -- a_state_of_version_5_is_recorded`.

#![deny(unsafe_op_in_unsafe_fn)]

use std::collections::BTreeMap;
use std::ffi::{CStr, CString};
use std::path::Path;

use sce_peer_capi::*;
use sce_rust_relay::{Authority, CourierState, Limits, MemoryStore, Record, Relay, Store};
use sce_rust_trust::backup::{self, Contents, DataKey};
use sce_rust_trust::membership::{Body, Entry, Log, Profile};
use sce_rust_trust::message::{self, Link};
use sce_rust_trust::{encoding, parts, Identity, Trust};
use serde_json::{json, Value};

const LIST: &str = "home";
const ALICE: u64 = 1;
const BOB: u64 = 2;
const CAROL: u64 = 3;

/// The data key the state is sealed under: fixed here, so that the recording is made of the same
/// bytes at every run. A device drew one for each save.
const DATA_KEY: [u8; 32] = [4; 32];

/// The most values a restore of the recorded fold carries: a build's `shared_parts`.
const PARTS: u32 = 64;

/// The key the relay's tokens are made under, and its name.
const AUTHORITY: [u8; 32] = [3; 32];
const AUDIENCE: &str = "the-relay";

const PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/a_device_saved_at_version_5.json"
);
const WRITE_AGAIN: &str = "SCE_PEER_STATE_5=write cargo test -p sce-rust-peer-capi --test \
     a_device_saved_at_version_5_opens_from_its_recorded_state -- a_state_of_version_5_is_recorded";

fn person(node: u64) -> Identity {
    Identity::from_seed([node as u8; 32])
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

/// The log of the list: its genesis, alice its owner, and bob and carol granted writing.
fn entries() -> Vec<Entry> {
    let scratch = Trust::open(LIST);
    let genesis = genesis();
    scratch.append(&genesis).unwrap();
    let mut all = vec![genesis];
    for (node, role) in [(ALICE, 3u8), (BOB, 2), (CAROL, 2)] {
        let entry = scratch
            .draft(Body::Grant {
                node,
                role,
                key: person(node).public(),
                final_seq: None,
            })
            .sign(&administrator())
            .finish();
        scratch.append(&entry).unwrap();
        all.push(entry);
    }
    all
}

/// The parts each device signed, chain by chain, as a device of the second domain signed them,
/// each under the last entry of the log.
struct Chains {
    log: Vec<Entry>,
    last: BTreeMap<u64, (u64, [u8; 32])>,
}

impl Chains {
    fn new() -> Self {
        Self {
            log: entries(),
            last: BTreeMap::new(),
        }
    }

    /// The next part of `author`'s chain: the event and the data it travels as.
    fn sign(&mut self, author: u64, event: &str, data: Value) -> (String, String) {
        let (seq, prev) = self
            .last
            .get(&author)
            .map_or((1, message::NO_OPERATION), |(seq, id)| (seq + 1, *id));
        let link = Link {
            seq,
            prev,
            log: self.log.len() as u64 - 1,
            log_hash: self.log.last().unwrap().hash(),
            context: Vec::new(),
        };
        let signing = parts::signing(LIST, event, &data).unwrap().unwrap();
        let bytes = parts::operation_bytes(&signing.message, &link);
        let mut wire = data;
        wire["seq"] = json!(seq.to_string());
        wire["prev"] = json!(encoding::encode(&prev));
        wire["mh"] = json!(link.log.to_string());
        wire["mhh"] = json!(encoding::encode(&link.log_hash));
        wire["sig"] = json!(encoding::encode(&person(author).sign(&bytes)));
        self.last
            .insert(author, (seq, message::operation_id(&bytes)));
        (event.to_string(), wire.to_string())
    }
}

/// A write of `node`'s to field 3 of row `id`, one of `of` parts of its change at `wall`.
fn write(node: u64, id: u64, text: &str, wall: i64, of: u64) -> Value {
    json!({"id": id, "field": 3, "value": "0", "text": text, "wallTime": wall.to_string(),
           "counter": 0, "nodeId": node.to_string(), "parts": of})
}

/// A write-over by `by` of the value `node` wrote at `wall`, one of `of` parts of `by`'s change at
/// `at`; a change of one carries no stamp of its own.
fn write_over(by: u64, id: u64, node: u64, wall: i64, of: u64, at: i64) -> Value {
    let mut part = json!({"id": id, "field": 3, "wallTime": wall.to_string(), "counter": 0,
                          "nodeId": node.to_string(), "by": by.to_string()});
    if of > 1 {
        part["parts"] = json!(of);
        part["atWallTime"] = json!(at.to_string());
        part["atCounter"] = json!(0);
    }
    part
}

/// The list's history in the second domain: a change of two writes, a clear of one value by two
/// devices at once (bob writes over alice's milk with oat milk, carol clears it and alice's eggs),
/// a write-over sent as a change of one, and alice's bread after it.
fn history() -> Vec<(String, String)> {
    let mut chains = Chains::new();
    let (w, s) = ("record_list.write", "record_list.supersede");
    vec![
        chains.sign(ALICE, w, write(ALICE, 1, "milk", 1000, 1)),
        chains.sign(ALICE, w, write(ALICE, 2, "eggs", 1100, 2)),
        chains.sign(ALICE, w, write(ALICE, 4, "today", 1100, 2)),
        chains.sign(BOB, s, write_over(BOB, 1, ALICE, 1000, 2, 1200)),
        chains.sign(BOB, w, write(BOB, 1, "oat milk", 1200, 2)),
        chains.sign(CAROL, s, write_over(CAROL, 1, ALICE, 1000, 2, 1250)),
        chains.sign(CAROL, s, write_over(CAROL, 2, ALICE, 1100, 2, 1250)),
        chains.sign(ALICE, s, write_over(ALICE, 4, ALICE, 1100, 1, 0)),
        chains.sign(ALICE, w, write(ALICE, 3, "bread", 1400, 1)),
    ]
}

/// Alice's device as a core of version 5 sealed it: her person, the administrator's key, the log,
/// every part she held, and what the relay's client kept, under `key`.
fn sealed_at_version_5(key: &DataKey, operations: &[(String, String)]) -> Vec<u8> {
    let trust = json!({
        "checkpoints": [],
        "operations": operations
            .iter()
            .map(|(event, data)| json!([event, data]))
            .collect::<Vec<_>>(),
        "waiting": [],
    });
    let state = json!({
        "version": 5,
        "log": entries().iter().map(Entry::to_json).collect::<Vec<_>>(),
        "trust": trust.to_string(),
        "courier": CourierState::default().to_text(),
        "owed": [],
        "invitations": [],
        "joins": [],
    });
    let contents = Contents::of_identity(LIST, ALICE, &person(ALICE))
        .with("peer/state", state.to_string().into_bytes())
        .with_identity("peer/administrator", &administrator());
    backup::seal_state(key, "sce-peer/state/5", &contents).expect("the state seals")
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn unhex(text: &str) -> Vec<u8> {
    (0..text.len() / 2)
        .map(|index| u8::from_str_radix(&text[2 * index..2 * index + 2], 16).unwrap())
        .collect()
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

/// The bytes of a buffer the library gave, which is given back; `None` for an empty buffer.
fn taken_bytes(buf: ScePeerBuf) -> Option<Vec<u8>> {
    if buf.ptr.is_null() {
        return None;
    }
    // SAFETY: a buffer the library gave is `len` readable bytes, and is given back once, here.
    Some(unsafe {
        let bytes = std::slice::from_raw_parts(buf.ptr, buf.len).to_vec();
        sce_peer_buf_free(buf);
        bytes
    })
}

/// The text of a buffer the library gave, which is given back; `None` for an empty buffer.
fn taken(buf: ScePeerBuf) -> Option<String> {
    taken_bytes(buf)
        .map(|bytes| String::from_utf8(bytes).expect("a text the library gives is UTF-8"))
}

fn text(value: &str) -> CString {
    CString::new(value).expect("a text of the recording holds no NUL")
}

fn field<'a>(recorded: &'a Value, name: &str) -> &'a str {
    recorded[name]
        .as_str()
        .unwrap_or_else(|| panic!("the recording has no `{name}`"))
}

/// The upgraded device's core, reached through the C ABI as a host reaches it.
struct Core(*mut ScePeer);

impl Core {
    /// A core of the recorded list invited with its genesis and given nothing else: what a sealed
    /// state is restored into.
    fn bare(recorded: &Value) -> Self {
        let list = text(field(recorded, "list"));
        let genesis = unhex(field(recorded, "genesis"));
        assert_eq!(genesis.len(), 32, "a genesis is 32 bytes");
        // SAFETY: the list is NUL-terminated, the genesis is 32 bytes.
        let peer = unsafe { sce_peer_invited(list.as_ptr(), genesis.as_ptr()) };
        assert!(!peer.is_null(), "{}", last_error());
        Core(peer)
    }

    /// The state `state` taken back under the data key `key` for the device of `node`, and how
    /// many operations it kept.
    fn restore(&mut self, node: u64, key: &[u8], state: &[u8]) -> u64 {
        assert_eq!(key.len(), 32, "a data key is 32 bytes");
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

    /// A core of the recorded list with the recorded state taken back, and how many operations it
    /// kept.
    fn upgraded(recorded: &Value) -> (Self, u64) {
        let mut core = Self::bare(recorded);
        let node = recorded["node"]
            .as_u64()
            .expect("the recording names the node");
        let kept = core.restore(
            node,
            &unhex(field(recorded, "key")),
            &unhex(field(recorded, "state")),
        );
        (core, kept)
    }

    fn standing(&mut self) -> Value {
        let mut out = empty();
        // SAFETY: a live handle, and a place valid for a write.
        check(unsafe { sce_peer_standing(self.0, &mut out) }, "standing");
        serde_json::from_str(&taken(out).expect("a standing is a text"))
            .expect("a standing is JSON")
    }

    /// The fold of the parts the core holds, in restores of at most [`PARTS`] values, as the ABI's
    /// JSON text.
    fn fold(&mut self) -> String {
        let mut out = empty();
        // SAFETY: a live handle, and a place valid for a write.
        check(unsafe { sce_peer_fold(self.0, PARTS, &mut out) }, "fold");
        taken(out).expect("a fold is a text")
    }

    /// The whole history a machine built again is given.
    fn history(&mut self) -> Value {
        let mut out = empty();
        // SAFETY: a live handle, no cut, and a place valid for a write.
        check(
            unsafe { sce_peer_history(self.0, std::ptr::null(), &mut out) },
            "history",
        );
        serde_json::from_str(&taken(out).expect("a history is a text")).expect("a history is JSON")
    }

    /// A round, every request carried to `relay` with `token` and every answer given back: its
    /// outcome.
    fn round(&mut self, relay: &mut Relay<MemoryStore>, token: &str) -> Value {
        // SAFETY: a live handle.
        check(
            unsafe { sce_peer_start(self.0, SCE_PEER_ROUND_SYNC) },
            "start",
        );
        loop {
            let mut out = empty();
            // SAFETY: a live handle, and a place valid for a write.
            check(
                unsafe { sce_peer_poll_request(self.0, &mut out) },
                "poll_request",
            );
            let Some(request) = taken(out) else { break };
            let response = text(&relay.handle(token, &request, 0));
            // SAFETY: a live handle, and a NUL-terminated text.
            check(
                unsafe { sce_peer_respond(self.0, response.as_ptr()) },
                "respond",
            );
        }
        let mut out = empty();
        // SAFETY: a live handle, and a place valid for a write.
        check(
            unsafe { sce_peer_take_outcome(self.0, &mut out) },
            "take_outcome",
        );
        serde_json::from_str(&taken(out).expect("the round ended")).expect("an outcome is JSON")
    }

    /// What the device keeps, sealed under the data key `key`.
    fn save(&mut self, key: &[u8]) -> Vec<u8> {
        assert_eq!(key.len(), 32, "a data key is 32 bytes");
        let mut out = empty();
        // SAFETY: a live handle, a key of 32 bytes, and a place valid for a write.
        check(
            unsafe { sce_peer_save_sealed_under(self.0, key.as_ptr(), &mut out) },
            "save_sealed_under",
        );
        taken_bytes(out).expect("a sealed state is not empty")
    }
}

impl Drop for Core {
    fn drop(&mut self) {
        // SAFETY: a handle this test made and frees once.
        unsafe { sce_peer_free(self.0) };
    }
}

/// A relay that holds the recorded log and parts, as the relay of the list kept them, and a token
/// of its authority for the recorded device.
fn relay_of(recorded: &Value) -> (Relay<MemoryStore>, String) {
    let list = field(recorded, "list");
    let mut store = MemoryStore::new();
    for entry in recorded["log"]
        .as_array()
        .expect("the recording holds the log")
    {
        let entry = entry.as_str().expect("an entry is a JSON text").to_string();
        store.append(list, &Record::Entry(entry)).unwrap();
    }
    for part in recorded["parts"]
        .as_array()
        .expect("the recording holds the parts")
    {
        let record = Record::Operation {
            event: part[0].as_str().expect("a part's event").to_string(),
            data: part[1].as_str().expect("a part's data").to_string(),
        };
        store.append(list, &record).unwrap();
    }
    let authority = Authority::new(AUTHORITY, AUDIENCE);
    let node = recorded["node"]
        .as_u64()
        .expect("the recording names the node");
    let token = authority.issue(list, node, u64::MAX);
    let relay = Relay::open(store, authority, Limits::default())
        .expect("a relay that holds the recorded parts starts");
    (relay, token)
}

/// The recording made now: the parts, the state of version 5 sealed with them, its key, and the
/// fold a core of today gives once it opened the state.
fn record() -> Value {
    let operations = history();
    let state = sealed_at_version_5(&DataKey::from_bytes(DATA_KEY), &operations);
    let mut recording = json!({
        "list": LIST,
        "genesis": hex(&genesis().hash()),
        "node": ALICE,
        "version": 5,
        "key": hex(&DATA_KEY),
        "state": hex(&state),
        "log": entries().iter().map(Entry::to_json).collect::<Vec<_>>(),
        "parts": operations
            .iter()
            .map(|(event, data)| json!([event, data]))
            .collect::<Vec<_>>(),
    });
    let (mut core, _) = Core::upgraded(&recording);
    recording["fold"] = json!(core.fold());
    recording
}

fn recorded() -> Value {
    let text = std::fs::read_to_string(PATH)
        .unwrap_or_else(|_| panic!("{PATH} is not recorded: {WRITE_AGAIN}"));
    serde_json::from_str(&text).expect("the recording is JSON")
}

/// The recording is written in its mode; at every run, the list, the log and the parts it holds
/// are the ones made now, signed over the bytes the reader holds to the old encoder's vectors. The
/// sealed state is not made again to be compared: it is a device's old save. Red if a part of the
/// recording is signed over other bytes than the reader gives.
#[test]
fn a_state_of_version_5_is_recorded() {
    let fresh = record();
    if std::env::var("SCE_PEER_STATE_5").as_deref() == Ok("write") {
        std::fs::create_dir_all(Path::new(PATH).parent().unwrap()).unwrap();
        std::fs::write(PATH, serde_json::to_string_pretty(&fresh).unwrap() + "\n").unwrap();
    }
    let recorded = recorded();
    for name in ["list", "genesis", "node", "version", "log", "parts"] {
        assert_eq!(
            recorded[name], fresh[name],
            "the recording's `{name}` is not the one made now"
        );
    }
}

/// The recorded state of version 5 opens under its key in a core of today and is upgraded: every
/// part is held, the device keeps its person, the administrator's key and its role, and waits for
/// its own chain; the fold is the recorded one, and holds the rows the list held, bob's oat milk
/// and alice's bread, at each device's last part; no machine is told a part. Red if the state is
/// refused, if a part is dropped, or if the fold gives other rows.
#[test]
fn the_recorded_state_of_version_5_opens_with_the_rows_it_held() {
    let recorded = recorded();
    let parts = recorded["parts"]
        .as_array()
        .expect("the recording holds the parts");
    let (mut core, kept) = Core::upgraded(&recorded);
    assert_eq!(
        kept,
        u64::try_from(parts.len()).unwrap(),
        "every part is held"
    );

    let standing = core.standing();
    assert_eq!(standing["node"], "1", "{standing}");
    assert_eq!(standing["role"], 3, "{standing}");
    assert_eq!(standing["administrator"], true, "{standing}");
    assert_eq!(standing["awaits_own_chain"], true, "{standing}");

    let fold = core.fold();
    assert_eq!(
        fold,
        field(&recorded, "fold"),
        "the fold the recording holds"
    );
    let fold: Value = serde_json::from_str(&fold).expect("a fold is JSON");
    assert_eq!(
        fold["cut"],
        json!([["1", "5"], ["2", "2"], ["3", "2"]]),
        "{fold}"
    );
    let restores = fold["restores"].as_array().expect("a fold's restores");
    assert_eq!(restores.len(), 1, "{fold}");
    let restore: Value =
        serde_json::from_str(restores[0].as_str().expect("a restore is a JSON text"))
            .expect("a restore is JSON");
    assert_eq!(restore["wallTime"], "1400", "{restore}");
    let rows: Vec<(&str, &str, &str)> = restore["ops"]
        .as_array()
        .expect("a restore's values")
        .iter()
        .map(|op| {
            (
                op["text"].as_str().unwrap(),
                op["nodeId"].as_str().unwrap(),
                op["wallTime"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        rows,
        vec![("oat milk", "2", "1200"), ("bread", "1", "1400")],
        "{restore}"
    );
    assert_eq!(
        core.history()["changes"],
        json!([]),
        "no machine is told a part: the list is the fold"
    );
}

/// A relay that holds the recorded log and parts starts, and the upgraded device's first round
/// reads its own chain back from it with nothing to build again, since every part it reads it
/// held; its next save opens again with the same fold. Red if the relay refuses a recorded part,
/// or if the device reads its parts as new.
#[test]
fn the_upgraded_device_reads_its_chain_back_from_a_relay_of_the_recorded_parts() {
    let recorded = recorded();
    let (mut relay, token) = relay_of(&recorded);
    let (mut core, _) = Core::upgraded(&recorded);
    let outcome = core.round(&mut relay, &token);
    assert_eq!(outcome["outcome"], "ended", "{outcome}");
    assert_eq!(outcome["refused"], json!([]), "{outcome}");
    assert_eq!(
        outcome["rebuild"], false,
        "the parts read again were held: {outcome}"
    );
    let standing = core.standing();
    assert_eq!(standing["awaits_own_chain"], false, "{standing}");

    let key = unhex(field(&recorded, "key"));
    let saved = core.save(&key);
    let mut again = Core::bare(&recorded);
    let node = recorded["node"]
        .as_u64()
        .expect("the recording names the node");
    again.restore(node, &key, &saved);
    assert_eq!(
        again.fold(),
        core.fold(),
        "the next save holds the same parts"
    );
}
