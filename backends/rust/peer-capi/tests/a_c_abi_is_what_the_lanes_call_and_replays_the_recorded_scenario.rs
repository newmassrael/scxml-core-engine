// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
//
// The application lanes that are not Rust reach the peer core through the C ABI. What is asked here:
// that the header declares exactly what the library exports, with the same numbers; and that the
// boundary refuses a null pointer, a text that is not UTF-8, an answer out of turn, a kind of round
// that is none and a saved text that is not one, with a status and a text, takes no outcome into
// nowhere, and never crosses with a panic.

#![deny(unsafe_op_in_unsafe_fn)]

use std::ffi::{c_char, CStr, CString};
use std::path::Path;

use sce_peer_capi::*;
use serde_json::Value;

const MANIFEST: &str = env!("CARGO_MANIFEST_DIR");

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
    assert!(exported.len() >= 21, "{exported:?}");

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
        ("SCE_PEER_ERR_NULL_ARGUMENT", SCE_PEER_ERR_NULL_ARGUMENT),
        ("SCE_PEER_ERR_BAD_TEXT", SCE_PEER_ERR_BAD_TEXT),
        ("SCE_PEER_ERR_PANIC", SCE_PEER_ERR_PANIC),
        ("SCE_PEER_ERR_BAD_ARGUMENT", SCE_PEER_ERR_BAD_ARGUMENT),
        ("SCE_PEER_KEPT_TAKEN", SCE_PEER_KEPT_TAKEN),
        ("SCE_PEER_KEPT_HELD", SCE_PEER_KEPT_HELD),
        ("SCE_PEER_KEPT_NOT_SIGNED", SCE_PEER_KEPT_NOT_SIGNED),
        ("SCE_PEER_KEPT_REFUSED", SCE_PEER_KEPT_REFUSED),
        ("SCE_PEER_ROUND_SYNC", SCE_PEER_ROUND_SYNC),
        ("SCE_PEER_ROUND_RESYNC", SCE_PEER_ROUND_RESYNC),
        ("SCE_PEER_ROUND_REREAD", SCE_PEER_ROUND_REREAD),
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

#[test]
fn the_boundary_refuses_what_it_is_not_given_and_says_why() {
    let list = c"home";
    let seed = [1u8; 32];
    let genesis = [0u8; 32];
    let write = c"{\"id\":1,\"field\":3,\"value\":\"1\",\"text\":\"\",\"wallTime\":\"1000\",\"counter\":0,\"nodeId\":\"2\"}";
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
                c"record_list.propose".as_ptr(),
                c"{\"before\":\"30000\"}".as_ptr(),
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
                c"record_list.propose".as_ptr(),
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
                c"record_list.write".as_ptr(),
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
                c"record_list.write".as_ptr(),
                write.as_ptr(),
                &mut kept,
                std::ptr::null_mut()
            ),
            SCE_PEER_OK,
            "the reason may be left"
        );
        assert_eq!(kept, SCE_PEER_KEPT_REFUSED);

        // The group's device: none is held until one is given, and bytes that are no state of one
        // are refused with the group's words; a new device leaves a key package, and what it keeps
        // is given back and taken again.
        let mut state = empty();
        assert_eq!(
            sce_peer_group_state(peer, &mut state),
            SCE_PEER_ERR_NO_GROUP_DEVICE
        );
        assert!(state.ptr.is_null());
        assert_eq!(
            sce_peer_publish_key_package(peer),
            SCE_PEER_ERR_NO_GROUP_DEVICE
        );
        let garbage = [1u8, 2, 3];
        assert_eq!(
            sce_peer_with_group(peer, 1, seed.as_ptr(), garbage.as_ptr(), garbage.len()),
            SCE_PEER_ERR_GROUP
        );
        assert!(last_error().contains("Malformed"), "{}", last_error());
        assert_eq!(
            sce_peer_with_group(peer, 1, seed.as_ptr(), std::ptr::null(), 0),
            SCE_PEER_OK
        );
        assert_eq!(sce_peer_publish_key_package(peer), SCE_PEER_OK);
        let kept_state = group_state_of(peer);
        assert!(!kept_state.is_empty());
        let again = sce_peer_open(list.as_ptr());
        assert_eq!(
            sce_peer_with_group(
                again,
                1,
                seed.as_ptr(),
                kept_state.as_ptr(),
                kept_state.len()
            ),
            SCE_PEER_OK
        );
        assert_eq!(sce_peer_for_a_hub(again), SCE_PEER_OK);
        assert_eq!(
            sce_peer_for_a_hub(std::ptr::null_mut()),
            SCE_PEER_ERR_NULL_ARGUMENT
        );

        // No checkpoint makes a settlement here, and that is an empty buffer, not a failure.
        let mut floor = empty();
        assert_eq!(
            sce_peer_floor_event(peer, 30_000, 7, &mut floor),
            SCE_PEER_OK
        );
        assert_eq!(taken(floor), None);
        assert_eq!(
            sce_peer_floor_event(peer, 30_000, 7, std::ptr::null_mut()),
            SCE_PEER_ERR_NULL_ARGUMENT
        );

        // A text that is not a saved one is refused; a save is one text, taken back by a core
        // just made for the list.
        let fresh = sce_peer_open(list.as_ptr());
        assert_eq!(
            sce_peer_restore(fresh, c"{}".as_ptr(), std::ptr::null_mut()),
            SCE_PEER_ERR_UNREADABLE
        );
        assert!(last_error().contains("Unreadable"), "{}", last_error());
        assert_eq!(
            sce_peer_restore(fresh, c"not JSON".as_ptr(), std::ptr::null_mut()),
            SCE_PEER_ERR_UNREADABLE
        );
        let saved = CString::new(text_of(peer, sce_peer_save).expect("a save is a text")).unwrap();
        let mut count = u64::MAX;
        assert_eq!(
            sce_peer_restore(fresh, saved.as_ptr(), &mut count),
            SCE_PEER_OK
        );
        assert_eq!(count, 0, "the core held no operation");

        sce_peer_buf_free(empty());
        sce_peer_free(fresh);
        sce_peer_free(again);
        sce_peer_free(nobody);
        sce_peer_free(peer);
        sce_peer_free(std::ptr::null_mut());
    }
}

/// What the group's device of `peer` keeps, as bytes.
///
/// # Safety
/// `peer` is a live handle holding a group's device.
unsafe fn group_state_of(peer: *mut ScePeer) -> Vec<u8> {
    let mut out = empty();
    // SAFETY: `peer` is live and `out` is valid for a write.
    assert_eq!(
        unsafe { sce_peer_group_state(peer, &mut out) },
        SCE_PEER_OK,
        "{}",
        last_error()
    );
    assert!(!out.ptr.is_null());
    // SAFETY: the library's buffer is `len` readable bytes.
    let bytes = unsafe { std::slice::from_raw_parts(out.ptr, out.len) }.to_vec();
    // SAFETY: returned by the library and not given back yet.
    unsafe { sce_peer_buf_free(out) };
    bytes
}
