// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The shared library (`cdylib`) of a package of this workspace, for a test that has another
// language load it through the package's C ABI. A test binary links Rust libraries and builds no
// shared one, so the test builds it, and where it lies is read from cargo's own artifact message,
// not worked out from where the test binary runs: cargo places a test binary in its build
// directory, whose layout is cargo's to change (Rust 1.100 moved test binaries from
// `<profile>/deps/` to `<profile>/build/<package>/<hash>/out/`), and a shared library elsewhere.
//
// A test file includes this one by path (`#[path = ...] mod shared_library;`), so the packages
// whose tests load a C ABI share one implementation without a crate of their own.

use std::env::consts::{DLL_PREFIX, DLL_SUFFIX};
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

/// Build the library of the package whose manifest is `manifest`, and answer the path of the
/// shared library of its library target `lib` (the `name` of its `[lib]`), as cargo names it.
///
/// The build is of the release profile when the test was built without debug assertions and of
/// the dev profile otherwise: cargo gives a test no profile's name, and `debug_assertions`, which
/// the release profile turns off and the dev and test profiles leave on, is what it compiles in.
///
/// # Panics
///
/// When cargo does not run or the build fails, with what the compiler said, or when cargo reports
/// no shared library of `lib`.
pub fn shared_library(manifest: &Path, lib: &str) -> PathBuf {
    let mut build = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string()));
    build
        .args([
            "build",
            "--lib",
            "--locked",
            "--message-format=json",
            "--manifest-path",
        ])
        .arg(manifest);
    if !cfg!(debug_assertions) {
        build.arg("--release");
    }
    let output = build.output().expect("cargo runs");
    let messages: Vec<Value> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();
    assert!(
        output.status.success(),
        "the shared library of `{lib}` did not build:\n{}\n{}",
        rendered(&messages),
        String::from_utf8_lossy(&output.stderr)
    );
    let file_name = format!("{DLL_PREFIX}{lib}{DLL_SUFFIX}");
    messages
        .iter()
        .filter(|message| message["reason"] == "compiler-artifact")
        .filter(|message| message["target"]["name"] == lib)
        .filter(|message| {
            message["target"]["kind"]
                .as_array()
                .is_some_and(|kinds| kinds.iter().any(|kind| kind == "cdylib"))
        })
        .filter_map(|message| message["filenames"].as_array())
        .flatten()
        .filter_map(Value::as_str)
        .map(PathBuf::from)
        .find(|path| {
            path.file_name()
                .is_some_and(|name| name == file_name.as_str())
        })
        .unwrap_or_else(|| panic!("cargo reported no shared library of `{lib}` ({file_name})"))
}

/// What the compiler said, as cargo's diagnostic messages render it.
fn rendered(messages: &[Value]) -> String {
    messages
        .iter()
        .filter(|message| message["reason"] == "compiler-message")
        .filter_map(|message| message["message"]["rendered"].as_str())
        .collect()
}
