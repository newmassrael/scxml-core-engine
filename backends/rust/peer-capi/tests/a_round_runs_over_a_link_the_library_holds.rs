// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
//
// A lane that does not carry the relay's texts itself runs a round over a link the library holds,
// through the C ABI. What is asked: that a link is made and freed, and the boundary refuses what it
// is not given; that a round runs over the relay's WebSocket binding against a relay in this process,
// carries a write from one device to another, and the link tells its host it connected; that a
// device handed no token logs in over a link made with none and runs its rounds with the token;
// that a
// `wss://` link refuses a relay's certificate no public authority issued and the outcome says why,
// and one made believing a deployment's own authority, given as a file's bytes in PEM or in DER,
// alone or with the public authorities, reaches the relay;
// that a round over a transport the host carries itself (one function the library calls) asks what a host
// carrying the texts by hand asks and comes to the same outcome, says when the host's transport
// failed or the token was refused, is told the time it may take, and that the host's context is
// released once with the link; and that a run ends when its cancellation is raised from another
// thread, when its deadline passes, and when the network changes, each with an outcome that says
// so, long before the run would otherwise.

#![deny(unsafe_op_in_unsafe_fn)]

use std::ffi::{c_void, CStr};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use sce_peer_capi::*;
use sce_rust_relay::serve::{serve, serve_tls};
use sce_rust_relay::{Authority, Doorway, Limits, LoginPolicy, MemoryStore, Relay};
use serde_json::Value;

const ALICE_SEED: [u8; 32] = [1; 32];
const BOB_SEED: [u8; 32] = [2; 32];
const ADMINISTRATOR: [u8; 32] = [9; 32];
const ALICE: u64 = 1;
const BOB: u64 = 2;

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

/// What the exchange that ended came to.
///
/// # Safety
/// `peer` is a live handle.
unsafe fn outcome_of(peer: *mut ScePeer) -> Value {
    // SAFETY: `peer` is live.
    let text = unsafe { text_of(peer, sce_peer_take_outcome) }.expect("the exchange ended");
    serde_json::from_str(&text).expect("an outcome is JSON")
}

/// The notices of `link`, as the JSON array it gives; `None` when there are none.
///
/// # Safety
/// `link` is a live handle no run uses.
unsafe fn notices_of(link: *mut ScePeerLink) -> Option<Value> {
    let mut out = empty();
    // SAFETY: `link` is live and `out` is valid for a write.
    assert_eq!(
        unsafe { sce_peer_link_take_notices(link, &mut out) },
        SCE_PEER_OK,
        "{}",
        last_error()
    );
    // SAFETY: what the call just gave.
    unsafe { taken(out) }.map(|text| serde_json::from_str(&text).expect("notices are JSON"))
}

/// Alice's core, which holds the administrator's key, opens the list `home` and grants bob writing;
/// and the hash of the genesis.
fn alice() -> (*mut ScePeer, [u8; 32]) {
    let bob_key = sce_rust_trust::Identity::from_seed(BOB_SEED)
        .public()
        .to_bytes();
    let mut genesis = [0u8; 32];
    // SAFETY: the list is NUL-terminated, the seeds and the key are 32 bytes, the genesis is room
    // for 32.
    unsafe {
        let peer = sce_peer_open(c"home".as_ptr());
        assert!(!peer.is_null(), "{}", last_error());
        assert_eq!(
            sce_peer_set_identity(peer, ALICE, ALICE_SEED.as_ptr()),
            SCE_PEER_OK
        );
        assert_eq!(
            sce_peer_set_administrator(peer, ADMINISTRATOR.as_ptr()),
            SCE_PEER_OK
        );
        assert_eq!(
            sce_peer_open_list(peer, SCE_PEER_PROFILE_UNTRUSTED, genesis.as_mut_ptr()),
            SCE_PEER_OK,
            "{}",
            last_error()
        );
        assert_eq!(
            sce_peer_grant(peer, BOB, 2, bob_key.as_ptr()),
            SCE_PEER_OK,
            "{}",
            last_error()
        );
        (peer, genesis)
    }
}

/// Bob's core, invited with the hash of the genesis.
fn bob(genesis: [u8; 32]) -> *mut ScePeer {
    // SAFETY: the list is NUL-terminated, the genesis and the seed are 32 bytes.
    unsafe {
        let peer = sce_peer_invited(c"home".as_ptr(), genesis.as_ptr());
        assert!(!peer.is_null(), "{}", last_error());
        assert_eq!(
            sce_peer_set_identity(peer, BOB, BOB_SEED.as_ptr()),
            SCE_PEER_OK
        );
        peer
    }
}

/// A change of one write of `value` to field 3 of row `id`, stamped by `node` at `wall`,
/// NUL-terminated.
fn write(id: u32, node: u64, wall: i64, value: i64) -> std::ffi::CString {
    let text = serde_json::json!({"by": node.to_string(), "wallTime": wall.to_string(),
        "counter": 0, "ops": [{"kind": 0, "id": id, "field": 3, "value": value.to_string(),
        "text": "", "wallTime": "0", "counter": 0, "nodeId": "0"}]})
    .to_string();
    std::ffi::CString::new(text).unwrap()
}

/// `peer` sends a change, which its core takes.
///
/// # Safety
/// `peer` is a live handle.
unsafe fn sends(peer: *mut ScePeer, data: &std::ffi::CStr) {
    let mut kept = -1;
    // SAFETY: `peer` is live, the texts are NUL-terminated, `kept` is valid for a write.
    let status = unsafe {
        sce_peer_outbound(
            peer,
            c"record_list.change".as_ptr(),
            data.as_ptr(),
            &mut kept,
            std::ptr::null_mut(),
        )
    };
    assert_eq!(status, SCE_PEER_OK, "{}", last_error());
    assert_eq!(kept, SCE_PEER_KEPT_TAKEN);
}

/// How many `record_list.change` events an outcome tells the machine.
fn writes_in(outcome: &Value) -> usize {
    outcome["events"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|pair| pair[0] == "record_list.change")
        .count()
}

/// A run of a round of `peer` over `link`, with `within_ms`, that ends: what it came to.
///
/// # Safety
/// `peer` and `link` are live handles.
unsafe fn round_over(peer: *mut ScePeer, link: *mut ScePeerLink, within_ms: u64) -> Value {
    // SAFETY: the handles are live; no cancellation is given.
    unsafe {
        assert_eq!(sce_peer_start(peer, SCE_PEER_ROUND_SYNC), SCE_PEER_OK);
        assert_eq!(
            sce_peer_run(peer, link, within_ms, std::ptr::null()),
            SCE_PEER_OK,
            "{}",
            last_error()
        );
        outcome_of(peer)
    }
}

#[test]
fn a_link_is_made_and_freed_and_the_boundary_refuses_what_it_is_not_given() {
    // SAFETY: each call below passes pointers that are null, or valid for what the function's
    // contract names.
    unsafe {
        assert!(sce_peer_link_wss(c"http://127.0.0.1:1".as_ptr(), c"token".as_ptr()).is_null());
        assert!(last_error().contains("ws://"), "{}", last_error());
        assert!(sce_peer_link_wss(std::ptr::null(), c"token".as_ptr()).is_null());
        assert!(last_error().contains("null"), "{}", last_error());
        assert!(sce_peer_link_host(None, None, std::ptr::null_mut()).is_null());
        assert!(last_error().contains("exchange"), "{}", last_error());

        // Nothing is dialled until a run asks: a link to an address nothing listens at is made.
        let link = sce_peer_link_wss(c"ws://127.0.0.1:1/".as_ptr(), c"token".as_ptr());
        assert!(!link.is_null(), "{}", last_error());
        assert_eq!(notices_of(link), None, "nothing happened to it yet");
        assert_eq!(
            sce_peer_link_take_notices(link, std::ptr::null_mut()),
            SCE_PEER_ERR_NULL_ARGUMENT
        );
        assert_eq!(sce_peer_link_network_changed(link), SCE_PEER_OK);
        assert_eq!(
            sce_peer_link_network_changed(std::ptr::null()),
            SCE_PEER_ERR_NULL_ARGUMENT
        );
        assert_eq!(
            sce_peer_run(std::ptr::null_mut(), link, 1000, std::ptr::null()),
            SCE_PEER_ERR_NULL_ARGUMENT
        );

        // A core with no exchange running runs at once, and nothing is asked of the relay.
        let (peer, _) = alice();
        assert_eq!(
            sce_peer_run(peer, std::ptr::null_mut(), 1000, std::ptr::null()),
            SCE_PEER_ERR_NULL_ARGUMENT
        );
        assert_eq!(
            sce_peer_run(peer, link, 1000, std::ptr::null()),
            SCE_PEER_OK
        );
        assert_eq!(text_of(peer, sce_peer_take_outcome), None);
        assert_eq!(notices_of(link), None, "nothing was dialled");

        let cancel = sce_peer_cancel_new();
        assert!(!cancel.is_null());
        assert_eq!(sce_peer_cancel_raise(cancel), SCE_PEER_OK);
        assert_eq!(
            sce_peer_cancel_raise(std::ptr::null()),
            SCE_PEER_ERR_NULL_ARGUMENT
        );
        let text = b"{}";
        assert_eq!(
            sce_peer_reply(std::ptr::null_mut(), text.as_ptr(), text.len()),
            SCE_PEER_ERR_NULL_ARGUMENT
        );

        sce_peer_cancel_free(cancel);
        sce_peer_cancel_free(std::ptr::null_mut());
        sce_peer_link_free(link);
        sce_peer_link_free(std::ptr::null_mut());
        sce_peer_free(peer);
    }
}

#[test]
fn a_round_runs_over_the_websocket_binding_to_a_relay_in_this_process() {
    let authority = Authority::new([4; 32], "the-relay");
    let relay = Arc::new(Mutex::new(
        Relay::open(MemoryStore::new(), authority.clone(), Limits::default()).unwrap(),
    ));
    // The relay's clock: a second further at each reading, so that no device spends its allowance.
    let now = Arc::new(AtomicU64::new(0));
    let clock: Arc<dyn Fn() -> u64 + Send + Sync> = {
        let now = now.clone();
        Arc::new(move || now.fetch_add(1000, Ordering::SeqCst))
    };
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let listener = runtime
        .block_on(tokio::net::TcpListener::bind("127.0.0.1:0"))
        .unwrap();
    let base = std::ffi::CString::new(format!("ws://{}", listener.local_addr().unwrap())).unwrap();
    runtime.spawn(serve(listener, relay, clock, 64 * 1024));
    let token_of =
        |node: u64| std::ffi::CString::new(authority.issue("home", node, u64::MAX)).unwrap();
    let (alice_token, bob_token) = (token_of(ALICE), token_of(BOB));

    // SAFETY: each call below passes live handles and NUL-terminated texts.
    unsafe {
        let (alice, genesis) = alice();
        sends(alice, &write(1, ALICE, 1000, 10));
        let alice_link = sce_peer_link_wss(base.as_ptr(), alice_token.as_ptr());
        assert!(!alice_link.is_null(), "{}", last_error());
        let given = round_over(alice, alice_link, 30_000);
        assert_eq!(given["outcome"], "ended", "{given}");
        assert_eq!(given["pushed"], 3 + 1, "the log and the write: {given}");
        assert_eq!(
            notices_of(alice_link),
            Some(serde_json::json!([{"notice": "connected"}]))
        );

        let bob = bob(genesis);
        let bob_link = sce_peer_link_wss(base.as_ptr(), bob_token.as_ptr());
        let told = round_over(bob, bob_link, 30_000);
        assert_eq!(told["outcome"], "ended", "{told}");
        assert_eq!(writes_in(&told), 1, "{told}");

        // A link whose token the relay did not issue is refused, and the outcome says so in the
        // relay's word, as a token that expires does.
        let stranger = sce_peer_link_wss(base.as_ptr(), c"nonsense".as_ptr());
        let refused = round_over(bob, stranger, 30_000);
        assert_eq!(refused["failure"], "relay", "{refused}");
        assert_eq!(refused["code"], "unauthorized", "{refused}");

        sce_peer_link_free(stranger);
        sce_peer_link_free(bob_link);
        sce_peer_link_free(alice_link);
        sce_peer_free(bob);
        sce_peer_free(alice);
    }
}

/// A device handed no token logs in through the ABI (ADR 0080): over a link made with none, a round
/// is the relay's refusal of the token, which is when a host logs in; the login over the same link
/// comes to a token, and the link given it carries the device's rounds.
#[test]
fn a_device_logs_in_over_a_link_with_no_token_and_runs_its_rounds_with_the_token() {
    let authority = Authority::new([4; 32], "the-relay");
    let relay = Arc::new(Mutex::new(
        Relay::open(MemoryStore::new(), authority.clone(), Limits::default())
            .unwrap()
            .with_login(Doorway::with_key(LoginPolicy::default(), [5; 32])),
    ));
    let now = Arc::new(AtomicU64::new(0));
    let clock: Arc<dyn Fn() -> u64 + Send + Sync> = {
        let now = now.clone();
        Arc::new(move || now.fetch_add(1000, Ordering::SeqCst))
    };
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let listener = runtime
        .block_on(tokio::net::TcpListener::bind("127.0.0.1:0"))
        .unwrap();
    let base = std::ffi::CString::new(format!("ws://{}", listener.local_addr().unwrap())).unwrap();
    runtime.spawn(serve(listener, relay, clock, 64 * 1024));
    let alice_token = std::ffi::CString::new(authority.issue("home", ALICE, u64::MAX)).unwrap();

    // SAFETY: each call below passes live handles and NUL-terminated texts, or null for no token.
    unsafe {
        // The list is created with a token of the relay's authority.
        let (alice, genesis) = alice();
        let alice_link = sce_peer_link_wss(base.as_ptr(), alice_token.as_ptr());
        let given = round_over(alice, alice_link, 30_000);
        assert_eq!(given["outcome"], "ended", "{given}");

        let bob = bob(genesis);
        let bob_link = sce_peer_link_wss(base.as_ptr(), std::ptr::null());
        assert!(!bob_link.is_null(), "{}", last_error());
        let refused = round_over(bob, bob_link, 30_000);
        assert_eq!(refused["failure"], "relay", "{refused}");
        assert_eq!(refused["code"], "unauthorized", "{refused}");

        assert_eq!(sce_peer_login(bob, c"the-relay".as_ptr()), SCE_PEER_OK);
        assert_eq!(
            sce_peer_run(bob, bob_link, 30_000, std::ptr::null()),
            SCE_PEER_OK,
            "{}",
            last_error()
        );
        let logged_in = outcome_of(bob);
        assert_eq!(logged_in["outcome"], "ended", "{logged_in}");
        let token = logged_in["token"]["token"]
            .as_str()
            .expect("the outcome holds the token");
        assert_eq!(
            authority.verify(token, 0).map(|session| session.node),
            Ok(BOB),
            "a token of the relay's authority for bob's node"
        );
        let token = std::ffi::CString::new(token).unwrap();
        assert_eq!(
            sce_peer_link_set_token(bob_link, token.as_ptr()),
            SCE_PEER_OK
        );
        let told = round_over(bob, bob_link, 30_000);
        assert_eq!(told["outcome"], "ended", "{told}");
        assert_eq!(writes_in(&told), 0, "alice wrote nothing: {told}");
        assert!(told.get("token").is_none(), "a round earns no token");

        sce_peer_link_free(bob_link);
        sce_peer_link_free(alice_link);
        sce_peer_free(bob);
        sce_peer_free(alice);
    }
}

/// A `wss://` link runs TLS on the library's own cryptography and believes the public certificate
/// authorities: a relay presenting a certificate none of them issued is refused, and the run ends
/// with an outcome that says the link failed and why, where the library once panicked
/// (`SCE_PEER_ERR_PANIC`) for want of a cryptographic provider.
#[test]
fn a_wss_link_refuses_a_certificate_no_public_authority_issued_and_says_why() {
    let issued = rcgen::generate_simple_self_signed(vec!["127.0.0.1".to_string()]).unwrap();
    let tls = sce_rust_relay::tls::server_config(
        vec![issued.cert.der().to_vec()],
        issued.key_pair.serialize_der(),
    )
    .unwrap();
    let authority = Authority::new([4; 32], "the-relay");
    let relay = Arc::new(Mutex::new(
        Relay::open(MemoryStore::new(), authority.clone(), Limits::default()).unwrap(),
    ));
    let clock: Arc<dyn Fn() -> u64 + Send + Sync> = Arc::new(|| 0);
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let listener = runtime
        .block_on(tokio::net::TcpListener::bind("127.0.0.1:0"))
        .unwrap();
    let base = std::ffi::CString::new(format!("wss://{}", listener.local_addr().unwrap())).unwrap();
    runtime.spawn(serve_tls(listener, tls, relay, clock, 64 * 1024));
    let token = std::ffi::CString::new(authority.issue("home", ALICE, u64::MAX)).unwrap();

    // SAFETY: each call below passes live handles and NUL-terminated texts.
    unsafe {
        let (alice, _) = alice();
        let link = sce_peer_link_wss(base.as_ptr(), token.as_ptr());
        assert!(!link.is_null(), "{}", last_error());
        let failed = round_over(alice, link, 30_000);
        assert_eq!(failed["outcome"], "failed", "{failed}");
        assert_eq!(failed["failure"], "link", "{failed}");
        let reason = failed["reason"].as_str().unwrap_or_default();
        assert!(
            reason.contains("invalid peer certificate: UnknownIssuer"),
            "{failed}"
        );
        assert_eq!(notices_of(link), None, "it never connected");

        sce_peer_link_free(link);
        sce_peer_free(alice);
    }
}

/// `der` as a PEM certificate, as a deployment hands its authority out: base64 in the standard
/// alphabet, padded, in lines of 64, between the markers.
fn pem_of(der: &[u8]) -> Vec<u8> {
    // The trust layer writes bytes in the URL alphabet with no padding; PEM is the standard one.
    let mut body: String = sce_rust_trust::encoding::encode(der)
        .chars()
        .map(|c| match c {
            '-' => '+',
            '_' => '/',
            other => other,
        })
        .collect();
    let pad = (4 - body.len() % 4) % 4;
    for _ in 0..pad {
        body.push('=');
    }
    let mut pem = String::from("-----BEGIN CERTIFICATE-----\n");
    for line in body.as_bytes().chunks(64) {
        pem.push_str(std::str::from_utf8(line).unwrap());
        pem.push('\n');
    }
    pem.push_str("-----END CERTIFICATE-----\n");
    pem.into_bytes()
}

/// A `wss://` link made through the ABI believes the authority a deployment names, from the bytes
/// of a file an application ships: given in PEM and believed alone, the relay's certificate is
/// believed and alice's round carries the log and a write; given in DER with the public authorities,
/// bob's round reads the write. A link that believes the public authorities alone refuses the same
/// certificate (`a_wss_link_refuses_a_certificate_no_public_authority_issued_and_says_why`). Bytes
/// that hold no certificate or are not one, a null pointer, no choice of authorities and an address
/// that is not a WebSocket's are refused when the link is made, and say why.
#[test]
fn a_wss_link_believes_the_authority_it_is_given_through_the_abi() {
    let issued = rcgen::generate_simple_self_signed(vec!["127.0.0.1".to_string()]).unwrap();
    let certificate = issued.cert.der().to_vec();
    let tls = sce_rust_relay::tls::server_config(
        vec![certificate.clone()],
        issued.key_pair.serialize_der(),
    )
    .unwrap();
    let authority = Authority::new([4; 32], "the-relay");
    let relay = Arc::new(Mutex::new(
        Relay::open(MemoryStore::new(), authority.clone(), Limits::default()).unwrap(),
    ));
    // The relay's clock: a second further at each reading, so that no device spends its allowance.
    let now = Arc::new(AtomicU64::new(0));
    let clock: Arc<dyn Fn() -> u64 + Send + Sync> = {
        let now = now.clone();
        Arc::new(move || now.fetch_add(1000, Ordering::SeqCst))
    };
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let listener = runtime
        .block_on(tokio::net::TcpListener::bind("127.0.0.1:0"))
        .unwrap();
    let base = std::ffi::CString::new(format!("wss://{}", listener.local_addr().unwrap())).unwrap();
    runtime.spawn(serve_tls(listener, tls, relay, clock, 64 * 1024));
    let token_of =
        |node: u64| std::ffi::CString::new(authority.issue("home", node, u64::MAX)).unwrap();
    let (alice_token, bob_token) = (token_of(ALICE), token_of(BOB));
    let pem = pem_of(&certificate);

    // SAFETY: each call below passes live handles, NUL-terminated texts, and pointers that are null
    // or valid for the length given.
    unsafe {
        let (alice, genesis) = alice();
        sends(alice, &write(1, ALICE, 1000, 10));
        let alice_link = sce_peer_link_wss_believing(
            base.as_ptr(),
            alice_token.as_ptr(),
            pem.as_ptr(),
            pem.len(),
            SCE_PEER_ROOTS_ONLY,
        );
        assert!(!alice_link.is_null(), "{}", last_error());
        let given = round_over(alice, alice_link, 30_000);
        assert_eq!(given["outcome"], "ended", "{given}");
        assert_eq!(given["pushed"], 3 + 1, "the log and the write: {given}");
        assert_eq!(
            notices_of(alice_link),
            Some(serde_json::json!([{"notice": "connected"}]))
        );

        let bob = bob(genesis);
        let bob_link = sce_peer_link_wss_believing(
            base.as_ptr(),
            bob_token.as_ptr(),
            certificate.as_ptr(),
            certificate.len(),
            SCE_PEER_ROOTS_AND_PUBLIC,
        );
        assert!(!bob_link.is_null(), "{}", last_error());
        let told = round_over(bob, bob_link, 30_000);
        assert_eq!(told["outcome"], "ended", "{told}");
        assert_eq!(writes_in(&told), 1, "{told}");

        let none: [u8; 0] = [];
        let not_one = b"not a certificate";
        let wss = base.as_c_str();
        // The address, the authorities' bytes and their length, the choice, and why it is refused.
        type Refusal<'a> = (&'a std::ffi::CStr, *const u8, usize, i32, &'a str);
        let refusals: [Refusal; 5] = [
            (wss, none.as_ptr(), 0, SCE_PEER_ROOTS_ONLY, "no certificate"),
            (wss, std::ptr::null(), 4, SCE_PEER_ROOTS_ONLY, "null"),
            (wss, pem.as_ptr(), pem.len(), 7, "choice of authorities"),
            (
                wss,
                not_one.as_ptr(),
                not_one.len(),
                SCE_PEER_ROOTS_ONLY,
                "not usable",
            ),
            (
                c"http://127.0.0.1:1",
                pem.as_ptr(),
                pem.len(),
                SCE_PEER_ROOTS_ONLY,
                "ws://",
            ),
        ];
        for (address, authorities, len, roots, why) in refusals {
            let refused = sce_peer_link_wss_believing(
                address.as_ptr(),
                bob_token.as_ptr(),
                authorities,
                len,
                roots,
            );
            assert!(refused.is_null(), "{why}");
            assert!(last_error().contains(why), "{why}: {}", last_error());
        }

        sce_peer_link_free(bob_link);
        sce_peer_link_free(alice_link);
        sce_peer_free(bob);
        sce_peer_free(alice);
    }
}

/// What the tests' transport is to do with the next request, and what it was told.
#[derive(Default)]
struct Plan {
    /// Fail the next request as a connection that closed.
    fail_next: bool,
    /// Refuse the next request as the relay's endpoint refuses a token.
    refuse_next: bool,
    /// The time each request was given, in milliseconds.
    timeouts: Vec<u64>,
    /// Each request carried.
    carried: Vec<String>,
}

/// The context of a transport the host carries: a relay in this process, the device's token, and
/// the plan the test sets.
struct Carrier {
    relay: Arc<Mutex<Relay<MemoryStore>>>,
    token: String,
    plan: Arc<Mutex<Plan>>,
    released: Arc<AtomicUsize>,
}

/// Give `text` as the reply.
///
/// # Safety
/// `reply` is the one the library gave the call.
unsafe fn reply_with(reply: *mut ScePeerReply, text: &str) {
    // SAFETY: `reply` is live for the call, and `text` is valid for its length.
    unsafe { sce_peer_reply(reply, text.as_ptr(), text.len()) };
}

/// The host's transport: what it carries goes to the relay in this process.
///
/// # Safety
/// As `ScePeerExchangeFn` says; `context` is a `Carrier`.
unsafe extern "C" fn carry(
    context: *mut c_void,
    request: *const u8,
    len: usize,
    timeout_ms: u64,
    reply: *mut ScePeerReply,
) -> i32 {
    // SAFETY: the context the link was made with, used by one thread at a time.
    let carrier = unsafe { &*context.cast::<Carrier>() };
    // SAFETY: the request is `len` readable bytes for the call.
    let request = unsafe { std::slice::from_raw_parts(request, len) };
    let Ok(request) = std::str::from_utf8(request) else {
        // SAFETY: `reply` is the call's.
        unsafe { reply_with(reply, "the request is not UTF-8") };
        return SCE_PEER_TRANSPORT_LINK;
    };
    let mut plan = carrier.plan.lock().unwrap_or_else(|p| p.into_inner());
    plan.timeouts.push(timeout_ms);
    plan.carried.push(request.to_string());
    if std::mem::take(&mut plan.fail_next) {
        // SAFETY: `reply` is the call's.
        unsafe { reply_with(reply, "the connection closed") };
        return SCE_PEER_TRANSPORT_LINK;
    }
    if std::mem::take(&mut plan.refuse_next) {
        // SAFETY: `reply` is the call's.
        unsafe { reply_with(reply, "an expired token") };
        return SCE_PEER_TRANSPORT_UNAUTHORIZED;
    }
    let response = carrier
        .relay
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .handle(&carrier.token, request, 0);
    // SAFETY: `reply` is the call's.
    unsafe { reply_with(reply, &response) };
    SCE_PEER_TRANSPORT_ANSWERED
}

/// The host's context is given back once, when its link is freed.
///
/// # Safety
/// `context` is a `Carrier` made by `Box::into_raw`, released once.
unsafe extern "C" fn release(context: *mut c_void) {
    // SAFETY: made by `Box::into_raw` in `host_link`, and released once.
    let carrier = unsafe { Box::from_raw(context.cast::<Carrier>()) };
    carrier.released.fetch_add(1, Ordering::SeqCst);
}

/// A link whose transport the host carries to `relay` for the device of `node`.
fn host_link(
    relay: &Arc<Mutex<Relay<MemoryStore>>>,
    authority: &Authority,
    node: u64,
) -> (*mut ScePeerLink, Arc<Mutex<Plan>>, Arc<AtomicUsize>) {
    let plan = Arc::new(Mutex::new(Plan::default()));
    let released = Arc::new(AtomicUsize::new(0));
    let carrier = Box::new(Carrier {
        relay: relay.clone(),
        token: authority.issue("home", node, u64::MAX),
        plan: plan.clone(),
        released: released.clone(),
    });
    // SAFETY: `carry` and `release` are as the contract says, and the carrier is theirs until
    // `release` takes it back.
    let link = unsafe {
        sce_peer_link_host(
            Some(carry),
            Some(release),
            Box::into_raw(carrier).cast::<c_void>(),
        )
    };
    assert!(!link.is_null(), "{}", last_error());
    (link, plan, released)
}

/// A round of `peer`, the device of `node`, with every text carried to `relay` by hand: what it came
/// to and the requests it asked.
///
/// # Safety
/// `peer` is a live handle.
unsafe fn by_hand(
    peer: *mut ScePeer,
    relay: &Arc<Mutex<Relay<MemoryStore>>>,
    authority: &Authority,
    node: u64,
) -> (Value, Vec<String>) {
    let token = authority.issue("home", node, u64::MAX);
    let mut asked = Vec::new();
    // SAFETY: `peer` is live, and every text given to it is NUL-terminated.
    unsafe {
        assert_eq!(sce_peer_start(peer, SCE_PEER_ROUND_SYNC), SCE_PEER_OK);
        while let Some(request) = text_of(peer, sce_peer_poll_request) {
            let response = relay.lock().unwrap().handle(&token, &request, 0);
            asked.push(request);
            let response = std::ffi::CString::new(response).unwrap();
            assert_eq!(sce_peer_respond(peer, response.as_ptr()), SCE_PEER_OK);
        }
        (outcome_of(peer), asked)
    }
}

#[test]
fn a_round_runs_over_a_transport_the_host_carries_as_it_runs_by_hand() {
    let authority = Authority::new([4; 32], "the-relay");
    let open = || {
        Arc::new(Mutex::new(
            Relay::open(MemoryStore::new(), authority.clone(), Limits::default()).unwrap(),
        ))
    };
    let (by_hand_relay, carried_relay) = (open(), open());

    // SAFETY: each call below passes live handles, and pointers valid for what the function's
    // contract names.
    unsafe {
        let (hand_alice, genesis) = alice();
        let (carried_alice, _) = alice();
        let (hand_bob, carried_bob) = (bob(genesis), bob(genesis));
        let (alice_link, alice_plan, alice_released) = host_link(&carried_relay, &authority, ALICE);
        let (bob_link, bob_plan, bob_released) = host_link(&carried_relay, &authority, BOB);

        for alice in [hand_alice, carried_alice] {
            sends(alice, &write(1, ALICE, 1000, 10));
        }
        let (expected, asked) = by_hand(hand_alice, &by_hand_relay, &authority, ALICE);
        let outcome = round_over(carried_alice, alice_link, 30_000);
        assert_eq!(outcome, expected);
        assert_eq!(
            std::mem::take(&mut alice_plan.lock().unwrap().carried),
            asked
        );
        assert!(
            alice_plan
                .lock()
                .unwrap()
                .timeouts
                .iter()
                .all(|&ms| ms <= 30_000),
            "the time a run has left"
        );

        let (expected, asked) = by_hand(hand_bob, &by_hand_relay, &authority, BOB);
        let outcome = round_over(carried_bob, bob_link, u64::MAX);
        assert_eq!(outcome, expected);
        assert_eq!(writes_in(&outcome), 1, "{outcome}");
        assert_eq!(std::mem::take(&mut bob_plan.lock().unwrap().carried), asked);
        assert!(
            bob_plan
                .lock()
                .unwrap()
                .timeouts
                .iter()
                .all(|&ms| ms == u64::MAX),
            "no deadline"
        );

        // The host's transport fails: the round ends on it, with the host's reason, and the next
        // round goes on.
        bob_plan.lock().unwrap().fail_next = true;
        let failed = round_over(carried_bob, bob_link, 30_000);
        assert_eq!(failed["outcome"], "failed", "{failed}");
        assert_eq!(failed["failure"], "link", "{failed}");
        assert_eq!(failed["reason"], "the connection closed", "{failed}");
        let again = round_over(carried_bob, bob_link, 30_000);
        assert_eq!(again["outcome"], "ended", "{again}");

        // The host's transport says the token was refused: the relay's word for it.
        bob_plan.lock().unwrap().refuse_next = true;
        let refused = round_over(carried_bob, bob_link, 30_000);
        assert_eq!(refused["failure"], "relay", "{refused}");
        assert_eq!(refused["code"], "unauthorized", "{refused}");
        assert_eq!(refused["reason"], "an expired token", "{refused}");
        assert_eq!(
            notices_of(bob_link),
            None,
            "a host's transport tells its host nothing"
        );

        assert_eq!(bob_released.load(Ordering::SeqCst), 0);
        sce_peer_link_free(bob_link);
        assert_eq!(
            bob_released.load(Ordering::SeqCst),
            1,
            "released once, when freed"
        );
        sce_peer_link_free(alice_link);
        assert_eq!(alice_released.load(Ordering::SeqCst), 1);
        for peer in [hand_alice, carried_alice, hand_bob, carried_bob] {
            sce_peer_free(peer);
        }
    }
}

/// A listener that takes connections into its backlog and never answers one: a relay that does not
/// answer, so that a run waits.
fn silent() -> (std::net::TcpListener, std::ffi::CString) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base = std::ffi::CString::new(format!("ws://{}", listener.local_addr().unwrap())).unwrap();
    (listener, base)
}

/// Call `act` with `address` from another thread after 200 milliseconds: a handle's address, since a
/// pointer does not cross threads.
fn later(address: usize, act: unsafe fn(usize) -> i32) -> std::thread::JoinHandle<i32> {
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(200));
        // SAFETY: the handle lives until the thread is joined, and the function may be called
        // from any thread.
        unsafe { act(address) }
    })
}

/// Raise the cancellation at `address`.
///
/// # Safety
/// `address` is a live `ScePeerCancel`.
unsafe fn raise_at(address: usize) -> i32 {
    // SAFETY: the caller's promise.
    unsafe { sce_peer_cancel_raise(address as *const ScePeerCancel) }
}

/// Tell the link at `address` the network changed.
///
/// # Safety
/// `address` is a live `ScePeerLink`.
unsafe fn network_changed_at(address: usize) -> i32 {
    // SAFETY: the caller's promise.
    unsafe { sce_peer_link_network_changed(address as *const ScePeerLink) }
}

#[test]
fn a_run_ends_when_cancelled_when_its_deadline_passes_and_when_the_network_changes() {
    let (_listener, base) = silent();
    // SAFETY: each call below passes live handles, and the threads that raise are joined before
    // the handles are freed.
    unsafe {
        let (peer, _) = alice();
        let link = sce_peer_link_wss(base.as_ptr(), c"token".as_ptr());
        assert!(!link.is_null(), "{}", last_error());

        // Cancelled from another thread, a minute before the deadline.
        let cancel = sce_peer_cancel_new();
        assert_eq!(sce_peer_start(peer, SCE_PEER_ROUND_SYNC), SCE_PEER_OK);
        let raiser = later(cancel as usize, raise_at);
        let began = Instant::now();
        let status = sce_peer_run(peer, link, 60_000, cancel);
        assert_eq!(raiser.join().unwrap(), SCE_PEER_OK);
        assert_eq!(status, SCE_PEER_ERR_CANCELLED, "{}", last_error());
        assert!(
            began.elapsed() < Duration::from_secs(20),
            "{:?}",
            began.elapsed()
        );
        let cancelled = outcome_of(peer);
        assert_eq!(cancelled["failure"], "link", "{cancelled}");
        assert_eq!(
            cancelled["reason"], "the exchange was cancelled",
            "{cancelled}"
        );
        sce_peer_cancel_free(cancel);

        // The deadline passes.
        assert_eq!(sce_peer_start(peer, SCE_PEER_ROUND_SYNC), SCE_PEER_OK);
        let began = Instant::now();
        let status = sce_peer_run(peer, link, 300, std::ptr::null());
        let waited = began.elapsed();
        assert_eq!(status, SCE_PEER_ERR_DEADLINE, "{}", last_error());
        assert!(
            waited >= Duration::from_millis(200) && waited < Duration::from_secs(20),
            "{waited:?}"
        );
        let expired = outcome_of(peer);
        assert_eq!(expired["failure"], "link", "{expired}");
        assert_eq!(
            expired["reason"], "the deadline passed before the relay answered",
            "{expired}"
        );

        // The network changes while the run waits: the exchange fails as the link's failure, and
        // the run ended, since it was neither cancelled nor out of time.
        assert_eq!(sce_peer_start(peer, SCE_PEER_ROUND_SYNC), SCE_PEER_OK);
        let teller = later(link as usize, network_changed_at);
        let began = Instant::now();
        let status = sce_peer_run(peer, link, 60_000, std::ptr::null());
        assert_eq!(teller.join().unwrap(), SCE_PEER_OK);
        assert_eq!(status, SCE_PEER_OK, "{}", last_error());
        assert!(
            began.elapsed() < Duration::from_secs(20),
            "{:?}",
            began.elapsed()
        );
        let moved = outcome_of(peer);
        assert_eq!(moved["failure"], "link", "{moved}");
        assert_eq!(moved["reason"], "the network changed", "{moved}");

        sce_peer_link_free(link);
        sce_peer_free(peer);
    }
}
