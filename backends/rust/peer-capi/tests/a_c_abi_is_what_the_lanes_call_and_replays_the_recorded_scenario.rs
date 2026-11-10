// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
//
// The application lanes that are not Rust reach the peer core through the C ABI. What is asked here:
// that the header declares exactly what the library exports, with the same numbers; and that the
// boundary refuses a null pointer, a text that is not UTF-8, an answer out of turn, a kind of round
// that is none and bytes that are no sealed state, with a status and a text, takes no outcome into
// nowhere, and never crosses with a panic; that an administrator's act the core cannot do is refused
// with its own status and nothing written; that an administrator's device opens a list keyed by its
// group, admits a device from the key package a round read and takes it off again through the ABI
// alone, the device joining from the Welcome and holding the key of the epoch the administrator's
// does, coming back whole from its sealed state with no key given again, and holding none of the
// epoch after its removal; that a sealed state is refused under another key or for another node,
// and bytes that are none, with nothing taken; and that the recorded scenario of two devices and a relay
// (`tests/fixtures/two_devices_and_a_relay.json`, recorded and held to a fresh run by
// `the_recorded_scenario_is_what_a_fresh_run_records`) is replayed through the shared library, with
// no relay, by C, C++, Python (ctypes), Go (cgo), Kotlin (JNA) and Swift (a module map over the
// header), and every text each core gives is the recorded one. A lane whose tool is missing says so
// and passes, unless `SCE_REQUIRE_TOOLS` is set, which is a lane's claim that it ran. The Kotlin lane
// needs its JVM program built (`./gradlew :sce-kotlin-peer-replay:installDist`).

#![deny(unsafe_op_in_unsafe_fn)]

use std::ffi::{c_char, CStr, CString};
use std::path::{Path, PathBuf};
use std::process::Command;

use sce_peer_capi::*;
use sce_rust_relay::{Authority, Doorway, Limits, LoginPolicy, MemoryStore, Relay};
use sce_rust_trust::{encoding, Identity};
use serde_json::{json, Value};

const MANIFEST: &str = env!("CARGO_MANIFEST_DIR");

fn scenario_path() -> PathBuf {
    Path::new(MANIFEST).join("tests/fixtures/two_devices_and_a_relay.json")
}

fn scenario() -> Value {
    let path = scenario_path();
    let text = std::fs::read_to_string(&path).unwrap_or_else(|_| {
        panic!(
            "{} is not recorded: SCE_PEER_SCENARIO=write cargo test -p sce-rust-peer-capi --test \
             the_recorded_scenario_is_what_a_fresh_run_records -- the_recorded_scenario_is_what",
            path.display()
        )
    });
    serde_json::from_str(&text).expect("the recording is JSON")
}

/// How many steps a lane must say it replayed.
fn steps_of(scenario: &Value) -> usize {
    scenario["steps"].as_array().unwrap().len()
}

fn last_error() -> String {
    // SAFETY: the library returns a NUL-terminated text it owns.
    unsafe { CStr::from_ptr(sce_peer_last_error()) }
        .to_string_lossy()
        .into_owned()
}

fn empty() -> ScePeerBuf {
    ScePeerBuf {
        ptr: std::ptr::null_mut(),
        len: 0,
    }
}

/// The text of a buffer the library gave, which is given back; `None` for an empty buffer.
///
/// # Safety
/// `buf` is one the library returned and has not given back, or empty.
unsafe fn taken(buf: ScePeerBuf) -> Option<String> {
    if buf.ptr.is_null() {
        return None;
    }
    // SAFETY: the library's buffer is `len` readable bytes.
    let bytes = unsafe { std::slice::from_raw_parts(buf.ptr, buf.len) }.to_vec();
    // SAFETY: returned by the library and not given back yet.
    unsafe { sce_peer_buf_free(buf) };
    Some(String::from_utf8(bytes).expect("a text the library gives is UTF-8"))
}

/// What a call that fills a buffer gave, after it returned `SCE_PEER_OK`.
///
/// # Safety
/// `peer` is a live handle.
unsafe fn text_of(
    peer: *mut ScePeer,
    call: unsafe extern "C" fn(*mut ScePeer, *mut ScePeerBuf) -> i32,
) -> Option<String> {
    let mut out = empty();
    // SAFETY: `peer` is live and `out` is valid for a write.
    assert_eq!(
        unsafe { call(peer, &mut out) },
        SCE_PEER_OK,
        "{}",
        last_error()
    );
    // SAFETY: what the call just gave.
    unsafe { taken(out) }
}

#[test]
fn the_header_declares_exactly_what_the_library_exports() {
    let header = std::fs::read_to_string(Path::new(MANIFEST).join("include/sce_peer.h")).unwrap();
    let source = std::fs::read_to_string(Path::new(MANIFEST).join("src/lib.rs")).unwrap();

    let names_in = |text: &str, prefix: &str| -> std::collections::BTreeSet<String> {
        let mut names = std::collections::BTreeSet::new();
        let mut rest = text;
        while let Some(found) = rest.find(prefix) {
            let tail = &rest[found..];
            let end = tail
                .find(|c: char| !(c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'))
                .unwrap_or(tail.len());
            names.insert(tail[..end].to_string());
            rest = &tail[end..];
        }
        names
    };
    let declared: std::collections::BTreeSet<String> = names_in(&header, "sce_peer_")
        .into_iter()
        .filter(|name| header.contains(&format!("{name}(")))
        .collect();
    let exported: std::collections::BTreeSet<String> = source
        .lines()
        .filter(|line| line.contains("extern \"C\" fn sce_peer_"))
        .flat_map(|line| names_in(line, "sce_peer_"))
        .collect();
    assert_eq!(declared, exported);
    assert!(exported.len() >= 27, "{exported:?}");

    // The statuses, the answers to a send and the kinds of round are the same numbers on both sides.
    for (name, value) in [
        ("SCE_PEER_OK", SCE_PEER_OK),
        ("SCE_PEER_ERR_NOTHING_ASKED", SCE_PEER_ERR_NOTHING_ASKED),
        (
            "SCE_PEER_ERR_OUTCOME_NOT_TAKEN",
            SCE_PEER_ERR_OUTCOME_NOT_TAKEN,
        ),
        ("SCE_PEER_ERR_NO_GROUP_DEVICE", SCE_PEER_ERR_NO_GROUP_DEVICE),
        ("SCE_PEER_ERR_GROUP", SCE_PEER_ERR_GROUP),
        ("SCE_PEER_ERR_UNREADABLE", SCE_PEER_ERR_UNREADABLE),
        (
            "SCE_PEER_ERR_NO_ADMINISTRATOR",
            SCE_PEER_ERR_NO_ADMINISTRATOR,
        ),
        ("SCE_PEER_ERR_NO_PERSON", SCE_PEER_ERR_NO_PERSON),
        ("SCE_PEER_ERR_OPENED", SCE_PEER_ERR_OPENED),
        ("SCE_PEER_ERR_LOG", SCE_PEER_ERR_LOG),
        ("SCE_PEER_ERR_NOT_A_MEMBER", SCE_PEER_ERR_NOT_A_MEMBER),
        (
            "SCE_PEER_ERR_ADMITTED_THROUGH_GROUP",
            SCE_PEER_ERR_ADMITTED_THROUGH_GROUP,
        ),
        ("SCE_PEER_ERR_NO_KEY_PACKAGE", SCE_PEER_ERR_NO_KEY_PACKAGE),
        ("SCE_PEER_ERR_DRAWN_KEYS", SCE_PEER_ERR_DRAWN_KEYS),
        ("SCE_PEER_ERR_NOT_OPENED", SCE_PEER_ERR_NOT_OPENED),
        ("SCE_PEER_ERR_NO_RANDOMNESS", SCE_PEER_ERR_NO_RANDOMNESS),
        ("SCE_PEER_ERR_GRANTED", SCE_PEER_ERR_GRANTED),
        ("SCE_PEER_ERR_INVITATION", SCE_PEER_ERR_INVITATION),
        ("SCE_PEER_ERR_NOT_JOINED", SCE_PEER_ERR_NOT_JOINED),
        ("SCE_PEER_ERR_NULL_ARGUMENT", SCE_PEER_ERR_NULL_ARGUMENT),
        ("SCE_PEER_ERR_BAD_TEXT", SCE_PEER_ERR_BAD_TEXT),
        ("SCE_PEER_ERR_PANIC", SCE_PEER_ERR_PANIC),
        ("SCE_PEER_ERR_BAD_ARGUMENT", SCE_PEER_ERR_BAD_ARGUMENT),
        ("SCE_PEER_ERR_CANCELLED", SCE_PEER_ERR_CANCELLED),
        ("SCE_PEER_ERR_DEADLINE", SCE_PEER_ERR_DEADLINE),
        ("SCE_PEER_ERR_SINK_TAKEN", SCE_PEER_ERR_SINK_TAKEN),
        (
            "SCE_PEER_TRANSPORT_RATE_LIMITED",
            SCE_PEER_TRANSPORT_RATE_LIMITED,
        ),
        ("SCE_PEER_PAUSE_WAITED", SCE_PEER_PAUSE_WAITED),
        ("SCE_PEER_PAUSE_WOKEN", SCE_PEER_PAUSE_WOKEN),
        ("SCE_PEER_LOG_OFF", SCE_PEER_LOG_OFF),
        ("SCE_PEER_LOG_ERROR", SCE_PEER_LOG_ERROR),
        ("SCE_PEER_LOG_WARN", SCE_PEER_LOG_WARN),
        ("SCE_PEER_LOG_INFO", SCE_PEER_LOG_INFO),
        ("SCE_PEER_LOG_DEBUG", SCE_PEER_LOG_DEBUG),
        ("SCE_PEER_LOG_TRACE", SCE_PEER_LOG_TRACE),
        ("SCE_PEER_TRANSPORT_ANSWERED", SCE_PEER_TRANSPORT_ANSWERED),
        ("SCE_PEER_TRANSPORT_LINK", SCE_PEER_TRANSPORT_LINK),
        (
            "SCE_PEER_TRANSPORT_UNAUTHORIZED",
            SCE_PEER_TRANSPORT_UNAUTHORIZED,
        ),
        ("SCE_PEER_TRANSPORT_DEADLINE", SCE_PEER_TRANSPORT_DEADLINE),
        ("SCE_PEER_TRANSPORT_CANCELLED", SCE_PEER_TRANSPORT_CANCELLED),
        ("SCE_PEER_KEPT_TAKEN", SCE_PEER_KEPT_TAKEN),
        ("SCE_PEER_KEPT_HELD", SCE_PEER_KEPT_HELD),
        ("SCE_PEER_KEPT_NOT_SIGNED", SCE_PEER_KEPT_NOT_SIGNED),
        ("SCE_PEER_KEPT_REFUSED", SCE_PEER_KEPT_REFUSED),
        ("SCE_PEER_KEPT_WAITING", SCE_PEER_KEPT_WAITING),
        ("SCE_PEER_ROOTS_ONLY", SCE_PEER_ROOTS_ONLY),
        ("SCE_PEER_ROOTS_AND_PUBLIC", SCE_PEER_ROOTS_AND_PUBLIC),
        ("SCE_PEER_ROUND_SYNC", SCE_PEER_ROUND_SYNC),
        ("SCE_PEER_ROUND_RESYNC", SCE_PEER_ROUND_RESYNC),
        ("SCE_PEER_ROUND_REREAD", SCE_PEER_ROUND_REREAD),
        ("SCE_PEER_PROFILE_UNTRUSTED", SCE_PEER_PROFILE_UNTRUSTED),
        ("SCE_PEER_PROFILE_TRUSTED_HUB", SCE_PEER_PROFILE_TRUSTED_HUB),
    ] {
        assert!(
            header.contains(&format!("#define {name} {value}\n")),
            "{name} is not {value} in the header"
        );
    }
    assert!(header.contains(&format!(
        "#define SCE_PEER_ABI_VERSION {}u",
        SCE_PEER_ABI_VERSION
    )));
}

/// A send the core cannot sign yet is kept, and the answer says so and why: here the device of an
/// invited person that has read nothing of its list. The standing counts it, and a send of
/// another content under its stamp is refused at once.
#[test]
fn a_send_the_core_cannot_sign_yet_is_kept_and_says_why() {
    let genesis = [0u8; 32];
    let seed = [2u8; 32];
    let write = c"{\"by\":\"2\",\"wallTime\":\"1000\",\"counter\":0,\"ops\":[{\"kind\":0,\"id\":1,\"field\":3,\"value\":\"1\",\"text\":\"\",\"wallTime\":\"0\",\"counter\":0,\"nodeId\":\"0\"}]}";
    let other = c"{\"by\":\"2\",\"wallTime\":\"1000\",\"counter\":0,\"ops\":[{\"kind\":0,\"id\":1,\"field\":3,\"value\":\"2\",\"text\":\"\",\"wallTime\":\"0\",\"counter\":0,\"nodeId\":\"0\"}]}";
    // SAFETY: each call below passes a live handle, NUL-terminated texts and places valid for a
    // write.
    unsafe {
        let peer = sce_peer_invited(c"home".as_ptr(), genesis.as_ptr());
        assert!(!peer.is_null(), "{}", last_error());
        assert_eq!(sce_peer_set_identity(peer, 2, seed.as_ptr()), SCE_PEER_OK);
        let mut kept = -1;
        let mut reason = empty();
        assert_eq!(
            sce_peer_outbound(
                peer,
                c"record_list.change".as_ptr(),
                write.as_ptr(),
                &mut kept,
                &mut reason
            ),
            SCE_PEER_OK,
            "{}",
            last_error()
        );
        assert_eq!(kept, SCE_PEER_KEPT_WAITING);
        let why = taken(reason).expect("a send that waits says why");
        assert!(why.contains("has not begun"), "{why}");
        assert_eq!(standing_of(peer)["waiting"], 1);

        let mut kept = -1;
        let mut reason = empty();
        assert_eq!(
            sce_peer_outbound(
                peer,
                c"record_list.change".as_ptr(),
                other.as_ptr(),
                &mut kept,
                &mut reason
            ),
            SCE_PEER_OK
        );
        assert_eq!(kept, SCE_PEER_KEPT_REFUSED);
        let why = taken(reason).expect("a refusal says why");
        assert!(why.contains("another content"), "{why}");
        assert_eq!(
            standing_of(peer)["waiting"],
            1,
            "the refused one is kept nowhere"
        );
        sce_peer_free(peer);
    }
}

/// A login through the ABI (ADR 0080): a core with no person is refused before anything is asked;
/// a login asks for a challenge for the device's node, signs nothing for a relay that names itself
/// otherwise than the device was configured for, and says in its outcome what the relay refused; a
/// link takes a token, and a host's own transport, whose credentials are the host's, takes none.
#[test]
fn a_login_through_the_abi_asks_for_a_challenge_and_says_what_came_of_it() {
    let mut relay = Relay::open(
        MemoryStore::new(),
        Authority::new([3; 32], "the-relay"),
        Limits::default(),
    )
    .unwrap()
    .with_login(Doorway::with_key(LoginPolicy::default(), [5; 32]));
    // SAFETY: each call below passes live handles, NUL-terminated texts, or null where refused.
    unsafe {
        let peer = sce_peer_open(c"home".as_ptr());
        assert_eq!(
            sce_peer_login(peer, c"the-relay".as_ptr()),
            SCE_PEER_ERR_NO_PERSON
        );
        assert_eq!(
            sce_peer_set_identity(peer, 1, [1u8; 32].as_ptr()),
            SCE_PEER_OK
        );
        assert_eq!(
            sce_peer_login(peer, std::ptr::null()),
            SCE_PEER_ERR_NULL_ARGUMENT
        );

        // Configured for another relay: the challenge is answered by this one, and nothing signed.
        assert_eq!(sce_peer_login(peer, c"another-relay".as_ptr()), SCE_PEER_OK);
        let challenge = text_of(peer, sce_peer_poll_request).expect("a challenge is asked");
        let asked: Value = serde_json::from_str(&challenge).unwrap();
        assert_eq!(asked["op"], "challenge", "{asked}");
        assert_eq!(asked["list"], "home");
        assert_eq!(asked["node"], "1");
        let answer = CString::new(relay.anonymous(None, &challenge, 1_000)).unwrap();
        assert_eq!(sce_peer_respond(peer, answer.as_ptr()), SCE_PEER_OK);
        assert_eq!(text_of(peer, sce_peer_poll_request), None);
        let stopped: Value =
            serde_json::from_str(&text_of(peer, sce_peer_take_outcome).unwrap()).unwrap();
        assert_eq!(stopped["failure"], "unexpected", "{stopped}");
        assert!(stopped.get("token").is_none(), "{stopped}");

        // Configured for this relay: the login is signed and carried, and the relay, which holds
        // no log of the list, refuses it; the outcome says so in the relay's word.
        assert_eq!(sce_peer_login(peer, c"the-relay".as_ptr()), SCE_PEER_OK);
        for _ in 0..2 {
            let request = text_of(peer, sce_peer_poll_request).expect("the login asks");
            let answer = CString::new(relay.anonymous(None, &request, 2_000)).unwrap();
            assert_eq!(sce_peer_respond(peer, answer.as_ptr()), SCE_PEER_OK);
        }
        assert_eq!(
            text_of(peer, sce_peer_poll_request),
            None,
            "the login is over"
        );
        let refused: Value =
            serde_json::from_str(&text_of(peer, sce_peer_take_outcome).unwrap()).unwrap();
        assert_eq!(refused["failure"], "relay", "{refused}");
        assert_eq!(refused["code"], "behind", "{refused}");

        // A link takes a token, and gives one up for none; a host's own transport takes none.
        let link = sce_peer_link_wss(c"ws://127.0.0.1:1".as_ptr(), std::ptr::null());
        assert!(!link.is_null(), "a link with no token: {}", last_error());
        assert_eq!(
            sce_peer_link_set_token(link, c"a-token".as_ptr()),
            SCE_PEER_OK
        );
        assert_eq!(sce_peer_link_set_token(link, std::ptr::null()), SCE_PEER_OK);
        assert_eq!(
            sce_peer_link_set_token(std::ptr::null_mut(), c"a-token".as_ptr()),
            SCE_PEER_ERR_NULL_ARGUMENT
        );
        unsafe extern "C" fn never(
            _: *mut std::ffi::c_void,
            _: *const u8,
            _: usize,
            _: u64,
            _: *mut ScePeerReply,
        ) -> i32 {
            SCE_PEER_TRANSPORT_LINK
        }
        let host = sce_peer_link_host(Some(never), None, std::ptr::null_mut());
        assert_eq!(
            sce_peer_link_set_token(host, c"a-token".as_ptr()),
            SCE_PEER_ERR_BAD_ARGUMENT
        );
        assert!(last_error().contains("host"), "{}", last_error());

        sce_peer_link_free(host);
        sce_peer_link_free(link);
        sce_peer_free(peer);
    }
}

/// What the parts of changes signed before a change was one unit make is given as one object of
/// restores and a cut (docs/adr/0085, section 8.3); a core that holds no part gives neither, and a
/// null buffer or handle is refused. The fold of parts held is the peer crate's to hold
/// (`a_device_saved_before_a_change_was_one_unit_keeps_its_list`). Red if the function is not
/// exported, or writes its object under other names.
#[test]
fn the_fold_is_given_as_restores_and_a_cut() {
    // SAFETY: each call below passes a live handle and a buffer valid for a write, or null where
    // refused.
    unsafe {
        let peer = sce_peer_open(c"home".as_ptr());
        let mut out = empty();
        assert_eq!(
            sce_peer_fold(peer, 64, &mut out),
            SCE_PEER_OK,
            "{}",
            last_error()
        );
        let fold: Value = serde_json::from_str(&taken(out).unwrap()).unwrap();
        assert_eq!(fold, json!({"restores": [], "cut": []}));
        assert_eq!(
            sce_peer_fold(peer, 64, std::ptr::null_mut()),
            SCE_PEER_ERR_NULL_ARGUMENT
        );
        let mut none = empty();
        assert_eq!(
            sce_peer_fold(std::ptr::null_mut(), 64, &mut none),
            SCE_PEER_ERR_NULL_ARGUMENT
        );
        assert!(none.ptr.is_null(), "nothing is given for no core");
        sce_peer_free(peer);
    }
}

#[test]
fn the_boundary_refuses_what_it_is_not_given_and_says_why() {
    let list = c"home";
    let seed = [1u8; 32];
    let genesis = [0u8; 32];
    let write = c"{\"by\":\"2\",\"wallTime\":\"1000\",\"counter\":0,\"ops\":[{\"kind\":0,\"id\":1,\"field\":3,\"value\":\"1\",\"text\":\"\",\"wallTime\":\"0\",\"counter\":0,\"nodeId\":\"0\"}]}";
    // SAFETY: each call below passes pointers that are null, or valid for the length given.
    unsafe {
        assert_eq!(sce_peer_abi_version(), SCE_PEER_ABI_VERSION);
        assert!(sce_peer_open(std::ptr::null::<c_char>()).is_null());
        assert!(last_error().contains("null"), "{}", last_error());
        let not_utf8 = c"\xff\xfe";
        assert!(sce_peer_open(not_utf8.as_ptr()).is_null());
        assert!(last_error().contains("UTF-8"), "{}", last_error());
        assert!(sce_peer_invited(list.as_ptr(), std::ptr::null()).is_null());

        let peer = sce_peer_open(list.as_ptr());
        assert!(!peer.is_null());
        assert_eq!(sce_peer_set_identity(peer, 1, seed.as_ptr()), SCE_PEER_OK);
        assert_eq!(
            sce_peer_set_identity(peer, 1, std::ptr::null()),
            SCE_PEER_ERR_NULL_ARGUMENT
        );
        assert_eq!(
            sce_peer_set_identity(std::ptr::null_mut(), 1, seed.as_ptr()),
            SCE_PEER_ERR_NULL_ARGUMENT
        );
        assert!(last_error().contains("handle"), "{}", last_error());

        // Nothing is asked before an exchange runs, and nothing is answered.
        assert_eq!(text_of(peer, sce_peer_poll_request), None);
        assert_eq!(
            sce_peer_respond(peer, c"{}".as_ptr()),
            SCE_PEER_ERR_NOTHING_ASKED
        );
        assert!(last_error().contains("NothingAsked"), "{}", last_error());
        assert_eq!(
            sce_peer_fail(peer, c"gone".as_ptr()),
            SCE_PEER_ERR_NOTHING_ASKED
        );
        assert_eq!(
            sce_peer_poll_request(peer, std::ptr::null_mut()),
            SCE_PEER_ERR_NULL_ARGUMENT
        );
        assert_eq!(sce_peer_start(peer, 7), SCE_PEER_ERR_BAD_ARGUMENT);
        assert!(last_error().contains("kind of round"), "{}", last_error());

        // An answer that is not a response of the protocol ends the round as a failure of the
        // transport; its outcome is taken once, and not into nowhere, and the next exchange is not
        // begun until it is taken.
        assert_eq!(sce_peer_start(peer, SCE_PEER_ROUND_SYNC), SCE_PEER_OK);
        assert_eq!(
            sce_peer_respond(peer, c"{}".as_ptr()),
            SCE_PEER_ERR_NOTHING_ASKED,
            "nothing was asked yet"
        );
        let request = text_of(peer, sce_peer_poll_request).expect("the round asks");
        assert!(request.contains("\"op\""), "{request}");
        assert_eq!(
            text_of(peer, sce_peer_poll_request).as_deref(),
            Some(request.as_str()),
            "asked again, it asks the same"
        );
        assert_eq!(
            sce_peer_respond(peer, c"not a response".as_ptr()),
            SCE_PEER_OK
        );
        assert_eq!(text_of(peer, sce_peer_poll_request), None);
        assert_eq!(
            sce_peer_start(peer, SCE_PEER_ROUND_SYNC),
            SCE_PEER_ERR_OUTCOME_NOT_TAKEN
        );
        assert_eq!(
            sce_peer_take_outcome(peer, std::ptr::null_mut()),
            SCE_PEER_ERR_NULL_ARGUMENT
        );
        let outcome: Value =
            serde_json::from_str(&text_of(peer, sce_peer_take_outcome).expect("the round ended"))
                .unwrap();
        assert_eq!(outcome["outcome"], "failed", "{outcome}");
        assert_eq!(outcome["failure"], "link", "{outcome}");
        assert_eq!(
            text_of(peer, sce_peer_take_outcome),
            None,
            "an outcome is taken once"
        );

        // A failure of the transport ends an exchange the same way, with its reason.
        assert_eq!(sce_peer_start(peer, SCE_PEER_ROUND_RESYNC), SCE_PEER_OK);
        assert!(text_of(peer, sce_peer_poll_request).is_some());
        assert_eq!(
            sce_peer_fail(peer, c"the connection closed".as_ptr()),
            SCE_PEER_OK
        );
        assert_eq!(text_of(peer, sce_peer_poll_request), None);
        let failed: Value =
            serde_json::from_str(&text_of(peer, sce_peer_take_outcome).unwrap()).unwrap();
        assert_eq!(failed["failure"], "link", "{failed}");
        assert!(
            failed["reason"]
                .as_str()
                .unwrap()
                .contains("the connection closed"),
            "{failed}"
        );

        // What is not signed is kept nowhere; the answer needs a place, the reason does not.
        let mut kept = -1;
        let mut reason = empty();
        assert_eq!(
            sce_peer_outbound(
                peer,
                c"record_list.edit".as_ptr(),
                c"{\"id\":1}".as_ptr(),
                &mut kept,
                &mut reason
            ),
            SCE_PEER_OK
        );
        assert_eq!(kept, SCE_PEER_KEPT_NOT_SIGNED);
        assert_eq!(taken(reason), None);
        assert_eq!(
            sce_peer_outbound(
                peer,
                c"record_list.edit".as_ptr(),
                c"{}".as_ptr(),
                std::ptr::null_mut(),
                std::ptr::null_mut()
            ),
            SCE_PEER_ERR_NULL_ARGUMENT
        );

        // A device with no person signs nothing, and says why.
        let nobody = sce_peer_invited(list.as_ptr(), genesis.as_ptr());
        assert!(!nobody.is_null());
        let mut kept = -1;
        let mut reason = empty();
        assert_eq!(
            sce_peer_outbound(
                nobody,
                c"record_list.change".as_ptr(),
                write.as_ptr(),
                &mut kept,
                &mut reason
            ),
            SCE_PEER_OK
        );
        assert_eq!(kept, SCE_PEER_KEPT_REFUSED);
        assert!(!taken(reason).expect("a refusal says why").is_empty());
        let mut kept = -1;
        assert_eq!(
            sce_peer_outbound(
                nobody,
                c"record_list.change".as_ptr(),
                write.as_ptr(),
                &mut kept,
                std::ptr::null_mut()
            ),
            SCE_PEER_OK,
            "the reason may be left"
        );
        assert_eq!(kept, SCE_PEER_KEPT_REFUSED);

        // The group's device: none is held until one is given, and a new device leaves a key
        // package.
        assert_eq!(
            sce_peer_publish_key_package(peer),
            SCE_PEER_ERR_NO_GROUP_DEVICE
        );
        assert_eq!(
            sce_peer_with_group(peer, 1, std::ptr::null()),
            SCE_PEER_ERR_NULL_ARGUMENT
        );
        assert_eq!(sce_peer_with_group(peer, 1, seed.as_ptr()), SCE_PEER_OK);
        assert_eq!(sce_peer_publish_key_package(peer), SCE_PEER_OK);
        let again = sce_peer_open(list.as_ptr());
        assert_eq!(sce_peer_for_a_hub(again), SCE_PEER_OK);
        assert_eq!(
            sce_peer_for_a_hub(std::ptr::null_mut()),
            SCE_PEER_ERR_NULL_ARGUMENT
        );

        // The outbox of a device that signed nothing: no record, an empty history and cut, no
        // mark owed and no own stamp, each an answer and not a failure. A change offered in
        // another's name is refused with why, an offer that is no change too, and what a text
        // must be is checked before anything is taken.
        assert_eq!(text_of(peer, sce_peer_take_records).as_deref(), Some("[]"));
        assert_eq!(text_of(peer, sce_peer_stable_cut).as_deref(), Some("[]"));
        let mut history = empty();
        assert_eq!(
            sce_peer_history(peer, std::ptr::null(), &mut history),
            SCE_PEER_OK,
            "{}",
            last_error()
        );
        let history: Value = serde_json::from_str(&taken(history).unwrap()).unwrap();
        assert_eq!(history["changes"], json!([]), "{history}");
        let mut history = empty();
        assert_eq!(
            sce_peer_history(peer, c"[[\"1\"]]".as_ptr(), &mut history),
            SCE_PEER_ERR_BAD_ARGUMENT
        );
        assert_eq!(
            sce_peer_history(peer, std::ptr::null(), std::ptr::null_mut()),
            SCE_PEER_ERR_NULL_ARGUMENT
        );
        let mut marked = u64::MAX;
        assert_eq!(sce_peer_mark(peer, &mut marked), SCE_PEER_OK);
        assert_eq!(marked, 0, "no mark is owed");
        let (mut wall, mut counter, mut read_back) = (-1i64, u32::MAX, -1i32);
        assert_eq!(
            sce_peer_own_stamp(peer, &mut wall, &mut counter, &mut read_back),
            SCE_PEER_OK
        );
        assert_eq!((wall, counter, read_back), (0, 0, 1));
        let mut kept = -1;
        let mut reason = empty();
        assert_eq!(
            sce_peer_made(peer, write.as_ptr(), &mut kept, &mut reason),
            SCE_PEER_OK
        );
        assert_eq!(
            kept, SCE_PEER_KEPT_REFUSED,
            "node 1 signs nothing in node 2's name"
        );
        assert!(!taken(reason).expect("a refusal says why").is_empty());
        let mut kept = -1;
        assert_eq!(
            sce_peer_made(
                peer,
                c"not a change".as_ptr(),
                &mut kept,
                std::ptr::null_mut()
            ),
            SCE_PEER_OK
        );
        assert_eq!(kept, SCE_PEER_KEPT_REFUSED);
        assert_eq!(
            sce_peer_made(
                peer,
                write.as_ptr(),
                std::ptr::null_mut(),
                std::ptr::null_mut()
            ),
            SCE_PEER_ERR_NULL_ARGUMENT
        );
        let mut restored = u64::MAX;
        assert_eq!(
            sce_peer_restore_records(peer, c"[]".as_ptr(), &mut restored),
            SCE_PEER_OK
        );
        assert_eq!(restored, 0);
        assert_eq!(
            sce_peer_restore_records(peer, c"{}".as_ptr(), std::ptr::null_mut()),
            SCE_PEER_ERR_BAD_ARGUMENT
        );
        assert_eq!(sce_peer_recorded(peer, 5), SCE_PEER_OK);

        // Bytes that are no sealed state are refused, and a sealed state that does not open under
        // the key for this list and node is refused with its own status; the core takes nothing of
        // either. A save is one sealed state and the key drawn for it, taken back by a core just
        // made for the list and given nothing else: its person comes back with the state.
        let fresh = sce_peer_open(list.as_ptr());
        let given = [5u8; 32];
        let not_sealed = b"{}";
        let none = std::ptr::null_mut();
        assert_eq!(
            sce_peer_restore_sealed(
                fresh,
                1,
                not_sealed.as_ptr(),
                not_sealed.len(),
                given.as_ptr(),
                none
            ),
            SCE_PEER_ERR_UNREADABLE
        );
        assert!(last_error().contains("Unreadable"), "{}", last_error());
        assert_eq!(
            sce_peer_restore_sealed(fresh, 1, std::ptr::null(), 0, given.as_ptr(), none),
            SCE_PEER_ERR_UNREADABLE
        );
        assert_eq!(
            sce_peer_restore_sealed(
                fresh,
                1,
                not_sealed.as_ptr(),
                not_sealed.len(),
                std::ptr::null(),
                none
            ),
            SCE_PEER_ERR_NULL_ARGUMENT
        );
        let mut state = empty();
        let mut drawn = [0u8; 32];
        assert_eq!(
            sce_peer_save_sealed(peer, std::ptr::null_mut(), drawn.as_mut_ptr()),
            SCE_PEER_ERR_NULL_ARGUMENT
        );
        assert_eq!(
            sce_peer_save_sealed(peer, &mut state, std::ptr::null_mut()),
            SCE_PEER_ERR_NULL_ARGUMENT
        );
        assert_eq!(
            sce_peer_save_sealed(peer, &mut state, drawn.as_mut_ptr()),
            SCE_PEER_OK,
            "{}",
            last_error()
        );
        let sealed = bytes_of(state);
        assert_ne!(drawn, [0u8; 32], "the key drawn for the save is given");
        let wrong = [6u8; 32];
        assert_eq!(
            sce_peer_restore_sealed(
                fresh,
                1,
                sealed.as_ptr(),
                sealed.len(),
                wrong.as_ptr(),
                none
            ),
            SCE_PEER_ERR_NOT_OPENED
        );
        assert!(last_error().contains("NotOpened"), "{}", last_error());
        assert_eq!(
            sce_peer_restore_sealed(
                fresh,
                2,
                sealed.as_ptr(),
                sealed.len(),
                drawn.as_ptr(),
                none
            ),
            SCE_PEER_ERR_NOT_OPENED,
            "another node's"
        );
        assert_eq!(standing_of(fresh)["node"], Value::Null, "nothing taken");
        let mut count = u64::MAX;
        assert_eq!(
            sce_peer_restore_sealed(
                fresh,
                1,
                sealed.as_ptr(),
                sealed.len(),
                drawn.as_ptr(),
                &mut count
            ),
            SCE_PEER_OK,
            "{}",
            last_error()
        );
        assert_eq!(count, 0, "the core held no operation");
        assert_eq!(
            standing_of(fresh)["node"],
            "1",
            "the person came back with no seed given"
        );

        // Under a key the lane gives, the same state is the same bytes; a core with no person
        // seals nothing.
        let (mut first, mut second) = (empty(), empty());
        assert_eq!(
            sce_peer_save_sealed_under(fresh, given.as_ptr(), &mut first),
            SCE_PEER_OK
        );
        assert_eq!(
            sce_peer_save_sealed_under(fresh, given.as_ptr(), &mut second),
            SCE_PEER_OK
        );
        assert_eq!(bytes_of(first), bytes_of(second));
        let mut third = empty();
        assert_eq!(
            sce_peer_save_sealed_under(fresh, std::ptr::null(), &mut third),
            SCE_PEER_ERR_NULL_ARGUMENT
        );
        assert!(third.ptr.is_null());
        assert_eq!(
            sce_peer_save_sealed(nobody, &mut third, drawn.as_mut_ptr()),
            SCE_PEER_ERR_NO_PERSON
        );
        assert!(third.ptr.is_null());

        sce_peer_buf_free(empty());
        sce_peer_free(fresh);
        sce_peer_free(again);
        sce_peer_free(nobody);
        sce_peer_free(peer);
        sce_peer_free(std::ptr::null_mut());
    }
}

/// The bytes of a buffer the library gave, which is given back; it is not empty.
///
/// # Safety
/// `buf` is one the library returned and has not given back.
unsafe fn bytes_of(buf: ScePeerBuf) -> Vec<u8> {
    assert!(!buf.ptr.is_null(), "the library gave bytes");
    // SAFETY: the library's buffer is `len` readable bytes.
    let bytes = unsafe { std::slice::from_raw_parts(buf.ptr, buf.len) }.to_vec();
    // SAFETY: returned by the library and not given back yet.
    unsafe { sce_peer_buf_free(buf) };
    bytes
}

/// The seed of the administrator's key in the tests of the administrator's acts.
const ADMINISTRATOR: [u8; 32] = [9; 32];

/// The public key (32 bytes) the seed `seed` makes.
fn public_of(seed: [u8; 32]) -> [u8; 32] {
    Identity::from_seed(seed).public().to_bytes()
}

/// What `peer` stands at in its list.
///
/// # Safety
/// `peer` is a live handle.
unsafe fn standing_of(peer: *mut ScePeer) -> Value {
    // SAFETY: `peer` is live.
    let text = unsafe { text_of(peer, sce_peer_standing) }.expect("a standing is a text");
    serde_json::from_str(&text).expect("a standing is JSON")
}

/// A core of the list `home`: opened by whoever makes it, or invited with `genesis`; with the person
/// of `node` whose key `seed` makes, and the administrator's key when `administrator`.
fn core_of(
    node: u64,
    seed: [u8; 32],
    genesis: Option<[u8; 32]>,
    administrator: Option<[u8; 32]>,
) -> *mut ScePeer {
    let list = c"home";
    // SAFETY: the list is NUL-terminated, and the genesis, the seeds are 32 bytes each.
    unsafe {
        let peer = match genesis {
            None => sce_peer_open(list.as_ptr()),
            Some(genesis) => sce_peer_invited(list.as_ptr(), genesis.as_ptr()),
        };
        assert!(!peer.is_null(), "{}", last_error());
        assert_eq!(
            sce_peer_set_identity(peer, node, seed.as_ptr()),
            SCE_PEER_OK
        );
        if let Some(administrator) = administrator {
            assert_eq!(
                sce_peer_set_administrator(peer, administrator.as_ptr()),
                SCE_PEER_OK
            );
        }
        peer
    }
}

/// An administrator's act the core cannot do is refused with its own status, says why, and writes
/// nothing: no administrator's key, no person, a list opened already, a genesis that is not the one
/// the device was invited with, a role that is none, a device that is no member, a device that is
/// admitted to a list keyed by its group without its key package.
#[test]
fn an_administrators_act_the_core_cannot_do_is_refused_and_says_why() {
    let (alice, bob) = ([1u8; 32], [2u8; 32]);
    let bob_key = public_of(bob);
    let mut genesis = [0u8; 32];
    // SAFETY: each call below passes live handles, and pointers that are null or valid for the
    // length the function's contract names.
    unsafe {
        // A core with no administrator's key opens nothing and grants and takes off no one.
        let plain = core_of(1, alice, None, None);
        assert_eq!(
            sce_peer_open_list(plain, SCE_PEER_PROFILE_UNTRUSTED, genesis.as_mut_ptr()),
            SCE_PEER_ERR_NO_ADMINISTRATOR
        );
        assert!(last_error().contains("NoAdministrator"), "{}", last_error());
        assert_eq!(
            sce_peer_grant(plain, 2, 2, bob_key.as_ptr()),
            SCE_PEER_ERR_NO_ADMINISTRATOR
        );
        assert_eq!(
            sce_peer_remove_member(plain, 2),
            SCE_PEER_ERR_NO_ADMINISTRATOR
        );
        assert_eq!(
            sce_peer_set_administrator(plain, std::ptr::null()),
            SCE_PEER_ERR_NULL_ARGUMENT
        );
        assert_eq!(standing_of(plain)["administrator"], false);

        // A core with the administrator's key and no person opens no list it would not be in.
        let nobody = sce_peer_open(c"home".as_ptr());
        assert_eq!(
            sce_peer_set_administrator(nobody, ADMINISTRATOR.as_ptr()),
            SCE_PEER_OK
        );
        assert_eq!(
            sce_peer_open_list(nobody, SCE_PEER_PROFILE_UNTRUSTED, genesis.as_mut_ptr()),
            SCE_PEER_ERR_NO_PERSON
        );
        assert_eq!(standing_of(nobody)["genesis"], Value::Null);

        // The administrator's device: the place for the genesis and the profile are asked for
        // before anything is written, and a list is opened once.
        let admin = core_of(1, alice, None, Some(ADMINISTRATOR));
        assert_eq!(
            sce_peer_open_list(admin, SCE_PEER_PROFILE_UNTRUSTED, std::ptr::null_mut()),
            SCE_PEER_ERR_NULL_ARGUMENT
        );
        assert_eq!(
            sce_peer_open_list(admin, 7, genesis.as_mut_ptr()),
            SCE_PEER_ERR_BAD_ARGUMENT
        );
        assert!(last_error().contains("profile"), "{}", last_error());
        assert_eq!(
            standing_of(admin)["genesis"],
            Value::Null,
            "nothing written"
        );
        assert_eq!(
            sce_peer_open_list(admin, SCE_PEER_PROFILE_UNTRUSTED, genesis.as_mut_ptr()),
            SCE_PEER_OK
        );
        let opened = standing_of(admin);
        assert_eq!(opened["genesis"], encoding::encode(&genesis), "{opened}");
        assert_eq!(opened["administrator"], true, "{opened}");
        assert_eq!(opened["members"], json!([{"node": "1", "role": 3}]));
        let mut again = [0u8; 32];
        assert_eq!(
            sce_peer_open_list(admin, SCE_PEER_PROFILE_UNTRUSTED, again.as_mut_ptr()),
            SCE_PEER_ERR_OPENED
        );

        // A role is 1 to 3, and a number wider than a role is refused, not cut to one.
        for role in [0, 4, 258] {
            assert_eq!(
                sce_peer_grant(admin, 2, role, bob_key.as_ptr()),
                SCE_PEER_ERR_BAD_ARGUMENT,
                "{role}"
            );
        }
        assert_eq!(
            sce_peer_grant(admin, 2, 2, std::ptr::null()),
            SCE_PEER_ERR_NULL_ARGUMENT
        );
        assert_eq!(sce_peer_remove_member(admin, 2), SCE_PEER_ERR_NOT_A_MEMBER);
        assert_eq!(
            sce_peer_add_member(admin, 2, 2, bob_key.as_ptr()),
            SCE_PEER_ERR_NO_GROUP_DEVICE
        );
        assert_eq!(standing_of(admin)["members"].as_array().unwrap().len(), 1);
        assert_eq!(
            sce_peer_grant(admin, 2, 2, bob_key.as_ptr()),
            SCE_PEER_OK,
            "{}",
            last_error()
        );
        assert_eq!(
            standing_of(admin)["members"],
            json!([{"node": "1", "role": 3}, {"node": "2", "role": 2}])
        );
        assert_eq!(sce_peer_remove_member(admin, 2), SCE_PEER_OK);
        assert_eq!(
            standing_of(admin)["members"],
            json!([{"node": "1", "role": 3}])
        );
        assert_eq!(
            sce_peer_standing(admin, std::ptr::null_mut()),
            SCE_PEER_ERR_NULL_ARGUMENT
        );

        // A device invited to the list opens no other: its log takes only the genesis it was
        // invited with, and another administrator's is not that one.
        let invited = core_of(1, alice, Some(genesis), Some([8; 32]));
        assert_eq!(
            sce_peer_open_list(invited, SCE_PEER_PROFILE_UNTRUSTED, again.as_mut_ptr()),
            SCE_PEER_ERR_LOG
        );
        assert!(last_error().contains("NotTheGenesis"), "{}", last_error());
        assert_eq!(
            standing_of(invited)["genesis"],
            Value::Null,
            "nothing written"
        );

        // A list keyed by its group admits a device only with its key package, and none was read;
        // another role for a member the group holds is a grant alone.
        let grouped = core_of(1, alice, None, Some(ADMINISTRATOR));
        assert_eq!(sce_peer_with_group(grouped, 1, alice.as_ptr()), SCE_PEER_OK);
        assert_eq!(
            sce_peer_open_list(grouped, SCE_PEER_PROFILE_UNTRUSTED, again.as_mut_ptr()),
            SCE_PEER_OK
        );
        assert_eq!(
            sce_peer_grant(grouped, 2, 2, bob_key.as_ptr()),
            SCE_PEER_ERR_ADMITTED_THROUGH_GROUP
        );
        assert_eq!(
            sce_peer_add_member(grouped, 2, 2, bob_key.as_ptr()),
            SCE_PEER_ERR_NO_KEY_PACKAGE
        );
        assert_eq!(
            sce_peer_grant(grouped, 1, 2, public_of(alice).as_ptr()),
            SCE_PEER_OK,
            "{}",
            last_error()
        );
        assert_eq!(standing_of(grouped)["role"], 2);

        for peer in [plain, nobody, admin, invited, grouped] {
            sce_peer_free(peer);
        }
    }
}

/// The relay of a test in this process, and the authority whose tokens it takes.
struct Network {
    relay: Relay<MemoryStore>,
    authority: Authority,
    /// The relay's clock: a second further at each request, so that no device spends its allowance.
    now: u64,
}

impl Network {
    fn new() -> Self {
        let authority = Authority::new([3; 32], "the-relay");
        Self {
            relay: Relay::open(MemoryStore::new(), authority.clone(), Limits::default()).unwrap(),
            authority,
            now: 0,
        }
    }

    /// The exchange `peer`, the device of `node`, runs, every request carried to the relay and every
    /// answer back, and what it came to.
    ///
    /// # Safety
    /// `peer` is a live handle.
    unsafe fn finish(&mut self, peer: *mut ScePeer, node: u64) -> Value {
        // SAFETY: `peer` is live, and every text given to it is NUL-terminated.
        unsafe {
            while let Some(request) = text_of(peer, sce_peer_poll_request) {
                let token = self.authority.issue("home", node, u64::MAX);
                self.now += 1000;
                let response = CString::new(self.relay.handle(&token, &request, self.now)).unwrap();
                assert_eq!(
                    sce_peer_respond(peer, response.as_ptr()),
                    SCE_PEER_OK,
                    "{}",
                    last_error()
                );
            }
            let outcome = text_of(peer, sce_peer_take_outcome).expect("the exchange ended");
            serde_json::from_str(&outcome).expect("an outcome is JSON")
        }
    }

    /// A round of `peer`, the device of `node`, carried to its end.
    ///
    /// # Safety
    /// `peer` is a live handle.
    unsafe fn round(&mut self, peer: *mut ScePeer, node: u64) -> Value {
        // SAFETY: `peer` is live.
        unsafe {
            assert_eq!(sce_peer_start(peer, SCE_PEER_ROUND_SYNC), SCE_PEER_OK);
            self.finish(peer, node)
        }
    }
}

/// An administrator's device opens a list keyed by its group, admits bob's device from the key
/// package a round read from the relay, and takes it off again, each in one call through the ABI;
/// the commit and the Welcome go to the relay after the next round. Bob's device joins from the
/// Welcome and holds the key of the epoch alice's holds (what she writes under it opens on his), and
/// once he is taken off it holds no key of the epoch after. What MLS draws at random is not compared:
/// the properties are.
#[test]
fn the_administrators_acts_on_a_group_go_through_the_abi() {
    let (alice, bob) = ([1u8; 32], [2u8; 32]);
    let mut network = Network::new();
    let mut genesis = [0u8; 32];
    // SAFETY: each call below passes live handles, and pointers that are null or valid for the
    // length the function's contract names.
    unsafe {
        let admin = core_of(1, alice, None, Some(ADMINISTRATOR));
        assert_eq!(sce_peer_with_group(admin, 1, alice.as_ptr()), SCE_PEER_OK);
        assert_eq!(
            sce_peer_open_list(admin, SCE_PEER_PROFILE_UNTRUSTED, genesis.as_mut_ptr()),
            SCE_PEER_OK,
            "{}",
            last_error()
        );
        let opened = network.round(admin, 1);
        assert_eq!(opened["refused"], json!([]), "{opened}");

        // Bob's device leaves a key package with the relay, and alice's next round reads it.
        let joiner = core_of(2, bob, Some(genesis), None);
        assert_eq!(sce_peer_with_group(joiner, 2, bob.as_ptr()), SCE_PEER_OK);
        assert_eq!(sce_peer_publish_key_package(joiner), SCE_PEER_OK);
        let left = network.finish(joiner, 2);
        assert_eq!(left["outcome"], "ended", "{left}");
        network.round(admin, 1);

        // One call admits him to the list and to the group; the relay is owed the commit, his
        // Welcome and the key of the epoch the opening began until the next round gives them.
        assert_eq!(
            sce_peer_add_member(admin, 2, 2, public_of(bob).as_ptr()),
            SCE_PEER_OK,
            "{}",
            last_error()
        );
        assert_eq!(standing_of(admin)["owed"], 3);
        let admitted = network.round(admin, 1);
        assert_eq!(admitted["refused"], json!([]), "{admitted}");
        assert_eq!(admitted["owed"], 0, "{admitted}");
        assert_eq!(standing_of(admin)["owed"], 0);

        // He joins from the Welcome, both hold the key of the epoch it began, and what alice writes
        // under it opens on his device.
        let joined = network.round(joiner, 2);
        assert_eq!(
            joined["steps"],
            json!([{"step": "joined", "epoch": "2"}]),
            "{joined}"
        );
        assert_eq!(standing_of(joiner)["keys"], json!([1, 2]));
        assert_eq!(standing_of(admin)["keys"], json!([1, 2]));

        // His device is stopped and made again from its sealed state, given nothing but the list
        // and the genesis: its person, its group's device and the key of the epoch come back, and
        // what alice writes next opens on it.
        let mut state = empty();
        let mut key = [0u8; 32];
        assert_eq!(
            sce_peer_save_sealed(joiner, &mut state, key.as_mut_ptr()),
            SCE_PEER_OK,
            "{}",
            last_error()
        );
        let sealed = bytes_of(state);
        sce_peer_free(joiner);
        let joiner = sce_peer_invited(c"home".as_ptr(), genesis.as_ptr());
        assert_eq!(
            sce_peer_restore_sealed(
                joiner,
                2,
                sealed.as_ptr(),
                sealed.len(),
                key.as_ptr(),
                std::ptr::null_mut()
            ),
            SCE_PEER_OK,
            "{}",
            last_error()
        );
        let restored = standing_of(joiner);
        assert_eq!(restored["keys"], json!([1, 2]), "{restored}");
        assert_eq!(restored["awaits_own_chain"], true, "{restored}");
        let back = network.round(joiner, 2);
        assert_eq!(back["refused"], json!([]), "{back}");
        let write = c"{\"by\":\"1\",\"wallTime\":\"3000\",\"counter\":0,\"ops\":[{\"kind\":0,\"id\":1,\"field\":3,\"value\":\"5\",\"text\":\"\",\"wallTime\":\"0\",\"counter\":0,\"nodeId\":\"0\"}]}";
        let mut kept = -1;
        assert_eq!(
            sce_peer_outbound(
                admin,
                c"record_list.change".as_ptr(),
                write.as_ptr(),
                &mut kept,
                std::ptr::null_mut()
            ),
            SCE_PEER_OK
        );
        assert_eq!(kept, SCE_PEER_KEPT_TAKEN);
        network.round(admin, 1);
        let told = network.round(joiner, 2);
        let changes: Vec<Value> = told["events"]
            .as_array()
            .expect("an ended round tells events")
            .iter()
            .filter(|pair| pair[0] == "record_list.change")
            .map(|pair| serde_json::from_str(pair[1].as_str().unwrap()).unwrap())
            .collect();
        assert_eq!(changes.len(), 1, "{told}");
        assert_eq!(
            changes[0]["ops"][0]["value"], "5",
            "opened under the group's key"
        );

        // One call takes him off the list and the group: the group moves to an epoch his device
        // holds no key of, and the relay reads him nothing more.
        assert_eq!(
            sce_peer_remove_member(admin, 2),
            SCE_PEER_OK,
            "{}",
            last_error()
        );
        assert_eq!(standing_of(admin)["owed"], 1, "the commit, and no Welcome");
        let removed = network.round(admin, 1);
        assert_eq!(removed["refused"], json!([]), "{removed}");
        assert_eq!(removed["owed"], 0, "{removed}");
        let after = standing_of(admin);
        assert_eq!(after["keys"], json!([1, 2, 3]), "{after}");
        assert_eq!(after["members"], json!([{"node": "1", "role": 3}]));
        let cut = network.round(joiner, 2);
        assert_eq!(cut["outcome"], "ended", "{cut}");
        let off = standing_of(joiner);
        assert_eq!(off["role"], 0, "{off}");
        assert_eq!(
            off["keys"],
            json!([1, 2]),
            "no key of the epoch after: {off}"
        );

        sce_peer_free(joiner);
        sce_peer_free(admin);
    }
}

/// A text as the token the C, C++ and Swift replays read: `x` and its UTF-8 bytes in hex.
fn token(text: &str) -> String {
    let mut token = String::with_capacity(1 + 2 * text.len());
    token.push('x');
    for byte in text.bytes() {
        token += &format!("{byte:02x}");
    }
    token
}

/// The scenario as the tokens the C, C++ and Swift replays read (described in `tests/c/replay.c`).
fn script_of(scenario: &Value) -> String {
    let devices = scenario["devices"].as_array().unwrap();
    let mut script = format!(
        "{} {}\n",
        token(scenario["list"].as_str().unwrap()),
        devices.len()
    );
    for device in devices {
        script += &format!(
            "{} {} {} {}\n",
            device["node"],
            device["seed"].as_str().unwrap(),
            device["genesis"].as_str().unwrap_or("-"),
            device["administrator"].as_str().unwrap_or("-")
        );
    }
    for step in scenario["steps"].as_array().unwrap() {
        let device = &step["device"];
        let text = |name: &str| token(step[name].as_str().unwrap());
        let word = |name: &str| step[name].as_str().unwrap().to_string();
        let op = step["op"].as_str().unwrap();
        script += &match op {
            "make" => format!("make {device}"),
            "open" => format!("open {device} {} {}", word("profile"), word("genesis")),
            "grant" => format!(
                "grant {device} {} {} {}",
                step["node"],
                step["role"],
                word("key")
            ),
            "remove" => format!("remove {device} {}", step["node"]),
            "save" => format!("save {device} {} {}", word("key"), word("state")),
            "restore" => format!(
                "restore {device} {} {} {}",
                word("key"),
                word("state"),
                step["count"]
            ),
            "outbound" => format!(
                "outbound {device} {} {} {} {}",
                text("event"),
                text("data"),
                word("kept"),
                text("reason")
            ),
            "start" => format!("start {device} {}", word("kind")),
            "login" => format!("login {device} {}", text("audience")),
            "invite" => format!(
                "invite {device} {} {} {} {} {} {} {}",
                text("relay"),
                text("audience"),
                step["node"],
                step["now_ms"],
                step["valid_ms"],
                word("secret"),
                text("text")
            ),
            "invitation" => format!(
                "invitation {device} {} {}",
                text("invitation"),
                text("text")
            ),
            "make_invited" => format!("make_invited {device} {}", text("invitation")),
            "join" => format!(
                "join {device} {} {}",
                text("invitation"),
                text("fingerprint")
            ),
            "joined" => format!("joined {device} {} {}", step["node"], text("text")),
            "made" => format!(
                "made {device} {} {} {}",
                text("data"),
                word("kept"),
                text("reason")
            ),
            "recorded" => format!("recorded {device} {}", step["through"]),
            "restore_records" => format!(
                "restore_records {device} {} {}",
                text("records"),
                step["count"]
            ),
            "mark" => format!("mark {device} {}", step["seq"]),
            "fold" => format!("fold {device} {} {}", step["parts"], text("text")),
            "own_stamp" => format!(
                "own_stamp {device} {} {} {}",
                step["wall_time"], step["counter"], step["read_back"]
            ),
            "request" | "respond" | "outcome" | "standing" | "records" | "history"
            | "stable_cut" => {
                format!("{op} {device} {}", text("text"))
            }
            "fail" => format!("fail {device} {}", text("reason")),
            other => panic!("`{other}` is no step of the scenario"),
        };
        script.push('\n');
    }
    script
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sce-peer-capi-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_nanos())
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A missing tool is a skip unless the lane claims it ran.
fn skip(what: &str) {
    assert!(
        std::env::var_os("SCE_REQUIRE_TOOLS").is_none(),
        "SCE_REQUIRE_TOOLS is set and {what} is missing"
    );
    eprintln!("skipped: {what} is missing");
}

/// Whether `tool` runs; a missing one is a skip unless the lane claims it ran.
fn have(tool: &str, version_flag: &str) -> bool {
    let present = Command::new(tool)
        .arg(version_flag)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false);
    if !present {
        skip(&format!("`{tool}`"));
    }
    present
}

/// The shared library this crate builds, which the lanes load. A test binary links the Rust library
/// and does not build the shared one, so it is built here, into the same target directory.
fn library() -> PathBuf {
    let exe = std::env::current_exe().unwrap();
    let profile_dir = exe.parent().unwrap().parent().unwrap().to_path_buf();
    let mut build = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string()));
    build
        .args(["build", "--lib", "--locked", "--manifest-path"])
        .arg(Path::new(MANIFEST).join("Cargo.toml"));
    if profile_dir
        .file_name()
        .is_some_and(|name| name == "release")
    {
        build.arg("--release");
    }
    let output = build.output().expect("cargo runs");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    for name in [
        "libsce_peer_capi.so",
        "libsce_peer_capi.dylib",
        "sce_peer_capi.dll",
    ] {
        let path = profile_dir.join(name);
        if path.exists() {
            return path;
        }
    }
    panic!("the shared library is not in {}", profile_dir.display());
}

fn replay_in(command: &mut Command, what: &str, steps: usize) {
    let output = command
        .output()
        .unwrap_or_else(|error| panic!("{what}: {error}"));
    assert!(
        output.status.success(),
        "{what} failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains(&format!("replayed {steps} steps")),
        "{what}: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}

/// The scenario written as the token script, in `dir`.
fn script_in(dir: &Path, scenario: &Value) -> PathBuf {
    let script = dir.join("scenario.txt");
    std::fs::write(&script, script_of(scenario)).unwrap();
    script
}

/// Compile `source` with `compiler` and the flags the lane needs, against the header and the
/// shared library, into `program`.
fn compile(compiler: &str, flags: &[&str], source: &Path, program: &Path, library: &Path) {
    let libdir = library.parent().unwrap();
    let compiled = Command::new(compiler)
        .args(flags)
        .arg("-o")
        .arg(program)
        .arg(source)
        .arg("-I")
        .arg(Path::new(MANIFEST).join("include"))
        .arg("-L")
        .arg(libdir)
        .arg("-lsce_peer_capi")
        .arg(format!("-Wl,-rpath,{}", libdir.display()))
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
}

#[test]
fn the_scenario_replays_from_c() {
    if !have("cc", "--version") {
        return;
    }
    let scenario = scenario();
    let library = library();
    let dir = scratch("c");
    let script = script_in(&dir, &scenario);
    let program = dir.join("replay");
    compile(
        "cc",
        &["-Wall", "-Werror"],
        &Path::new(MANIFEST).join("tests/c/replay.c"),
        &program,
        &library,
    );
    replay_in(
        Command::new(&program).arg(&script),
        "the C replay",
        steps_of(&scenario),
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_scenario_replays_from_cpp() {
    if !have("c++", "--version") {
        return;
    }
    let scenario = scenario();
    let library = library();
    let dir = scratch("cpp");
    let script = script_in(&dir, &scenario);
    let program = dir.join("replay");
    compile(
        "c++",
        &["-std=c++17", "-Wall", "-Wextra", "-Werror"],
        &Path::new(MANIFEST).join("tests/cpp/replay.cpp"),
        &program,
        &library,
    );
    replay_in(
        Command::new(&program).arg(&script),
        "the C++ replay",
        steps_of(&scenario),
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_scenario_replays_from_python() {
    if !have("python3", "--version") {
        return;
    }
    let scenario = scenario();
    let library = library();
    replay_in(
        Command::new("python3")
            .arg("-I")
            .arg(Path::new(MANIFEST).join("tests/python/replay.py"))
            .arg(&library)
            .arg(scenario_path()),
        "the Python replay",
        steps_of(&scenario),
    );
}

/// The Kotlin lane needs its JVM program built: `./gradlew :sce-kotlin-peer-replay:installDist`.
#[test]
fn the_scenario_replays_from_kotlin() {
    if !have("java", "-version") {
        return;
    }
    let lib = Path::new(MANIFEST)
        .join("../../kotlin/peer-replay/build/install/sce-kotlin-peer-replay/lib");
    let built = std::fs::read_dir(&lib).is_ok_and(|entries| entries.count() > 0);
    if !built {
        skip(&format!(
            "{} (`./gradlew :sce-kotlin-peer-replay:installDist`)",
            lib.display()
        ));
        return;
    }
    let scenario = scenario();
    let library = library();
    replay_in(
        Command::new("java")
            .arg("-cp")
            .arg(lib.join("*"))
            .arg("com.sce.peer.ReplayKt")
            .arg(&library)
            .arg(scenario_path()),
        "the Kotlin replay",
        steps_of(&scenario),
    );
}

#[test]
fn the_scenario_replays_from_go() {
    if !have("go", "version") {
        return;
    }
    let scenario = scenario();
    let library = library();
    let dir = scratch("go");
    std::fs::copy(
        Path::new(MANIFEST).join("tests/go/replay.go"),
        dir.join("replay.go"),
    )
    .unwrap();
    let include = Path::new(MANIFEST).join("include");
    let libdir = library.parent().unwrap();
    replay_in(
        Command::new("go")
            .current_dir(&dir)
            .env("CGO_ENABLED", "1")
            .env("GO111MODULE", "off")
            .env("CGO_CFLAGS", format!("-I{}", include.display()))
            .env(
                "CGO_LDFLAGS",
                format!("-L{0} -lsce_peer_capi -Wl,-rpath,{0}", libdir.display()),
            )
            .arg("run")
            .arg("replay.go")
            .arg(scenario_path()),
        "the Go replay",
        steps_of(&scenario),
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// The Swift compiler: `swiftc` on the path, or the one beside what the newest toolchain installed
/// by hand under `~/.local/share/swift/<version>/bin/swift` links to, as `scripts/test_swift_backend.sh`
/// finds it.
fn swiftc() -> Option<PathBuf> {
    let on_path = Command::new("swiftc")
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success());
    if on_path {
        return Some(PathBuf::from("swiftc"));
    }
    let home = std::env::var_os("HOME")?;
    let mut links: Vec<PathBuf> = std::fs::read_dir(Path::new(&home).join(".local/share/swift"))
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path().join("bin/swift"))
        .filter(|link| link.exists())
        .collect();
    links.sort();
    let toolchain = std::fs::canonicalize(links.pop()?).ok()?;
    let swiftc = toolchain.parent()?.join("swiftc");
    swiftc.exists().then_some(swiftc)
}

/// The Swift lane reaches the header as the Clang module `CScePeer` (`include/module.modulemap`).
#[test]
fn the_scenario_replays_from_swift() {
    let Some(swiftc) = swiftc() else {
        skip("`swiftc`");
        return;
    };
    let scenario = scenario();
    let library = library();
    let dir = scratch("swift");
    let script = script_in(&dir, &scenario);
    let program = dir.join("replay");
    let libdir = library.parent().unwrap();
    let compiled = Command::new(&swiftc)
        .arg("-parse-as-library")
        .arg("-o")
        .arg(&program)
        .arg(Path::new(MANIFEST).join("tests/swift/replay.swift"))
        .arg("-I")
        .arg(Path::new(MANIFEST).join("include"))
        .arg("-L")
        .arg(libdir)
        .arg("-lsce_peer_capi")
        .arg("-Xlinker")
        .arg("-rpath")
        .arg("-Xlinker")
        .arg(libdir)
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    replay_in(
        Command::new(&program).arg(&script),
        "the Swift replay",
        steps_of(&scenario),
    );
    std::fs::remove_dir_all(&dir).unwrap();
}
