// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
//
// The application lanes that are not Rust reach the peer core through the C ABI. What is asked here:
// that the header declares exactly what the library exports, with the same numbers; and that the
// boundary refuses a null pointer, a text that is not UTF-8, an answer out of turn, a kind of round
// that is none and a saved text that is not one, with a status and a text, takes no outcome into
// nowhere, and never crosses with a panic; and that the recorded scenario of two devices and a relay
// (`tests/fixtures/two_devices_and_a_relay.json`, recorded and held to a fresh run by
// `the_recorded_scenario_is_what_a_fresh_run_records`) is replayed through the shared library, with
// no relay, by each lane, and every text each core gives is the recorded one. A lane whose tool is
// missing says so and passes, unless `SCE_REQUIRE_TOOLS` is set, which is a lane's claim that it ran.

#![deny(unsafe_op_in_unsafe_fn)]

use std::ffi::{c_char, CStr, CString};
use std::path::{Path, PathBuf};
use std::process::Command;

use sce_peer_capi::*;
use serde_json::Value;

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
            "{} {} {}\n",
            device["node"],
            device["seed"].as_str().unwrap(),
            device["genesis"].as_str().unwrap_or("-")
        );
    }
    for step in scenario["steps"].as_array().unwrap() {
        let device = &step["device"];
        let text = |name: &str| token(step[name].as_str().unwrap());
        let word = |name: &str| step[name].as_str().unwrap().to_string();
        let op = step["op"].as_str().unwrap();
        script += &match op {
            "make" => format!("make {device}"),
            "restore" => format!("restore {device} {} {}", text("text"), step["count"]),
            "outbound" => format!(
                "outbound {device} {} {} {} {}",
                text("event"),
                text("data"),
                word("kept"),
                text("reason")
            ),
            "start" => format!("start {device} {}", word("kind")),
            "request" | "respond" | "outcome" | "save" => format!("{op} {device} {}", text("text")),
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
