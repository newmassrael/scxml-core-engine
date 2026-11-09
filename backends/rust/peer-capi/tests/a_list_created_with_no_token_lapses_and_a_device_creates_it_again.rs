// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
//
// A list created at a relay with no token, its lapse there, and its creation again by a device that
// holds it, through the C ABI alone (ADR 0080 section 3). What is asked here: that the name the ABI
// gives certifies the administrator's key the seed makes, under the host's salt or one it draws, and
// a salt of the wrong length is refused; that an administrator's device opens a list under that name
// and its login creates it at a relay that takes creations, with no token of the authority, and is
// issued a token; that its rounds and another member's run on the tokens their logins earned; that
// once the list lapsed at the relay the other member's login creates it again, with the same genesis
// and name, and its round gives the relay back what the device holds, the first device's write among
// it; and that the first device then logs in at the list again and reads the write made since. The
// relay is this process's, and its clock the test's; the recorded scenario keeps creation closed.

#![deny(unsafe_op_in_unsafe_fn)]

use std::ffi::{CStr, CString};

use sce_peer_capi::*;
use sce_rust_relay::{
    Authority, CreationPolicy, Doorway, Limits, LoginPolicy, MemoryStore, Relay, TOUCH_STEP_MS,
};
use sce_rust_trust::{creation, Identity};
use serde_json::Value;

const ADMINISTRATOR: [u8; 32] = [9; 32];
const ALICE: [u8; 32] = [1; 32];
const BOB: [u8; 32] = [2; 32];
const AUDIENCE: &str = "the-relay";
const DAY_MS: u64 = 86_400_000;
/// How long a list created with no token may go with no login: the decision's 90 days.
const IDLE_MS: u64 = 90 * DAY_MS;
/// The work the relay asks: enough to be found, few enough to be quick.
const BITS: u32 = 8;

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
/// `buf` is one the library returned and has not given back.
unsafe fn taken(buf: ScePeerBuf) -> Option<String> {
    if buf.ptr.is_null() {
        return None;
    }
    // SAFETY: the library's buffer is `len` readable bytes, given back once, here.
    let bytes = unsafe {
        let bytes = std::slice::from_raw_parts(buf.ptr, buf.len).to_vec();
        sce_peer_buf_free(buf);
        bytes
    };
    Some(String::from_utf8(bytes).expect("a text the library gives is UTF-8"))
}

/// The name the ABI gives for the administrator's seed `seed` under `salt` (none: one it draws),
/// or the status it refused with.
fn certified_name(seed: [u8; 32], salt: Option<&[u8]>) -> Result<String, i32> {
    let mut out = empty();
    let (pointer, length) = salt.map_or((std::ptr::null(), 0), |salt| (salt.as_ptr(), salt.len()));
    // SAFETY: a 32-byte seed, the salt for its length or null with 0, a place valid for a write.
    let status = unsafe { sce_peer_certified_name(seed.as_ptr(), pointer, length, &mut out) };
    if status != SCE_PEER_OK {
        return Err(status);
    }
    // SAFETY: what the call just gave.
    Ok(unsafe { taken(out) }.expect("a name is a text"))
}

/// The relay of this test, which takes creations with no token, and its clock: a second further
/// at each request.
struct Network {
    relay: Relay<MemoryStore>,
    now: u64,
}

impl Network {
    fn new() -> Self {
        let doorway =
            Doorway::with_key(LoginPolicy::default(), [5; 32]).with_creation(CreationPolicy {
                work_bits: BITS,
                ..CreationPolicy::default()
            });
        Self {
            relay: Relay::open(
                MemoryStore::new(),
                Authority::new([3; 32], AUDIENCE),
                Limits::default(),
            )
            .unwrap()
            .with_login(doorway),
            now: 1_000_000,
        }
    }

    /// `request` carried to the relay with `token`, or on a connection with none.
    fn carry(&mut self, token: Option<&str>, request: &str) -> String {
        self.now += 1000;
        match token {
            Some(token) => self.relay.handle(token, request, self.now),
            None => self.relay.anonymous(None, request, self.now),
        }
    }

    /// The exchange `peer` runs, every request carried with `token` or none, and what it came to.
    ///
    /// # Safety
    /// `peer` is a live handle.
    unsafe fn finish(&mut self, peer: *mut ScePeer, token: Option<&str>) -> Value {
        loop {
            let mut out = empty();
            // SAFETY: `peer` is live, `out` valid for a write.
            assert_eq!(
                unsafe { sce_peer_poll_request(peer, &mut out) },
                SCE_PEER_OK
            );
            // SAFETY: what the call just gave.
            let Some(request) = (unsafe { taken(out) }) else {
                break;
            };
            let response = CString::new(self.carry(token, &request)).unwrap();
            // SAFETY: `peer` is live, the response NUL-terminated.
            assert_eq!(
                unsafe { sce_peer_respond(peer, response.as_ptr()) },
                SCE_PEER_OK,
                "{}",
                last_error()
            );
        }
        let mut out = empty();
        // SAFETY: `peer` is live, `out` valid for a write.
        assert_eq!(
            unsafe { sce_peer_take_outcome(peer, &mut out) },
            SCE_PEER_OK
        );
        // SAFETY: what the call just gave.
        let outcome = unsafe { taken(out) }.expect("the exchange ended");
        serde_json::from_str(&outcome).expect("an outcome is JSON")
    }

    /// The login of `peer` on a connection with no token, and the token its outcome holds.
    ///
    /// # Safety
    /// `peer` is a live handle.
    unsafe fn log_in(&mut self, peer: *mut ScePeer) -> String {
        let audience = CString::new(AUDIENCE).unwrap();
        // SAFETY: a live handle, a NUL-terminated text.
        assert_eq!(
            unsafe { sce_peer_login(peer, audience.as_ptr()) },
            SCE_PEER_OK,
            "{}",
            last_error()
        );
        // SAFETY: the caller's promise.
        let outcome = unsafe { self.finish(peer, None) };
        assert_eq!(outcome["outcome"], "ended", "{outcome}");
        outcome["token"]["token"]
            .as_str()
            .unwrap_or_else(|| panic!("a login's outcome holds a token: {outcome}"))
            .to_string()
    }

    /// A round of `peer` with `token`, and what it came to.
    ///
    /// # Safety
    /// `peer` is a live handle.
    unsafe fn round(&mut self, peer: *mut ScePeer, token: &str) -> Value {
        // SAFETY: the caller's promise.
        unsafe {
            assert_eq!(sce_peer_start(peer, SCE_PEER_ROUND_SYNC), SCE_PEER_OK);
            let outcome = self.finish(peer, Some(token));
            assert_eq!(outcome["outcome"], "ended", "{outcome}");
            outcome
        }
    }
}

/// The change of one write the machine of `node` sends of row `id`.
fn write(id: u32, node: u64) -> CString {
    CString::new(format!(
        "{{\"by\":\"{node}\",\"wallTime\":\"{}\",\"counter\":0,\"ops\":[\
         {{\"kind\":0,\"id\":{id},\"field\":3,\"value\":\"1\",\"text\":\"\",\
         \"wallTime\":\"0\",\"counter\":0,\"nodeId\":\"0\"}}]}}",
        1000 * id
    ))
    .unwrap()
}

/// `peer` takes the machine's write of row `id` as its person's, `node`.
///
/// # Safety
/// `peer` is a live handle.
unsafe fn send(peer: *mut ScePeer, id: u32, node: u64) {
    let mut kept = -1;
    let mut reason = empty();
    let data = write(id, node);
    // SAFETY: a live handle, NUL-terminated texts, places valid for a write.
    unsafe {
        assert_eq!(
            sce_peer_outbound(
                peer,
                c"record_list.change".as_ptr(),
                data.as_ptr(),
                &mut kept,
                &mut reason
            ),
            SCE_PEER_OK,
            "{}",
            last_error()
        );
        let reason = taken(reason);
        assert_eq!(kept, SCE_PEER_KEPT_TAKEN, "{reason:?}");
    }
}

/// The core of the device of `node`, signing with `seed`, of the list `name`.
///
/// # Safety
/// Nothing beyond the calls' contracts.
unsafe fn device(name: &CString, node: u64, seed: [u8; 32]) -> *mut ScePeer {
    // SAFETY: a NUL-terminated name, a 32-byte seed.
    unsafe {
        let peer = sce_peer_open(name.as_ptr());
        assert!(!peer.is_null(), "{}", last_error());
        assert_eq!(
            sce_peer_set_identity(peer, node, seed.as_ptr()),
            SCE_PEER_OK
        );
        peer
    }
}

/// Whether `outcome` tells the machine a change that `node` made.
fn tells_a_write_of(outcome: &Value, node: u64) -> bool {
    let stamp = format!("\"by\":\"{node}\"");
    outcome["events"]
        .as_array()
        .expect("an outcome's events are a list")
        .iter()
        .any(|event| {
            event[0] == "record_list.change" && event[1].as_str().unwrap().contains(&stamp)
        })
}

#[test]
fn the_name_the_abi_gives_certifies_the_administrators_key() {
    let key = Identity::from_seed(ADMINISTRATOR).public();
    let chosen = certified_name(ADMINISTRATOR, Some(b"household")).unwrap();
    assert_eq!(
        Some(chosen.clone()),
        creation::list_name(b"household", &key),
        "the host's salt"
    );
    let drawn = certified_name(ADMINISTRATOR, None).unwrap();
    assert!(creation::certifies(&drawn, &key), "{drawn}");
    assert_ne!(drawn, chosen, "a salt the library draws");
    assert!(!creation::certifies(
        &chosen,
        &Identity::from_seed(ALICE).public()
    ));
    assert_eq!(
        certified_name(ADMINISTRATOR, Some(&[0; 33])),
        Err(SCE_PEER_ERR_BAD_ARGUMENT)
    );
    // SAFETY: a null seed, which the library refuses before it reads anything.
    let status = unsafe {
        let mut out = empty();
        sce_peer_certified_name(std::ptr::null(), std::ptr::null(), 0, &mut out)
    };
    assert_eq!(status, SCE_PEER_ERR_NULL_ARGUMENT);
}

#[test]
fn a_list_created_with_no_token_lapses_and_a_device_that_holds_it_creates_it_again() {
    let mut network = Network::new();
    let name = certified_name(ADMINISTRATOR, Some(b"household")).unwrap();
    let list = CString::new(name.clone()).unwrap();
    let bob_key = Identity::from_seed(BOB).public().to_bytes();
    let mut genesis = [0u8; 32];
    // SAFETY: every handle below is live until it is freed at the end, every text NUL-terminated,
    // every key and seed 32 bytes, every place valid for its write.
    unsafe {
        // Alice's device opens the list under the name and grants bob; its login creates the list
        // at the relay, which holds the log as far as her own grant.
        let alice = device(&list, 1, ALICE);
        assert_eq!(
            sce_peer_set_administrator(alice, ADMINISTRATOR.as_ptr()),
            SCE_PEER_OK
        );
        assert_eq!(
            sce_peer_open_list(alice, SCE_PEER_PROFILE_UNTRUSTED, genesis.as_mut_ptr()),
            SCE_PEER_OK,
            "{}",
            last_error()
        );
        assert_eq!(
            sce_peer_grant(alice, 2, 2, bob_key.as_ptr()),
            SCE_PEER_OK,
            "{}",
            last_error()
        );
        let alice_token = network.log_in(alice);
        assert!(network.relay.is_created_openly(&name));
        assert_eq!(network.relay.records(&name), 2);
        // Her round gives the relay bob's grant and her write.
        send(alice, 1, 1);
        let round = network.round(alice, &alice_token);
        assert_eq!(round["refused"], serde_json::json!([]), "{round}");
        assert_eq!(round["pushed"], 2, "{round}");
        assert_eq!(network.relay.records(&name), 4);

        // Bob's device logs in at the list the relay holds, and reads her write.
        let bob = device(&list, 2, BOB);
        let bob_token = network.log_in(bob);
        let round = network.round(bob, &bob_token);
        assert!(tells_a_write_of(&round, 1), "{round}");
        assert_eq!(network.relay.records(&name), 4, "a login is no creation");

        // No login touches the list for its days: it lapses, whole.
        network.now += IDLE_MS + TOUCH_STEP_MS;
        let lapsed = network.relay.lapse(IDLE_MS, network.now);
        assert_eq!((lapsed.removed, lapsed.kept), (1, 0));
        assert_eq!(network.relay.records(&name), 0);

        // Bob's login creates it again, with the same genesis and name, as far as his grant.
        let bob_token = network.log_in(bob);
        assert!(network.relay.is_created_openly(&name));
        assert_eq!(network.relay.records(&name), 3);
        // His round gives back what he holds and the relay lacks: alice's write, and his own.
        send(bob, 2, 2);
        let round = network.round(bob, &bob_token);
        assert_eq!(round["refused"], serde_json::json!([]), "{round}");
        assert_eq!(round["pushed"], 2, "{round}");
        assert_eq!(network.relay.records(&name), 5);

        // Alice's device logs in at the list again, and reads the write made since.
        let alice_token = network.log_in(alice);
        let round = network.round(alice, &alice_token);
        assert_eq!(round["refused"], serde_json::json!([]), "{round}");
        assert!(tells_a_write_of(&round, 2), "{round}");
        assert_eq!(
            network.relay.records(&name),
            5,
            "she held nothing it lacked"
        );

        sce_peer_free(alice);
        sce_peer_free(bob);
    }
}
