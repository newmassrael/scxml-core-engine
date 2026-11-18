// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The one way a suite starts `kotlinc`.
//!
//! A generated Kotlin file is compiled against the runtimes Gradle builds, and
//! Gradle builds them for the JVM their `jvmToolchain(N)` names. kotlinc left
//! to its own default writes JVM 1.8 bytecode, and refuses to inline a
//! runtime function built for a later target into it ("cannot inline bytecode
//! built with JVM target 17 into bytecode that is being built with JVM target
//! 1.8"): every generated decode that calls the forge runtime's inline
//! `valueOr` was refused so, 48 times in one run. A suite that compiled
//! without the jar was not refused, and was compiling for a JVM nothing ships.
//!
//! So every suite compiles at the target the runtimes' builds declare, read
//! from those builds rather than from a second literal here that a raise of the
//! toolchain would leave behind.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;

/// The Gradle builds of the runtimes a generated file is compiled against,
/// relative to the repository root: the forge runtime a codec calls and the
/// statechart runtime a machine runs on.
const RUNTIME_BUILDS: [&str; 2] = [
    "backends/kotlin/forge-runtime/build.gradle.kts",
    "backends/kotlin/runtime/build.gradle.kts",
];

/// The JVM target the Kotlin runtimes are built for: the `N` of the
/// `jvmToolchain(N)` each runtime's Gradle build declares. Two builds that
/// declare different targets have no one target a file compiled against both
/// can take, so that is a failure here and not a choice.
pub fn jvm_target() -> String {
    let root = repository_root();
    let targets: Vec<(&str, String)> = RUNTIME_BUILDS
        .iter()
        .map(|build| {
            let text = std::fs::read_to_string(root.join(build))
                .unwrap_or_else(|e| panic!("read {build}: {e}"));
            let target = declared_toolchain(&text)
                .unwrap_or_else(|| panic!("{build} declares no `jvmToolchain(N)`"));
            (*build, target)
        })
        .collect();
    let first = targets[0].1.clone();
    assert!(
        targets.iter().all(|(_, target)| *target == first),
        "the Kotlin runtimes are built for different JVM targets: {targets:?}"
    );
    first
}

/// The repository root: `sce-build`'s parent.
fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build's parent is the repository root")
        .to_path_buf()
}

/// The common sources of the Kotlin forge runtime, relative to the repository
/// root: `com.sce.forge.runtime` as every platform has it.
const FORGE_RUNTIME_COMMON: &str = "backends/kotlin/forge-runtime/src/commonMain";

/// Every source of the Kotlin forge runtime's common code, sorted: the whole of
/// what a generated file can name of `com.sce.forge.runtime`. It imports
/// nothing but the standard library, so kotlinc compiles it beside the
/// generated files with nothing else. A suite compiles all of it rather than
/// the files it knows a generated file names today: a list falls behind the
/// first time the printer names another part of the runtime, as one that held
/// `Codec.kt` alone did when a bounded text began to measure itself by `Utf8`.
pub fn forge_runtime_sources() -> Vec<PathBuf> {
    let mut found = Vec::new();
    kotlin_files(&repository_root().join(FORGE_RUNTIME_COMMON), &mut found);
    found.sort();
    found
}

/// Every `.kt` file under `dir`, into `found`.
fn kotlin_files(dir: &Path, found: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap_or_else(|e| panic!("read {}: {e}", dir.display())) {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            kotlin_files(&path, found);
        } else if path.extension().is_some_and(|e| e == "kt") {
            found.push(path);
        }
    }
}

/// The `N` of the first `jvmToolchain(N)` in the Gradle build `text`, where
/// `N` is a number.
fn declared_toolchain(text: &str) -> Option<String> {
    const CALL: &str = "jvmToolchain(";
    let after = &text[text.find(CALL)? + CALL.len()..];
    let target = after[..after.find(')')?].trim();
    if !target.is_empty() && target.chars().all(|c| c.is_ascii_digit()) {
        Some(target.to_string())
    } else {
        None
    }
}

/// `kotlinc` at `path`, compiling at the runtimes' JVM target
/// ([`jvm_target`]); the caller adds the rest of its arguments.
pub fn kotlinc(path: impl AsRef<Path>) -> Command {
    let mut command = Command::new(path.as_ref());
    command.args(jvm_target_args());
    command
}

/// The arguments that set the JVM target, for a suite that hands its
/// arguments to a runner of its own rather than building the command here.
pub fn jvm_target_args() -> [String; 2] {
    ["-jvm-target".to_string(), jvm_target()]
}
