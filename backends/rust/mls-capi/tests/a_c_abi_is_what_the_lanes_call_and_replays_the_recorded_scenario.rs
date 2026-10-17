// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
//
// The lanes that are not Rust reach the group key agreement through the C ABI. What is asked here:
// that the header declares exactly what the library exports; that the boundary refuses a null pointer, a
// device with no group, and a message that is not one, with a status and a text, and never crosses with
// a panic; and that the recorded scenario of the group (backends/rust/mls/tests/fixtures) is replayed
// through the library by C, by C++, by Python (ctypes), by Go (cgo) and by Kotlin (JNA) and each
// derives the recorded keys for every epoch, until the device is removed and derives none. A lane whose
// tool is missing says so and passes, unless `SCE_REQUIRE_TOOLS` is set, which is a lane's claim that it
// ran. The Kotlin lane needs the JVM program built (`./gradlew :sce-kotlin-mls-replay:installDist`).

use std::ffi::{c_char, CStr};
use std::path::{Path, PathBuf};
use std::process::Command;

use sce_mls_capi::*;
use serde_json::Value;

const MANIFEST: &str = env!("CARGO_MANIFEST_DIR");

fn scenario_path() -> PathBuf {
    Path::new(MANIFEST).join("../mls/tests/fixtures/join_and_remove.json")
}

fn last_error() -> String {
    // SAFETY: the library returns a NUL-terminated text it owns.
    unsafe { CStr::from_ptr(sce_mls_last_error()) }
        .to_string_lossy()
        .into_owned()
}

#[test]
fn the_header_declares_exactly_what_the_library_exports() {
    let header = std::fs::read_to_string(Path::new(MANIFEST).join("include/sce_mls.h")).unwrap();
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
    let declared: std::collections::BTreeSet<String> = names_in(&header, "sce_mls_")
        .into_iter()
        .filter(|name| header.contains(&format!("{name}(")))
        .collect();
    let exported: std::collections::BTreeSet<String> = source
        .lines()
        .filter(|line| line.contains("extern \"C\" fn sce_mls_"))
        .flat_map(|line| names_in(line, "sce_mls_"))
        .collect();
    assert_eq!(declared, exported);
    assert!(exported.len() >= 20, "{exported:?}");

    // The statuses are the same numbers on both sides.
    for (name, value) in [
        ("SCE_MLS_OK", SCE_MLS_OK),
        ("SCE_MLS_ERR_LIBRARY", SCE_MLS_ERR_LIBRARY),
        ("SCE_MLS_ERR_NO_GROUP", SCE_MLS_ERR_NO_GROUP),
        ("SCE_MLS_ERR_IN_GROUP", SCE_MLS_ERR_IN_GROUP),
        ("SCE_MLS_ERR_PENDING_COMMIT", SCE_MLS_ERR_PENDING_COMMIT),
        (
            "SCE_MLS_ERR_NO_PENDING_COMMIT",
            SCE_MLS_ERR_NO_PENDING_COMMIT,
        ),
        ("SCE_MLS_ERR_COMMIT_NOT_NAMED", SCE_MLS_ERR_COMMIT_NOT_NAMED),
        ("SCE_MLS_ERR_NOT_IN_LOG", SCE_MLS_ERR_NOT_IN_LOG),
        ("SCE_MLS_ERR_STILL_A_MEMBER", SCE_MLS_ERR_STILL_A_MEMBER),
        ("SCE_MLS_ERR_ROSTER_DIFFERS", SCE_MLS_ERR_ROSTER_DIFFERS),
        (
            "SCE_MLS_ERR_WELCOME_NOT_THE_LOGS",
            SCE_MLS_ERR_WELCOME_NOT_THE_LOGS,
        ),
        ("SCE_MLS_ERR_MALFORMED", SCE_MLS_ERR_MALFORMED),
        ("SCE_MLS_ERR_REMOVED", SCE_MLS_ERR_REMOVED),
        ("SCE_MLS_ERR_NULL_ARGUMENT", SCE_MLS_ERR_NULL_ARGUMENT),
        ("SCE_MLS_ERR_BAD_TEXT", SCE_MLS_ERR_BAD_TEXT),
        ("SCE_MLS_ERR_PANIC", SCE_MLS_ERR_PANIC),
    ] {
        assert!(
            header.contains(&format!("#define {name} {value}\n")),
            "{name} is not {value} in the header"
        );
    }
    assert!(header.contains(&format!(
        "#define SCE_MLS_ABI_VERSION {}u",
        SCE_MLS_ABI_VERSION
    )));
}

#[test]
fn the_boundary_refuses_what_it_is_not_given_and_says_why() {
    let list = c"home";
    let seed = [7u8; 32];
    // SAFETY: each call below passes pointers that are null, or valid for the length given.
    unsafe {
        assert_eq!(sce_mls_abi_version(), SCE_MLS_ABI_VERSION);
        assert!(sce_mls_device_new(std::ptr::null::<c_char>(), 1, seed.as_ptr()).is_null());
        assert!(last_error().contains("null"));
        assert!(sce_mls_device_new(list.as_ptr(), 1, std::ptr::null()).is_null());
        let not_utf8 = c"\xff\xfe";
        assert!(sce_mls_device_new(not_utf8.as_ptr(), 1, seed.as_ptr()).is_null());
        assert!(last_error().contains("UTF-8"));

        let device = sce_mls_device_new(list.as_ptr(), 1, seed.as_ptr());
        assert!(!device.is_null());
        let log = sce_mls_log_new(0);
        let mut epoch = 0u64;
        let mut key = [0u8; 32];

        // A device with no group derives nothing and has no epoch.
        assert_eq!(sce_mls_epoch(device, &mut epoch), SCE_MLS_ERR_NO_GROUP);
        assert_eq!(
            sce_mls_epoch_key(device, key.as_mut_ptr()),
            SCE_MLS_ERR_NO_GROUP
        );
        assert_eq!(sce_mls_is_active(device), 0);
        assert!(last_error().contains("NoGroup"));

        // Null handles and short pointers are refused.
        assert_eq!(
            sce_mls_create_group(std::ptr::null_mut()),
            SCE_MLS_ERR_NULL_ARGUMENT
        );
        assert_eq!(
            sce_mls_log_add_member(log, 1, std::ptr::null()),
            SCE_MLS_ERR_NULL_ARGUMENT
        );
        assert_eq!(
            sce_mls_epoch_key(device, std::ptr::null_mut()),
            SCE_MLS_ERR_NO_GROUP
        );
        assert_eq!(sce_mls_is_active(std::ptr::null_mut()), 0);

        // Bytes that are no Welcome and no commit are refused with the library's words.
        assert_eq!(sce_mls_create_group(device), SCE_MLS_OK);
        let garbage = [1u8, 2, 3];
        assert_eq!(
            sce_mls_join(
                device,
                garbage.as_ptr(),
                garbage.len(),
                garbage.as_ptr(),
                garbage.len(),
                log,
                &mut epoch
            ),
            SCE_MLS_ERR_IN_GROUP
        );
        let mut removed = 0i32;
        assert_eq!(
            sce_mls_apply_commit(
                device,
                garbage.as_ptr(),
                garbage.len(),
                log,
                &mut removed,
                &mut epoch
            ),
            SCE_MLS_ERR_COMMIT_NOT_NAMED,
            "the log names no commit"
        );
        assert_eq!(
            sce_mls_stage_add(
                device,
                garbage.as_ptr(),
                garbage.len(),
                log,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut()
            ),
            SCE_MLS_ERR_LIBRARY
        );
        assert_eq!(
            sce_mls_merge_pending(device, log, &mut epoch),
            SCE_MLS_ERR_NO_PENDING_COMMIT
        );

        // A key package of the device is bytes the library made and gives back.
        let mut package = SceMlsBuf {
            ptr: std::ptr::null_mut(),
            len: 0,
        };
        assert_eq!(sce_mls_key_package(device, &mut package), SCE_MLS_OK);
        assert!(!package.ptr.is_null() && package.len > 0);
        sce_mls_buf_free(package);
        sce_mls_buf_free(SceMlsBuf {
            ptr: std::ptr::null_mut(),
            len: 0,
        });

        // The state is kept and read back, and what is not the state is refused.
        let mut kept = SceMlsBuf {
            ptr: std::ptr::null_mut(),
            len: 0,
        };
        assert_eq!(sce_mls_device_to_bytes(device, &mut kept), SCE_MLS_OK);
        let again = sce_mls_device_from_bytes(list.as_ptr(), 1, seed.as_ptr(), kept.ptr, kept.len);
        assert!(!again.is_null());
        assert_eq!(sce_mls_epoch(again, &mut epoch), SCE_MLS_OK);
        assert_eq!(epoch, 0);
        assert!(
            sce_mls_device_from_bytes(list.as_ptr(), 1, seed.as_ptr(), garbage.as_ptr(), 3)
                .is_null()
        );
        assert!(last_error().contains("Malformed"));
        sce_mls_buf_free(kept);

        sce_mls_device_free(again);
        sce_mls_device_free(device);
        sce_mls_device_free(std::ptr::null_mut());
        sce_mls_log_free(log);
        sce_mls_log_free(std::ptr::null_mut());
    }
}

/// The scenario as the tokens the C replay reads.
fn script_of(scenario: &Value) -> String {
    let log = |log: &Value| -> String {
        let mut text = format!(
            "{} {}",
            log["epoch"],
            log["members"].as_array().unwrap().len()
        );
        for member in log["members"].as_array().unwrap() {
            text += &format!(" {} {}", member[0], member[1].as_str().unwrap());
        }
        text += &format!(" {}", log["commits"].as_array().unwrap().len());
        for commit in log["commits"].as_array().unwrap() {
            text += &format!(" {} {}", commit[0], commit[1].as_str().unwrap());
        }
        text
    };
    let device = &scenario["device"];
    let mut script = format!(
        "{} {} {} {}\n",
        scenario["list"].as_str().unwrap(),
        device["node"],
        device["seed"].as_str().unwrap(),
        device["state"].as_str().unwrap()
    );
    for step in scenario["steps"].as_array().unwrap() {
        let commit = step["commit"].as_str().unwrap();
        match step["op"].as_str().unwrap() {
            "join" => {
                script += &format!(
                    "join {} {} {} {} {} {} {}\n",
                    step["welcome"].as_str().unwrap(),
                    commit,
                    step["epoch"],
                    step["epoch_key"].as_str().unwrap(),
                    step["section"].as_str().unwrap(),
                    step["section_key"].as_str().unwrap(),
                    log(&step["log"])
                )
            }
            _ if step["removed"].as_bool() == Some(true) => {
                script += &format!("removed {} {}\n", commit, log(&step["log"]));
            }
            _ => {
                script += &format!(
                    "apply {} {} {} {}\n",
                    commit,
                    step["epoch"],
                    step["epoch_key"].as_str().unwrap(),
                    log(&step["log"])
                )
            }
        }
    }
    script
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sce-mls-capi-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_nanos())
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Whether `tool` runs; a missing one is a skip unless the lane claims it ran.
fn have(tool: &str, version_flag: &str) -> bool {
    let present = Command::new(tool)
        .arg(version_flag)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false);
    if !present {
        assert!(
            std::env::var_os("SCE_REQUIRE_TOOLS").is_none(),
            "SCE_REQUIRE_TOOLS is set and `{tool}` is missing"
        );
        eprintln!("skipped: `{tool}` is not installed");
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
        "libsce_mls_capi.so",
        "libsce_mls_capi.dylib",
        "sce_mls_capi.dll",
    ] {
        let path = profile_dir.join(name);
        if path.exists() {
            return path;
        }
    }
    panic!("the shared library is not in {}", profile_dir.display());
}

fn replay_in(command: &mut Command, what: &str) {
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
        String::from_utf8_lossy(&output.stdout).contains("replayed 3 steps"),
        "{what}: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn the_scenario_replays_from_c() {
    if !have("cc", "--version") {
        return;
    }
    let library = library();
    let dir = scratch("c");
    let scenario: Value =
        serde_json::from_str(&std::fs::read_to_string(scenario_path()).unwrap()).unwrap();
    let script = dir.join("scenario.txt");
    std::fs::write(&script, script_of(&scenario)).unwrap();
    let program = dir.join("replay");
    let compiled = Command::new("cc")
        .arg("-Wall")
        .arg("-Werror")
        .arg("-o")
        .arg(&program)
        .arg(Path::new(MANIFEST).join("tests/c/replay.c"))
        .arg("-I")
        .arg(Path::new(MANIFEST).join("include"))
        .arg("-L")
        .arg(library.parent().unwrap())
        .arg("-lsce_mls_capi")
        .arg(format!(
            "-Wl,-rpath,{}",
            library.parent().unwrap().display()
        ))
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    replay_in(Command::new(&program).arg(&script), "the C replay");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_scenario_replays_from_cpp() {
    if !have("c++", "--version") {
        return;
    }
    let library = library();
    let dir = scratch("cpp");
    let scenario: Value =
        serde_json::from_str(&std::fs::read_to_string(scenario_path()).unwrap()).unwrap();
    let script = dir.join("scenario.txt");
    std::fs::write(&script, script_of(&scenario)).unwrap();
    let program = dir.join("replay");
    let compiled = Command::new("c++")
        .arg("-std=c++17")
        .arg("-Wall")
        .arg("-Wextra")
        .arg("-Werror")
        .arg("-o")
        .arg(&program)
        .arg(Path::new(MANIFEST).join("tests/cpp/replay.cpp"))
        .arg("-I")
        .arg(Path::new(MANIFEST).join("include"))
        .arg("-L")
        .arg(library.parent().unwrap())
        .arg("-lsce_mls_capi")
        .arg(format!(
            "-Wl,-rpath,{}",
            library.parent().unwrap().display()
        ))
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    replay_in(Command::new(&program).arg(&script), "the C++ replay");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_scenario_replays_from_kotlin() {
    if !have("java", "-version") {
        return;
    }
    let lib =
        Path::new(MANIFEST).join("../../kotlin/mls-replay/build/install/sce-kotlin-mls-replay/lib");
    let built = std::fs::read_dir(&lib).is_ok_and(|entries| entries.count() > 0);
    if !built {
        assert!(
            std::env::var_os("SCE_REQUIRE_TOOLS").is_none(),
            "SCE_REQUIRE_TOOLS is set and {} is not built (`./gradlew :sce-kotlin-mls-replay:installDist`)",
            lib.display()
        );
        eprintln!("skipped: {} is not built", lib.display());
        return;
    }
    let library = library();
    replay_in(
        Command::new("java")
            .arg("-cp")
            .arg(lib.join("*"))
            .arg("com.sce.mls.ReplayKt")
            .arg(&library)
            .arg(scenario_path()),
        "the Kotlin replay",
    );
}

#[test]
fn the_scenario_replays_from_python() {
    if !have("python3", "--version") {
        return;
    }
    let library = library();
    replay_in(
        Command::new("python3")
            .arg("-I")
            .arg(Path::new(MANIFEST).join("tests/python/replay.py"))
            .arg(&library)
            .arg(scenario_path()),
        "the Python replay",
    );
}

#[test]
fn the_scenario_replays_from_go() {
    if !have("go", "version") {
        return;
    }
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
                format!("-L{0} -lsce_mls_capi -Wl,-rpath,{0}", libdir.display()),
            )
            .arg("run")
            .arg("replay.go")
            .arg(scenario_path()),
        "the Go replay",
    );
    std::fs::remove_dir_all(&dir).unwrap();
}
