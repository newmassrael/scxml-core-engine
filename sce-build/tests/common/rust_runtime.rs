// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The runtime crates a generated Rust file names, built from this tree for a
//! suite that compiles generated files with `rustc` itself.
//!
//! A unit names `sce_portable_bytes` wherever its type is a key of a map
//! (docs/adr/0108, D6 item 4) or holds a bounded text, and an algorithm names
//! `sce_forge_runtime` wherever it can fail or computes an integer operation the
//! interval analysis proved, so a crate of generated files is compiled against
//! both, at the versions the workspace tests.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;

/// The crates [`rustc_flags`] builds and finds, by the name `rustc` links each
/// under.
const CRATES: [&str; 2] = ["sce_forge_runtime", "sce_portable_bytes"];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// Build the crates a generated file names, once, under `scratch`, and answer
/// the flags that make `rustc` find them.
///
/// The crate that depends on them is a scratch one that is its own workspace and
/// carries a copy of the workspace's lock file, so cargo resolves the versions
/// the workspace resolved and, offline, from the cache the workspace filled
/// ([`super::run_cargo_offline_first`]). What it built is read from cargo's own
/// artifact messages, not guessed from a file name, and the directory holds
/// only this run's build, so there is one candidate for every crate a flag has
/// to find.
pub fn rustc_flags(scratch: &Path) -> Vec<String> {
    let root = repo_root();
    let crate_dir = scratch.join("runtime");
    std::fs::create_dir_all(crate_dir.join("src")).expect("mkdir");
    std::fs::write(crate_dir.join("src/lib.rs"), "").expect("write lib.rs");
    std::fs::write(
        crate_dir.join("Cargo.toml"),
        format!(
            "[package]\n\
             name = \"sce-rustc-runtime\"\n\
             version = \"0.0.0\"\n\
             edition = \"2021\"\n\
             publish = false\n\
             \n\
             [lib]\n\
             path = \"src/lib.rs\"\n\
             \n\
             # The two crates the generated files name, with the heap tier the\n\
             # conformance crate builds them with.\n\
             [dependencies]\n\
             sce-forge-runtime = {{ path = {forge:?}, default-features = false, features = [\"alloc\"] }}\n\
             sce-portable-bytes = {{ path = {bytes:?}, default-features = false, features = [\"alloc\"] }}\n\
             \n\
             [workspace]\n",
            forge = root.join("backends/rust/forge-runtime").to_string_lossy(),
            bytes = root.join("backends/rust/portable-bytes").to_string_lossy(),
        ),
    )
    .expect("write Cargo.toml");
    std::fs::copy(root.join("Cargo.lock"), crate_dir.join("Cargo.lock"))
        .expect("copy the workspace lock file");

    let target = scratch.join("target");
    let manifest = crate_dir.join("Cargo.toml");
    let built = super::run_cargo_offline_first(|| {
        let mut cmd = Command::new("cargo");
        cmd.arg("build")
            .arg("--message-format=json")
            .arg("--manifest-path")
            .arg(&manifest)
            .env("CARGO_TARGET_DIR", &target);
        cmd
    });
    assert!(
        built.output.status.success(),
        "the runtime crates a generated Rust file names do not build:\n{}",
        String::from_utf8_lossy(&built.output.stderr)
    );

    let stdout = String::from_utf8_lossy(&built.output.stdout);
    // Each library cargo built, by the crate name `rustc` links it under, as
    // cargo's artifact messages name its files.
    let rlibs: Vec<(String, PathBuf)> = stdout
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter(|message| message["reason"] == "compiler-artifact")
        .filter_map(|message| {
            let name = message["target"]["name"].as_str()?.replace('-', "_");
            let rlib = message["filenames"]
                .as_array()?
                .iter()
                .filter_map(|file| file.as_str())
                .find(|file| file.ends_with(".rlib"))?;
            Some((name, PathBuf::from(rlib)))
        })
        .collect();
    // The directory of every library built, `heapless` among them, which a crate
    // `--extern` names depends on: what `rustc` searches for a dependency of a
    // dependency, as cargo passes it. Read from where the libraries are, not
    // assumed, since a target or a profile cargo is run under moves them.
    let mut flags = Vec::new();
    let mut searched: Vec<&Path> = Vec::new();
    for (_, rlib) in &rlibs {
        let dir = rlib.parent().expect("a built library lies in a directory");
        if !searched.contains(&dir) {
            searched.push(dir);
            flags.push("-L".to_string());
            flags.push(format!("dependency={}", dir.display()));
        }
    }
    for crate_name in CRATES {
        let (_, rlib) = rlibs
            .iter()
            .find(|(name, _)| name == crate_name)
            .unwrap_or_else(|| panic!("cargo built no rlib of `{crate_name}`"));
        flags.push("--extern".to_string());
        flags.push(format!("{crate_name}={}", rlib.display()));
    }
    flags
}
