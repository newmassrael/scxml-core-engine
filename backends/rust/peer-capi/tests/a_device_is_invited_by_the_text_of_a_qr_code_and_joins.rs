// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
//
// An administrator's device invites a device that is not in the list's log by the text of a QR code,
// and the invited device joins under it, through the C ABI alone (ADR 0080 section 4). What is asked
// here: that on a list keyed by its group the invited device, made from the text, leaves its key
// package under the invitation over a connection with no token, the administrator's device reads it,
// shows the same fingerprint the invited device showed, admits the key the proof names and no other,
// and the device joins the group from the Welcome; that on a list no group keys, the device joins with
// its key alone and is granted that key and no other; that a key package a relay made up, under no
// proof or under a proof of another key, is not taken, and the administrator's device admits nothing
// from it; that the same invitation used by a second device is refused by the relay; and that an
// invitation the core cannot make or join with is refused with its own status. MLS draws randomness,
// so what it gives is not compared here; the properties are. The part with no group is in the recorded
// scenario too (`the_recorded_scenario_is_what_a_fresh_run_records`).

#![deny(unsafe_op_in_unsafe_fn)]

use std::ffi::{CStr, CString};

use sce_peer_capi::*;
use sce_rust_relay::{Authority, Doorway, Joined, Limits, LoginPolicy, MemoryStore, Record, Relay};
use sce_rust_trust::invitation::{self, Invitation, Proof};
use sce_rust_trust::{encoding, Identity};
use serde_json::{json, Value};

const ADMINISTRATOR: [u8; 32] = [9; 32];
const ALICE: [u8; 32] = [1; 32];
const BOB: [u8; 32] = [2; 32];
const RELAY: &str = "wss://relay.example";
const AUDIENCE: &str = "the-relay";
const DAY_MS: u64 = 86_400_000;

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

fn public_of(seed: [u8; 32]) -> [u8; 32] {
    Identity::from_seed(seed).public().to_bytes()
}

/// The relay of a test in this process, offering login and joins, and the authority whose tokens
/// it takes.
struct Network {
    relay: Relay<MemoryStore>,
    authority: Authority,
    /// The relay's clock: a second further at each request.
    now: u64,
}

impl Network {
    fn new() -> Self {
        let authority = Authority::new([3; 32], AUDIENCE);
        Self {
            relay: Relay::open(MemoryStore::new(), authority.clone(), Limits::default())
                .unwrap()
                .with_login(Doorway::with_key(LoginPolicy::default(), [5; 32])),
            authority,
            now: 0,
        }
    }

    /// `request` carried to the relay, with a token of the authority for `node`, or none.
    fn carry(&mut self, node: Option<u64>, request: &str) -> String {
        self.now += 1000;
        match node {
            Some(node) => {
                let token = self.authority.issue("home", node, u64::MAX);
                self.relay.handle(&token, request, self.now)
            }
            None => self.relay.anonymous(None, request, self.now),
        }
    }

    /// The exchange `peer` runs, every request carried as `node`'s transport carries it (with a
    /// token for `node`, or none), each answer changed by `change`, and what it came to.
    ///
    /// # Safety
    /// `peer` is a live handle.
    unsafe fn finish_with(
        &mut self,
        peer: *mut ScePeer,
        node: Option<u64>,
        change: impl Fn(&str, String) -> String,
    ) -> Value {
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
            let answer = self.carry(node, &request);
            let response = CString::new(change(&request, answer)).unwrap();
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

    /// # Safety
    /// `peer` is a live handle.
    unsafe fn finish(&mut self, peer: *mut ScePeer, node: Option<u64>) -> Value {
        // SAFETY: the caller's promise.
        unsafe { self.finish_with(peer, node, |_, answer| answer) }
    }

    /// A round of `peer`, the device of `node`, carried with a token for it.
    ///
    /// # Safety
    /// `peer` is a live handle.
    unsafe fn round(&mut self, peer: *mut ScePeer, node: u64) -> Value {
        // SAFETY: the caller's promise.
        unsafe {
            assert_eq!(sce_peer_start(peer, SCE_PEER_ROUND_SYNC), SCE_PEER_OK);
            self.finish(peer, Some(node))
        }
    }
}

/// The administrator's device of the list `home`, its person node 1, the list opened and given the
/// relay; with a device of the list's group when `group`.
///
/// # Safety
/// Nothing beyond the calls' contracts.
unsafe fn administrator(network: &mut Network, group: bool) -> *mut ScePeer {
    let mut genesis = [0u8; 32];
    // SAFETY: NUL-terminated texts, 32-byte seeds, a place valid for 32 bytes.
    unsafe {
        let admin = sce_peer_open(c"home".as_ptr());
        assert!(!admin.is_null());
        assert_eq!(sce_peer_set_identity(admin, 1, ALICE.as_ptr()), SCE_PEER_OK);
        assert_eq!(
            sce_peer_set_administrator(admin, ADMINISTRATOR.as_ptr()),
            SCE_PEER_OK
        );
        if group {
            assert_eq!(sce_peer_with_group(admin, 1, ALICE.as_ptr()), SCE_PEER_OK);
        }
        assert_eq!(
            sce_peer_open_list(admin, SCE_PEER_PROFILE_UNTRUSTED, genesis.as_mut_ptr()),
            SCE_PEER_OK,
            "{}",
            last_error()
        );
        let opened = network.round(admin, 1);
        assert_eq!(opened["refused"], json!([]), "{opened}");
        admin
    }
}

/// The invitation `admin` makes of node `node`, under the secret `secret`.
///
/// # Safety
/// `admin` is a live handle.
unsafe fn invite(admin: *mut ScePeer, node: u64, secret: [u8; 32]) -> String {
    let (relay, audience) = (
        CString::new(RELAY).unwrap(),
        CString::new(AUDIENCE).unwrap(),
    );
    let mut out = empty();
    // SAFETY: a live handle, NUL-terminated texts, 32 bytes of secret, a place valid for a write.
    unsafe {
        assert_eq!(
            sce_peer_invite_under(
                admin,
                relay.as_ptr(),
                audience.as_ptr(),
                node,
                0,
                DAY_MS,
                secret.as_ptr(),
                &mut out
            ),
            SCE_PEER_OK,
            "{}",
            last_error()
        );
        taken(out).expect("an invitation is a text")
    }
}

/// The core of the device the invitation `text` invites, its person node `node` with the key
/// `seed` makes, and a device of the group when `group`.
///
/// # Safety
/// Nothing beyond the calls' contracts.
unsafe fn invited(text: &str, node: u64, seed: [u8; 32], group: bool) -> *mut ScePeer {
    // SAFETY: the text for its length, a 32-byte seed.
    unsafe {
        let peer = sce_peer_invited_by(text.as_ptr(), text.len());
        assert!(!peer.is_null(), "{}", last_error());
        assert_eq!(
            sce_peer_set_identity(peer, node, seed.as_ptr()),
            SCE_PEER_OK
        );
        if group {
            assert_eq!(sce_peer_with_group(peer, node, seed.as_ptr()), SCE_PEER_OK);
        }
        peer
    }
}

/// The device of `peer` joins under the invitation `text`, and the fingerprint it shows.
///
/// # Safety
/// `peer` is a live handle.
unsafe fn join(peer: *mut ScePeer, text: &str) -> (i32, Option<String>) {
    let mut fingerprint = empty();
    // SAFETY: a live handle, the text for its length, a place valid for a write.
    unsafe {
        let status = sce_peer_join(peer, text.as_ptr(), text.len(), &mut fingerprint);
        (status, taken(fingerprint))
    }
}

/// What `admin` shows of the devices that joined under its invitation of `node`.
///
/// # Safety
/// `admin` is a live handle.
unsafe fn joined(admin: *mut ScePeer, node: u64) -> Value {
    let mut out = empty();
    // SAFETY: a live handle, a place valid for a write.
    unsafe {
        assert_eq!(sce_peer_joined(admin, node, &mut out), SCE_PEER_OK);
        taken(out).map_or(Value::Null, |text| serde_json::from_str(&text).unwrap())
    }
}

#[test]
fn an_invited_device_joins_a_list_keyed_by_its_group_and_both_show_one_fingerprint() {
    let mut network = Network::new();
    // SAFETY: each call passes live handles, and pointers valid for the lengths the contracts name.
    unsafe {
        let admin = administrator(&mut network, true);
        let text = invite(admin, 2, [4; 32]);

        // What the text says, for the host to dial and to make the device with; no secret in it.
        let mut out = empty();
        assert_eq!(
            sce_peer_invitation_read(text.as_ptr(), text.len(), &mut out),
            SCE_PEER_OK
        );
        let said: Value = serde_json::from_str(&taken(out).unwrap()).unwrap();
        assert_eq!(said["relay"], RELAY);
        assert_eq!(said["audience"], AUDIENCE);
        assert_eq!(said["list"], "home");
        assert_eq!(said["node"], "2");
        assert_eq!(said["expires_at_ms"], DAY_MS.to_string());
        assert!(!said.to_string().contains(&encoding::encode(&[4; 32])));

        // Bob's device is made from the text, joins over a connection with no token, and shows its
        // fingerprint.
        let bob = invited(&text, 2, BOB, true);
        let (status, shown) = join(bob, &text);
        assert_eq!(status, SCE_PEER_OK, "{}", last_error());
        let shown = shown.expect("a fingerprint");
        let left = network.finish(bob, None);
        assert_eq!(left["outcome"], "ended", "{left}");
        assert_eq!(left["events"], json!([]), "{left}");

        // The administrator's next round reads the join, and shows the same digits.
        assert_eq!(joined(admin, 2), Value::Null, "nothing read yet");
        let read = network.round(admin, 1);
        assert_eq!(read["refused"], json!([]), "{read}");
        let devices = joined(admin, 2);
        assert_eq!(
            devices,
            json!([{"key": Identity::from_seed(BOB).public().to_text(), "fingerprint": shown}]),
            "the fingerprint the same on both sides"
        );

        // Another key than the proof names is not admitted; the proved one is.
        assert_eq!(
            sce_peer_add_member(admin, 2, 2, public_of([7; 32]).as_ptr()),
            SCE_PEER_ERR_NOT_JOINED,
            "{}",
            last_error()
        );
        assert_eq!(
            sce_peer_add_member(admin, 2, 2, public_of(BOB).as_ptr()),
            SCE_PEER_OK,
            "{}",
            last_error()
        );
        assert_eq!(
            joined(admin, 2),
            Value::Null,
            "admitted: the invitation is done"
        );
        let admitted = network.round(admin, 1);
        assert_eq!(admitted["refused"], json!([]), "{admitted}");

        // Bob joins the group from the Welcome.
        let welcomed = network.round(bob, 2);
        assert_eq!(
            welcomed["steps"],
            json!([{"step": "joined", "epoch": "1"}]),
            "{welcomed}"
        );

        sce_peer_free(bob);
        sce_peer_free(admin);
    }
}

#[test]
fn a_device_with_no_group_joins_with_its_key_alone_and_is_granted_that_key() {
    let mut network = Network::new();
    // SAFETY: each call passes live handles, and pointers valid for the lengths the contracts name.
    unsafe {
        let admin = administrator(&mut network, false);
        let text = invite(admin, 2, [4; 32]);
        let bob = invited(&text, 2, BOB, false);
        let (status, shown) = join(bob, &text);
        assert_eq!(status, SCE_PEER_OK, "{}", last_error());
        let left = network.finish(bob, None);
        assert_eq!(left["outcome"], "ended", "{left}");

        // A second device with the same text (a person who photographed the code) is refused: the
        // invitation was used.
        let other = invited(&text, 2, [6; 32], false);
        let (status, _) = join(other, &text);
        assert_eq!(status, SCE_PEER_OK);
        let raced = network.finish(other, None);
        assert_eq!(raced["outcome"], "failed", "{raced}");
        assert_eq!(raced["code"], "forbidden", "{raced}");

        network.round(admin, 1);
        let devices = joined(admin, 2);
        assert_eq!(devices[0]["fingerprint"], shown.unwrap());
        assert_eq!(devices.as_array().map(Vec::len), Some(1), "{devices}");
        // A grant of another key is refused; the proved key is granted.
        assert_eq!(
            sce_peer_grant(admin, 2, 2, public_of([6; 32]).as_ptr()),
            SCE_PEER_ERR_NOT_JOINED
        );
        assert_eq!(
            sce_peer_grant(admin, 2, 2, public_of(BOB).as_ptr()),
            SCE_PEER_OK,
            "{}",
            last_error()
        );
        let given = network.round(admin, 1);
        assert_eq!(given["pushed"], 1, "the grant: {given}");

        sce_peer_free(other);
        sce_peer_free(bob);
        sce_peer_free(admin);
    }
}

#[test]
fn a_key_package_a_relay_made_up_is_not_taken_and_admits_nothing() {
    let mut network = Network::new();
    // SAFETY: each call passes live handles, and pointers valid for the lengths the contracts name.
    unsafe {
        let admin = administrator(&mut network, true);
        let text = invite(admin, 2, [4; 32]);
        let ticket = invitation::Ticket::from_text(&text).unwrap();
        let forged_key = Identity::from_seed([7; 32]).public();
        // What a relay that knows the invitation and its public key alone can make: a key package
        // of its own under the invitation, with a proof under a key of its own; and one under no
        // invitation at all.
        let under_another_key = Record::KeyPackage {
            node: 2,
            data: encoding::encode(b"the relay's own key package"),
            join: Some(Box::new(Joined {
                invitation: ticket.invitation.clone(),
                proof: Proof::make(
                    &[8; 32],
                    &ticket.invitation,
                    &[1; 32],
                    60_000,
                    forged_key,
                    b"the relay's own key package",
                ),
            })),
        };
        let mut moved: Invitation = ticket.invitation.clone();
        moved.key = invitation::key_of(&[8; 32]).public();
        let under_its_own_invitation = Record::KeyPackage {
            node: 2,
            data: encoding::encode(b"the relay's own key package"),
            join: Some(Box::new(Joined {
                proof: Proof::make(
                    &[8; 32],
                    &moved,
                    &[1; 32],
                    60_000,
                    forged_key,
                    b"the relay's own key package",
                ),
                invitation: moved,
            })),
        };
        let bare = Record::KeyPackage {
            node: 2,
            data: encoding::encode(b"the relay's own key package"),
            join: None,
        };
        let forged: Vec<Value> = [under_another_key, under_its_own_invitation, bare]
            .iter()
            .map(Record::to_value)
            .collect();

        assert_eq!(sce_peer_start(admin, SCE_PEER_ROUND_SYNC), SCE_PEER_OK);
        let read = network.finish_with(admin, Some(1), |request, answer| {
            if !request.contains("\"feed\"") {
                return answer;
            }
            let mut feed: Value = serde_json::from_str(&answer).unwrap();
            for record in &forged {
                feed["records"].as_array_mut().unwrap().push(record.clone());
            }
            feed.to_string()
        });
        assert_eq!(read["outcome"], "ended", "{read}");
        assert_eq!(
            read["refused"].as_array().map(Vec::len),
            Some(2),
            "the two under an invitation, refused: {read}"
        );
        assert_eq!(joined(admin, 2), Value::Null, "nothing taken");
        assert_eq!(
            sce_peer_add_member(admin, 2, 2, forged_key.to_bytes().as_ptr()),
            SCE_PEER_ERR_NOT_JOINED,
            "{}",
            last_error()
        );
        sce_peer_free(admin);
    }
}

#[test]
fn an_invitation_the_core_cannot_make_or_join_with_is_refused_with_its_own_status() {
    let mut network = Network::new();
    let (relay, audience) = (
        CString::new(RELAY).unwrap(),
        CString::new(AUDIENCE).unwrap(),
    );
    // SAFETY: each call passes live handles, and pointers valid for the lengths the contracts name.
    unsafe {
        let admin = administrator(&mut network, false);
        let mut out = empty();
        let invite_for = |node: u64, valid_ms: u64, out: &mut ScePeerBuf| {
            sce_peer_invite(
                admin,
                relay.as_ptr(),
                audience.as_ptr(),
                node,
                0,
                valid_ms,
                out,
            )
        };
        // Alice's node is granted: no invitation is for it.
        assert_eq!(invite_for(1, DAY_MS, &mut out), SCE_PEER_ERR_GRANTED);
        assert!(out.ptr.is_null());
        // An invitation lives at most a day, and at least a millisecond.
        assert_eq!(invite_for(2, DAY_MS + 1, &mut out), SCE_PEER_ERR_INVITATION);
        assert_eq!(invite_for(2, 0, &mut out), SCE_PEER_ERR_INVITATION);
        // A drawn secret: two invitations are two texts.
        assert_eq!(invite_for(2, DAY_MS, &mut out), SCE_PEER_OK);
        let first = taken(out).unwrap();
        let mut out = empty();
        assert_eq!(invite_for(2, DAY_MS, &mut out), SCE_PEER_OK);
        assert_ne!(taken(out).unwrap(), first);

        // A device that holds no administrator's key invites no one.
        let plain = sce_peer_open(c"home".as_ptr());
        let mut out = empty();
        assert_eq!(
            sce_peer_invite(
                plain,
                relay.as_ptr(),
                audience.as_ptr(),
                2,
                0,
                DAY_MS,
                &mut out
            ),
            SCE_PEER_ERR_NO_ADMINISTRATOR
        );
        sce_peer_free(plain);

        // Bytes that are no invitation's text.
        let nonsense = "SCEI1.not an invitation";
        let mut out = empty();
        assert_eq!(
            sce_peer_invitation_read(nonsense.as_ptr(), nonsense.len(), &mut out),
            SCE_PEER_ERR_INVITATION
        );
        assert!(sce_peer_invited_by(nonsense.as_ptr(), nonsense.len()).is_null());

        // A device whose person is another node than the invitation's, or whose core is another
        // list's, does not join with it.
        let elsewhere = invited(&first, 3, [3; 32], false);
        assert_eq!(join(elsewhere, &first).0, SCE_PEER_ERR_INVITATION);
        sce_peer_free(elsewhere);
        let other_list = sce_peer_open(c"work".as_ptr());
        assert_eq!(
            sce_peer_set_identity(other_list, 2, BOB.as_ptr()),
            SCE_PEER_OK
        );
        assert_eq!(join(other_list, &first).0, SCE_PEER_ERR_INVITATION);
        sce_peer_free(other_list);
        // With no person, a core joins as no one.
        let nobody = sce_peer_invited_by(first.as_ptr(), first.len());
        assert_eq!(join(nobody, &first).0, SCE_PEER_ERR_NO_PERSON);
        sce_peer_free(nobody);

        sce_peer_free(admin);
    }
}
